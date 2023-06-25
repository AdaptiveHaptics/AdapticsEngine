use std::thread;
use interoptopus::patterns::slice::FFISliceMut;
use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_type, Inventory, InventoryBuilder, function};
use pattern_evaluator::{BrushAtAnimLocalTime, PatternEvaluator};


pub mod threads;
use threads::pattern::pattern_eval;
use pattern_eval::PatternEvalUpdate;
use threads::streaming;
use threads::net::websocket::{self, PEWSServerMessage};


const CALLBACK_RATE: f64 = 500.0;
const SECONDS_PER_NETWORK_SEND: f64 = 1.0 / 30.0;
const DEVICE_UPDATE_RATE: u64 = 20000; //20khz


#[derive(Debug)]
pub(crate) struct TLError {
    details: String
}
impl TLError {
    pub(crate) fn new(msg: &str) -> TLError {
        TLError{ details: msg.to_string() }
    }
}
impl std::fmt::Display for TLError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}",self.details)
    }
}
impl std::error::Error for TLError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub struct AdapticsEngineHandle {
    its_over_tx: crossbeam_channel::Sender<()>,
    pattern_eval_handle: thread::JoinHandle<()>,
    patteval_update_tx: crossbeam_channel::Sender<pattern_eval::PatternEvalUpdate>,
    ulh_streaming_handle: thread::JoinHandle<Result<(), Box<dyn std::error::Error + std::marker::Send>>>,
    network_send_rx: Option<crossbeam_channel::Receiver<websocket::PEWSServerMessage>>,
}

fn create_threads(
    use_mock_streaming: bool,
    no_network_playback_updates: bool,
) -> AdapticsEngineHandle {
    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    let (network_send_tx, network_send_rx) = if !no_network_playback_updates { let (t,r) = crossbeam_channel::bounded(1); (Some(t), Some(r)) } else { (None, None) };

    let (its_over_tx, its_over_rx) = crossbeam_channel::bounded(1);

    // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let res = pattern_eval::pattern_eval_loop(
                SECONDS_PER_NETWORK_SEND,
                patteval_call_rx,
                patteval_update_rx,
                patteval_return_tx,
                network_send_tx,
            );

            // res.unwrap();
            res.ok(); // ignore error, only occurs when channel disconnected

            println!("pattern-eval thread exiting...");
        })
        .unwrap();

    let ulh_streaming_handle = if !use_mock_streaming {
        thread::Builder::new()
            .name("ulh-streaming".to_string())
            .spawn(move || -> Result<(), Box<dyn std::error::Error + Send>> {
                println!("ulhaptics streaming thread starting...");

                streaming::ulhaptics::start_streaming_emitter(
                    CALLBACK_RATE as f32,
                    patteval_call_tx,
                    patteval_return_rx,
                    its_over_rx,
                )
            }).unwrap()
    } else {
        println!("using mock streaming");
        thread::Builder::new()
            .name("mock-streaming".to_string())
            .spawn(move || {
                println!("mock streaming thread starting...");

                streaming::mock::start_mock_emitter(
                    DEVICE_UPDATE_RATE,
                    CALLBACK_RATE,
                    patteval_call_tx,
                    patteval_return_rx,
                    its_over_rx,
                );

                // println!("mock streaming thread exiting...");
                Ok(())
            })
            .unwrap()
    };

    AdapticsEngineHandle {
        its_over_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        network_send_rx,
    }
}


pub fn run_threads_and_wait(use_mock_streaming: bool, websocket_bind_addr: Option<String>) -> Result<(), Box<dyn std::error::Error + Send>> {

    let AdapticsEngineHandle {
        its_over_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        network_send_rx,
    } = create_threads(use_mock_streaming, websocket_bind_addr.is_none());

    let net_handle_opt = if let (Some(websocket_bind_addr), Some(network_send_rx)) = (websocket_bind_addr, network_send_rx) {
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                websocket::start_ws_server(&websocket_bind_addr, patteval_update_tx, network_send_rx);
                println!("net thread thread exiting...");
            })
            .unwrap();
        Some(thread)
    } else { None };



    pattern_eval_handle.join().unwrap();
    its_over_tx.send(()).ok(); // ignore send error (if thread already exited)
    ulh_streaming_handle.join().unwrap()?; // unwrap panics, return errors
    println!("waiting for net thread...");
    if let Some(h) = net_handle_opt { h.join().unwrap() }

    Ok(())
}


