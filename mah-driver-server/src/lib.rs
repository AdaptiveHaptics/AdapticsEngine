use std::thread;
use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_type, Inventory, InventoryBuilder, function};
use pattern_evaluator::BrushAtAnimLocalTime;


pub mod threads;
use threads::pattern::pattern_eval;
use pattern_eval::PatternEvalUpdate;
use threads::streaming;
use threads::net::websocket;


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


#[ffi_type(opaque)]
#[repr(C)]
pub struct AdapticsEngineHandleFFI {
    aeh: AdapticsEngineHandle,
}


#[ffi_function]
#[no_mangle]
pub extern "C" fn init_adaptics_engine(use_mock_streaming: bool) -> *mut AdapticsEngineHandleFFI {
    Box::into_raw(Box::new(AdapticsEngineHandleFFI {
        aeh: create_threads(use_mock_streaming, true)
    }))
}

/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandleFFI` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn deinit_adaptics_engine(handle: *mut AdapticsEngineHandleFFI) {
    if handle.is_null() { return }
    let handle = unsafe { Box::from_raw(handle) };
    handle.aeh.its_over_tx.send(()).ok(); // ignore send error (if thread already exited)
    handle.aeh.pattern_eval_handle.join().unwrap();
    handle.aeh.ulh_streaming_handle.join().unwrap().ok(); // unwrap panics, return errors
    println!("deinit_adaptics_engine done");
}

/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_pattern(handle: *mut AdapticsEngineHandleFFI, pattern_json: AsciiPointer) {
    if handle.is_null() { return }
    let handle = unsafe { &mut *handle };
    handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Pattern { pattern_json: pattern_json.as_str().unwrap().to_owned() }).unwrap();
}
/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_playstart(handle: *mut AdapticsEngineHandleFFI, playstart: f64, playstart_offset: f64) {
    if handle.is_null() { return }
    let handle = unsafe { &mut *handle };
    handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Playstart { playstart, playstart_offset }).unwrap();
}
/// # Safety
/// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
#[ffi_function]
#[no_mangle]
pub unsafe extern "C" fn adaptics_engine_update_parameters(handle: *mut AdapticsEngineHandleFFI, evaluator_params: AsciiPointer) {
    if handle.is_null() { return }
    let handle = unsafe { &mut *handle };
    handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params: serde_json::from_slice(evaluator_params.as_c_str().unwrap().to_bytes()).unwrap() }).unwrap();
}


// Guard function used by backends.
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
        .register(function!(ffi_api_guard))
        .inventory()
}