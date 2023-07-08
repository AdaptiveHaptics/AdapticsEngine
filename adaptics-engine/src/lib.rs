use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, self};
use std::sync::RwLock;
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
use threads::tracking;

const CALLBACK_RATE: f64 = 500.0;
const SECONDS_PER_PLAYBACK_UPDATE: f64 = 1.0 / 30.0;
const DEVICE_UPDATE_RATE: u64 = 20000; //20khz
const SEND_UNTRACKED_PLAYBACK_UPDATES: bool = false;


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
    end_streaming_tx: crossbeam_channel::Sender<()>,
    pattern_eval_handle: thread::JoinHandle<()>,
    patteval_update_tx: crossbeam_channel::Sender<pattern_eval::PatternEvalUpdate>,
    ulh_streaming_handle: thread::JoinHandle<Result<(), Box<dyn std::error::Error + std::marker::Send>>>,
    playback_updates_rx: Option<crossbeam_channel::Receiver<websocket::PEWSServerMessage>>,
}

fn create_threads(
    use_mock_streaming: bool,
    disable_playback_updates: bool,
    tracking_data_rx: Option<crossbeam_channel::Receiver<tracking::TrackingFrame>>,
) -> AdapticsEngineHandle {
    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    let (playback_updates_tx, playback_updates_rx) = if !disable_playback_updates { let (t,r) = crossbeam_channel::bounded(1); (Some(t), Some(r)) } else { (None, None) };

    let (end_streaming_tx, end_streaming_rx) = crossbeam_channel::bounded(1);

    // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let res = pattern_eval::pattern_eval_loop(
                SECONDS_PER_PLAYBACK_UPDATE,
                SEND_UNTRACKED_PLAYBACK_UPDATES,
                patteval_call_rx,
                patteval_update_rx,
                patteval_return_tx,
                playback_updates_tx,
                tracking_data_rx,
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
                    end_streaming_rx,
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
                    end_streaming_rx,
                );

                // println!("mock streaming thread exiting...");
                Ok(())
            })
            .unwrap()
    };

    AdapticsEngineHandle {
        end_streaming_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        playback_updates_rx,
    }
}


pub fn run_threads_and_wait(
    use_mock_streaming: bool,
    websocket_bind_addr: Option<String>,
    enable_tracking: bool,
) -> Result<(), Box<dyn std::error::Error + Send>> {

    let (tracking_data_tx, tracking_data_rx) = if enable_tracking { let (s, r) = crossbeam_channel::unbounded(); (Some(s), Some(r)) } else { (None, None) };

    let AdapticsEngineHandle {
        end_streaming_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        playback_updates_rx,
    } = create_threads(use_mock_streaming, websocket_bind_addr.is_none(), tracking_data_rx);

    let net_handle_opt = if let Some(websocket_bind_addr) = websocket_bind_addr {
        let playback_updates_rx = playback_updates_rx.unwrap();
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                websocket::start_ws_server(&websocket_bind_addr, patteval_update_tx, playback_updates_rx);
                println!("net thread thread exiting...");
            })
            .unwrap();
        Some(thread)
    } else { None };


    let lmc_tracking_handle = if let Some(tracking_data_tx) = tracking_data_tx {
        let thread = thread::Builder::new()
            .name("lmc-tracking".to_string())
            .spawn(move || {
                println!("tracking thread starting...");
                tracking::leapmotion::start_tracking_loop(tracking_data_tx);
                println!("tracking thread exiting...");
            })
            .unwrap();
        Some(thread)
    } else { None };


    // wait for threads to exit

    pattern_eval_handle.join().unwrap();

    end_streaming_tx.send(()).ok(); // ignore send error (if thread already exited)
    ulh_streaming_handle.join().unwrap()?; // unwrap panics, return errors

    if let Some(lmc_tracking_handle) = lmc_tracking_handle {
        tracking::leapmotion::TRACKING_IS_DONE.store(true, atomic::Ordering::Relaxed);
        lmc_tracking_handle.join().unwrap();
    }

    println!("waiting for net thread...");
    if let Some(h) = net_handle_opt { h.join().unwrap() }

    Ok(())
}