#[ffi_type(patterns(ffi_error))]
#[repr(C)]
pub enum FFIError {
    Ok = 0,
    NullPassed = 1,
    Panic = 2,
    OtherError = 3,
    AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo = 4,
    ErrMsgProvided = 5,
    EnablePlaybackUpdatesWasFalse = 6,
    //NoPlaybackUpdatesAvailable = 7,
}
// Gives special meaning to some of your error variants.
impl interoptopus::patterns::result::FFIError for FFIError {
    const SUCCESS: Self = Self::Ok;
    const NULL: Self = Self::NullPassed;
    const PANIC: Self = Self::Panic;
}
impl FFIError {
    fn update_last_error_msg(&self, handle: &mut AdapticsEngineHandleFFI) {
        handle.last_error_msg = Some(match self {
            FFIError::Ok => "ok",
            FFIError::NullPassed => "A null pointer was passed where an actual element (likely AdapticsEngineHandleFFI) was needed",
            FFIError::Panic => "A panic occurred. Further error information could not be marshalled",
            FFIError::OtherError => "An error occurred. Further error information could not be marshalled",
            FFIError::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo => "The AdapticsEngine thread disconnected. Check deinit_adaptics_engine for more information on what caused the disconnect",
            FFIError::ErrMsgProvided => "An error occurred. Check independent err_msg for more information",
            FFIError::EnablePlaybackUpdatesWasFalse => "enable_playback_updates was false. Call init_adaptics_engine with enable_playback_updates set to true to enable playback updates",
            // FFIError::NoPlaybackUpdatesAvailable => "No playback updates available. Playback updates are available at ~30hz while a pattern is playing.",
        }.to_string())
    }
}
impl<T> From<Result<(), crossbeam_channel::SendError<T>>> for FFIError {
    fn from(value: Result<(), crossbeam_channel::SendError<T>>) -> Self {
        match value {
            Ok(_) => Self::Ok,
            Err(_) => Self::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo,
        }
    }
}

#[ffi_type(opaque)]
#[repr(C)]
pub struct AdapticsEngineHandleFFI {
    last_error_msg: Option<String>,
    aeh: AdapticsEngineHandle,
}


/// use_mock_streaming: if true, use mock streaming. if false, use ulhaptics streaming
/// enable_playback_updates: if true, enable playback updates, adaptics_engine_get_playback_updates expected to be called at 30hz.
#[ffi_function]
#[no_mangle]
pub extern "C" fn init_adaptics_engine(use_mock_streaming: bool, enable_playback_updates: bool) -> *mut AdapticsEngineHandleFFI {
    let aeh = create_threads(use_mock_streaming, !enable_playback_updates);
    Box::into_raw(Box::new(AdapticsEngineHandleFFI {
        aeh,
        last_error_msg: None,
    }))
}

/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandleFFI` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn deinit_adaptics_engine(handle: *mut AdapticsEngineHandleFFI, mut err_msg: FFISliceMut<u8>) -> FFIError {
    if handle.is_null() { return FFIError::NullPassed; }
    let handle = unsafe { Box::from_raw(handle) };
    handle.aeh.its_over_tx.send(()).ok(); // ignore send error (if thread already exited)
    if handle.aeh.pattern_eval_handle.join().is_err() { return FFIError::Panic; }
    match handle.aeh.ulh_streaming_handle.join() {
        Ok(Ok(())) => FFIError::Ok,
        Ok(Err(res_err)) => {
            let err_msg_rv_slice = err_msg.as_slice_mut();
            let res_err_str_bytes = res_err.to_string().into_bytes();
            // copy as many bytes of res_err_str_bytes as possible into err_msg_rv_slice
            let bytes_to_copy = std::cmp::min(err_msg_rv_slice.len(), res_err_str_bytes.len());
            err_msg_rv_slice[..bytes_to_copy].copy_from_slice(&res_err_str_bytes[..bytes_to_copy]);
            FFIError::ErrMsgProvided
        },
        Err(_) => FFIError::Panic,
    }
}

macro_rules! deref_check_null {
    ($handle:expr) => {{
        if $handle.is_null() { return FFIError::NullPassed; }
        unsafe { &mut *$handle }
    }};
}

/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_pattern(handle: *mut AdapticsEngineHandleFFI, pattern_json: AsciiPointer) -> FFIError {
    let handle = deref_check_null!(handle);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Pattern { pattern_json: pattern_json.as_str().unwrap().to_owned() }).into();
    ffi_error.update_last_error_msg(handle);
    ffi_error
}
/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_playstart(handle: *mut AdapticsEngineHandleFFI, playstart: f64, playstart_offset: f64) -> FFIError {
    let handle = deref_check_null!(handle);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Playstart { playstart, playstart_offset }).into();
    ffi_error.update_last_error_msg(handle);
    ffi_error
}
/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_parameters(handle: *mut AdapticsEngineHandleFFI, evaluator_params: AsciiPointer) -> FFIError {
    let handle = deref_check_null!(handle);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params: serde_json::from_slice::<pattern_evaluator::PatternEvaluatorParameters>(evaluator_params.as_c_str().unwrap().to_bytes()).unwrap() }).into();
    ffi_error.update_last_error_msg(handle);
    ffi_error
}


/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[derive(Debug)]
pub struct UnityEvalCoords {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[derive(Debug)]
pub struct UnityEvalResult {
    /// !NOTE: y and z are swapped for Unity
    pub coords: UnityEvalCoords,
    pub intensity: f64,
}
impl From<BrushAtAnimLocalTime> for UnityEvalResult {
    fn from(be: BrushAtAnimLocalTime) -> UnityEvalResult {
        UnityEvalResult {
            // !NOTE: y and z are swapped for Unity
            coords: UnityEvalCoords {
                x: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.x),
                z: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.y), // !NOTE: y and z are swapped for Unity
                y: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.z), // !NOTE: y and z are swapped for Unity
            },
            intensity: be.ul_control_point.intensity,
        }
    }
}

/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_get_playback_updates(handle: *mut AdapticsEngineHandleFFI, eval_results: &mut FFISliceMut<UnityEvalResult>, num_evals: *mut u32) -> FFIError {
    let handle = deref_check_null!(handle);
    let num_evals = deref_check_null!(num_evals);
    let ffi_error: FFIError = match &handle.aeh.network_send_rx {
        Some(network_send_rx) => {
            match network_send_rx.try_recv() {
                Ok(PEWSServerMessage::PlaybackUpdate { evals }) => {
                    // copy as many evals as possible into eval_results
                    let eval_results_slice = eval_results.as_slice_mut();
                    let evalresults_to_copy = std::cmp::min(eval_results_slice.len(), evals.len());
                    evals.into_iter().take(evalresults_to_copy).enumerate().for_each(|(i, be)| eval_results_slice[i] = be.into());
                    *num_evals = evalresults_to_copy as u32;
                    FFIError::Ok
                },
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    *num_evals = 0;
                    FFIError::Ok
                },
                Err(crossbeam_channel::TryRecvError::Disconnected) => FFIError::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo,
            }
        },
        None => FFIError::EnablePlaybackUpdatesWasFalse,
    };
    ffi_error.update_last_error_msg(handle);
    ffi_error
}




/// Guard function used by backends.
///
/// Change impl version in this comment to force bump the API version.
/// impl_version: 1
#[ffi_function]
#[no_mangle]
pub extern "C" fn ffi_api_guard() -> interoptopus::patterns::api_guard::APIVersion {
    ffi_inventory().into()
}

pub fn ffi_inventory() -> Inventory {
	InventoryBuilder::new()
        .register(function!(init_adaptics_engine))
        .register(function!(deinit_adaptics_engine))
        .register(function!(adaptics_engine_update_pattern))
        .register(function!(adaptics_engine_update_playstart))
        .register(function!(adaptics_engine_update_parameters))
        .register(function!(adaptics_engine_get_playback_updates))
        .register(function!(ffi_api_guard))
        .inventory()
}