#[ffi_type(patterns(ffi_error))]
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum FFIError {
    Ok = 0,
    NullPassed = 1,
    Panic = 2,
    OtherError = 3,
    AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo = 4,
    ErrMsgProvided = 5,
    EnablePlaybackUpdatesWasFalse = 6,
    //NoPlaybackUpdatesAvailable = 7,
    ParameterJSONDeserializationFailed = 8,
    HandleIDNotFound = 9,
}
// Gives special meaning to some of your error variants.
impl interoptopus::patterns::result::FFIError for FFIError {
    const SUCCESS: Self = Self::Ok;
    const NULL: Self = Self::NullPassed;
    const PANIC: Self = Self::Panic;
}
impl FFIError {
    /// not actually exposed to FFI yet, just enforcing I write error messages for new errors
    pub fn get_msg(&self) -> &'static str {
        match self {
            FFIError::Ok => "ok",
            FFIError::NullPassed => "A null pointer was passed where an actual element (likely AdapticsEngineHandleFFI) was needed.",
            FFIError::Panic => "A panic occurred. Further error information could not be marshalled.",
            FFIError::OtherError => "An error occurred. Further error information could not be marshalled.",
            FFIError::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo => "The AdapticsEngine thread disconnected. Check deinit_adaptics_engine for more information on what caused the disconnect.",
            FFIError::ErrMsgProvided => "An error occurred. Check err_msg parameter for more information.",
            FFIError::EnablePlaybackUpdatesWasFalse => "enable_playback_updates was false. Call init_adaptics_engine with enable_playback_updates set to true to enable playback updates.",
            // FFIError::NoPlaybackUpdatesAvailable => "No playback updates available. Playback updates are available at ~30hz while a pattern is playing.",
            FFIError::ParameterJSONDeserializationFailed => "Parameter JSON deserialization failed.",
            FFIError::HandleIDNotFound => "Handle ID not found.",
        }
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
impl AdapticsEngineHandleFFI {
    fn new(aeh: AdapticsEngineHandle) -> Self {
        Self { aeh, last_error_msg: None, }
    }
}


type HandleID = u64;
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(0);
static ENGINE_HANDLE_MAP: RwLock<Option<HashMap<HandleID, AdapticsEngineHandleFFI>>> = RwLock::new(None);

/// use_mock_streaming: if true, use mock streaming. if false, use ulhaptics streaming
/// enable_playback_updates: if true, enable playback updates, adaptics_engine_get_playback_updates expected to be called at 30hz.
#[ffi_function]
#[no_mangle]
pub extern "C" fn init_adaptics_engine(use_mock_streaming: bool, enable_playback_updates: bool) -> HandleID {
    let aeh = create_threads(use_mock_streaming, !enable_playback_updates, None);
    let ffi_handle = AdapticsEngineHandleFFI::new(aeh);
    // get map or create new map
    let mut map = ENGINE_HANDLE_MAP.write().unwrap();
    let map = map.get_or_insert_with(HashMap::new);
    let handle_id = NEXT_HANDLE_ID.fetch_add(1, atomic::Ordering::Relaxed);
    map.insert(handle_id, ffi_handle);
    handle_id
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn deinit_adaptics_engine(handle_id: HandleID, mut err_msg: FFISliceMut<u8>) -> FFIError {
    let Some(handle) = ENGINE_HANDLE_MAP.write().unwrap().as_mut().and_then(|map| map.remove(&handle_id)) else { return FFIError::HandleIDNotFound; };
    handle.aeh.end_streaming_tx.send(()).ok(); // ignore send error (if thread already exited)
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
macro_rules! get_handle_from_id {
    ($handle:ident <- $handle_id:expr) => {
        let rguard = ENGINE_HANDLE_MAP.read().unwrap();
        let Some($handle) = rguard.as_ref().and_then(|map| map.get(&$handle_id)) else { return FFIError::HandleIDNotFound; };
    };
}
macro_rules! deserialize_json_parameter {
    ($asciiptr:ident) => {
        if let Some(cstr) = $asciiptr.as_c_str() {
            if let Ok(value) = serde_json::from_slice(cstr.to_bytes()) { value }
            else { return FFIError::ParameterJSONDeserializationFailed; }
        } else { return FFIError::ParameterJSONDeserializationFailed; }
    };
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_pattern(handle_id: HandleID, pattern_json: AsciiPointer) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Pattern { pattern_json: pattern_json.as_str().unwrap().to_owned() }).into();
    ffi_error
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_playstart(handle_id: HandleID, playstart: f64, playstart_offset: f64) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Playstart { playstart, playstart_offset }).into();
    ffi_error
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_parameters(handle_id: HandleID, evaluator_params: AsciiPointer) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let evaluator_params = deserialize_json_parameter!(evaluator_params);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params }).into();
    ffi_error
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_reset_parameters(handle_id: HandleID) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params: pattern_evaluator::PatternEvaluatorParameters::default() }).into();
    ffi_error
}

/// Will be overwritten by playstart time computation.
/// However, the time parameter is needed to correctly start in the middle of a pattern. (next_eval_params.last_eval_pattern_time will be set to this when a new playstart is received)
/// This will need to be called before playstart
#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_time(handle_id: HandleID, time: f64) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::ParameterTime { time }).into();
    ffi_error
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_user_parameters(handle_id: HandleID, user_parameters: AsciiPointer) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let user_parameters = deserialize_json_parameter!(user_parameters);
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::UserParameters { user_parameters }).into();
    ffi_error
}

#[ffi_type]
#[repr(C)]
pub struct GeoMatrix {
    pub data: [f64; 16],
}
/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub extern "C" fn adaptics_engine_update_geo_transform_matrix(handle_id: HandleID, geo_matrix: GeoMatrix) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let transform = {
        let g = geo_matrix.data;
        pattern_evaluator::GeometricTransformMatrix([
            [g[0], g[1], g[2], g[3]],
            [g[4], g[5], g[6], g[7]],
            [g[8], g[9], g[10], g[11]],
            [g[12], g[13], g[14], g[15]],
        ])
    };
    let ffi_error: FFIError = handle.aeh.patteval_update_tx.send(PatternEvalUpdate::GeoTransformMatrix { transform }).into();
    ffi_error
}



/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[repr(C)]
#[derive(Debug)]
pub struct UnityEvalCoords {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[repr(C)]
#[derive(Debug)]
pub struct UnityEvalResult {
    /// !NOTE: y and z are swapped for Unity
    pub coords: UnityEvalCoords,
    pub intensity: f64,
    pub pattern_time: f64,
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
            pattern_time: be.pattern_time,
        }
    }
}

/// Populate `eval_results` with the latest evaluation results.
/// `num_evals` will be set to the number of evaluations written to `eval_results`, or 0 if there are no new evaluations since the last call to this function.
///
/// # Safety
/// `num_evals` must be a valid pointer to a u32
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_get_playback_updates(handle_id: HandleID, eval_results: &mut FFISliceMut<UnityEvalResult>, num_evals: *mut u32) -> FFIError {
    get_handle_from_id!(handle <- handle_id);
    let num_evals = deref_check_null!(num_evals);
    let ffi_error: FFIError = match &handle.aeh.playback_updates_rx {
        Some(playback_updates_rx) => {
            match playback_updates_rx.try_recv() {
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
        .register(function!(adaptics_engine_reset_parameters))
        .register(function!(adaptics_engine_update_time))
        .register(function!(adaptics_engine_update_user_parameters))
        .register(function!(adaptics_engine_update_geo_transform_matrix))
        .register(function!(adaptics_engine_get_playback_updates))
        .register(function!(ffi_api_guard))
        .inventory()
}


#[cfg(test)]
mod test {
    use std::ffi::CString;

    use crate::*;

    #[test]
    fn test_update_user_params() {
        let handle = init_adaptics_engine(true, false);
        let cstr = CString::new("{\"dist\": 74.446439743042}").unwrap();
        let ap = AsciiPointer::from_cstr(&cstr);
        let rv = adaptics_engine_update_user_parameters(handle, ap);
        assert_eq!(rv, FFIError::Ok);
    }
}