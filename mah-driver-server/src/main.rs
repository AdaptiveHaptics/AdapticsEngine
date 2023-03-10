use std::ops::{Sub, Add};
use std::pin::Pin;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use cxx::CxxVector;
use pattern_evaluator::BrushAtAnimLocalTime;
use crossbeam_channel;

mod websocket;
use websocket::PEWSServerMessage;
mod pattern_eval_thread;
use pattern_eval_thread::{PatternEvalUpdate, PatternEvalCall};
// use thread_priority;

const CALLBACK_RATE: f64 = 500.0;
const SECONDS_PER_NETWORK_SEND: f64 = 1.0 / 30.0;
const DEVICE_UPDATE_RATE: u64 = 20000; //20khz


#[cxx::bridge]
mod ffi {
    #[derive(Debug)]
    struct EvalCoords {
        x: f64,
        y: f64,
        z: f64,
    }
    #[derive(Debug)]
    struct EvalResult {
        coords: EvalCoords,
        intensity: f64,
    }

    unsafe extern "C++" {
        include!("ulh3-streaming.h");

        type ULHStreamingController;

        fn pause_emitter(self: Pin<&mut ULHStreamingController>) -> Result<()>;
        fn resume_emitter(self: Pin<&mut ULHStreamingController>) -> Result<()>;
        fn getMissedCallbackIterations(&self) -> Result<usize>;
        fn new_ulh_streaming_controller(callback_rate: f32, cb_func: fn(&CxxVector<f64>, Pin<&mut CxxVector<EvalResult>>)) -> Result<UniquePtr<ULHStreamingController>>;

        fn get_current_chrono_time() -> f64;
    }
}
pub use ffi::*;
// pub use ffi::EvalCoords;
// pub use ffi::EvalResult;

impl From<BrushAtAnimLocalTime> for EvalResult {
    fn from(be: BrushAtAnimLocalTime) -> EvalResult {
        EvalResult {
            coords: EvalCoords {
                x: be.coords.x / 1000.0,
                y: be.coords.y / 1000.0,
                // z: be.coords.z / 1000.0,
                z: 0.1,
            },
            intensity: be.intensity,
        }
    }
}

type MilSec = f64;

fn js_milliseconds_to_duration(ms: f64) -> Duration {
    if ms.is_sign_negative() { panic!("js_milliseconds_to_duration: ms is negative"); }
    Duration::from_nanos((ms * 1e6) as u64)
}
fn instant_add_js_milliseconds(instant: Instant, ms: f64) -> Instant {
    if ms.is_sign_negative() {
        instant.sub(js_milliseconds_to_duration(-ms))
    } else {
        instant.add(js_milliseconds_to_duration(ms))
    }
}


use clap::Parser;

/// Renders patterns from the web based mid air haptics designer tool, over a WebSocket connection
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct MAHServerArgs {
    #[clap(short, long, default_value = "127.0.0.1:8080")]
    websocket_bind_addr: String,

    #[clap(short='m', long)]
    use_mock_streaming: bool,

    #[clap(short, long)]
    no_network: bool,
}


fn main() {
    let cli_args = MAHServerArgs::parse();

    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    let (network_send_tx, network_send_rx) = crossbeam_channel::bounded(1);


    // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let res = pattern_eval_thread::pattern_eval_loop(
                patteval_call_rx,
                patteval_update_rx,
                patteval_return_tx,
                network_send_tx,
            );

            res.unwrap();

            println!("pattern-eval thread exiting...");
        })
        .unwrap();

    let ulh_streaming_handle_opt = if !cli_args.use_mock_streaming {
        Some(thread::Builder::new()
            .name("ulh-streaming".to_string())
            .spawn(move || {
                println!("ulhaptics streaming thread starting...");

                static STATIC_ECALLBACK_MUTEX: Mutex<Option<Box<dyn Fn(&CxxVector<MilSec>, Pin<&mut CxxVector<EvalResult>>) + Send>>> = Mutex::new(None);

                // sync epochs are used to convert from chrono time to Instant
                // they both appear to use the same monotonic clock source and unix epoch, but i'd like to be agnostic of that assumption
                // I am assuming that these be called at the nearly the same time, in either order
                let sync_epoch_instant = Instant::now();
                let sync_epoch_chrono_ms = get_current_chrono_time();

                fn static_streaming_emission_callback(time_arr_ms: &CxxVector<MilSec>, eval_results_arr: Pin<&mut CxxVector<EvalResult>>) {
                    if let Some(f) = STATIC_ECALLBACK_MUTEX.lock().unwrap().as_ref() {
                        f(time_arr_ms, eval_results_arr);
                    }
                }
                let streaming_emission_callback = move |time_arr_ms: &CxxVector<MilSec>, eval_results_arr: Pin<&mut CxxVector<EvalResult>> | {
                    patteval_call_tx.send(PatternEvalCall::EvalBatch{
                        time_arr_instants: time_arr_ms.iter().map(|ms| sync_epoch_instant.add(js_milliseconds_to_duration(ms-sync_epoch_chrono_ms))).collect() // convert from chrono time to Instant using epoch
                    }).unwrap();
                    let eval_arr = patteval_return_rx.recv().unwrap();
                    let eval_results_arr = eval_results_arr.as_mut_slice();
                    for (i,eval) in eval_arr.into_iter().enumerate() {
                        eval_results_arr[i] = eval.into();
                    }
                };
                STATIC_ECALLBACK_MUTEX.lock().unwrap().replace(Box::new(streaming_emission_callback));

                match new_ulh_streaming_controller(CALLBACK_RATE as f32, static_streaming_emission_callback) {
                    Ok(mut ulh_streaming_controller) => {
                        ulh_streaming_controller.pin_mut().resume_emitter().unwrap();
                        println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations().unwrap());
                    },
                    Err(e) => {
                        println!("error creating ulhaptics streaming controller: {}", e);
                        let cb = STATIC_ECALLBACK_MUTEX.lock().unwrap().take();
                        drop(cb);
                    }
                }
            })
            .unwrap())
    } else {
        println!("using mock streaming");
        Some(thread::Builder::new()
            .name("mock-streaming".to_string())
            .spawn(move || {
                println!("mock streaming thread starting...");


                // println!("setting thread priority max");
                // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();


                let device_tick_dur = Duration::from_nanos(1_000_000_000/DEVICE_UPDATE_RATE);
                let ecallback_tick_dur = Duration::from_secs_f64(1.0/CALLBACK_RATE);
                let deadline_offset = ecallback_tick_dur * 1;
                let mut last_tick = Instant::now();

                assert!(device_tick_dur.as_secs_f64() > 0.0, "device_tick_dur must be > 0");
                loop {
                    while last_tick + ecallback_tick_dur > Instant::now() {} //busy wait
                    let curr_time = Instant::now();
                    let elapsed = curr_time - last_tick;
                    if elapsed > ecallback_tick_dur + Duration::from_micros(100) { println!("[WARN] elapsed > ecallback_tick_dur: {:?} > {:?}", elapsed, ecallback_tick_dur); }
                    last_tick = curr_time;

                    let deadline_time = curr_time + deadline_offset;

                    let mut time_arr_instants = Vec::with_capacity((DEVICE_UPDATE_RATE as f64 / CALLBACK_RATE) as usize + 2);
                    let mut future_device_tick_instant = deadline_time;
                    while future_device_tick_instant < (deadline_time + ecallback_tick_dur) {
                        time_arr_instants.push(future_device_tick_instant);
                        future_device_tick_instant += device_tick_dur;
                    }

                    patteval_call_tx.send(PatternEvalCall::EvalBatch{ time_arr_instants }).unwrap();
                    patteval_return_rx.recv().unwrap();

                    // if let Err(e) = patteval_return_rx.recv() {
                    //     // pattern eval thread exited, so we should exit
                    //     break;
                    // }
                    // println!("remaining time {:?}", deadline_time-Instant::now());

                    // both are needed because durations are always positive and subtraction saturates
                    let deadline_remaining = deadline_time - Instant::now();
                    let deadline_missed_by = Instant::now() - deadline_time;
                    if deadline_remaining.is_zero() {
                        eprintln!("missed deadline by {:?}", deadline_missed_by);
                    }
                }

                //println!("mock streaming thread exiting...");
            })
            .unwrap())
     };


    let net_handle_opt = if !cli_args.no_network {
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                websocket::start_ws_server(&cli_args.websocket_bind_addr, patteval_update_tx, network_send_rx);
                println!("net thread thread exiting...");
            })
            .unwrap();
        Some(thread)
    } else { None };


    pattern_eval_handle.join().unwrap();
    ulh_streaming_handle_opt.map(|h| h.join().unwrap());
    net_handle_opt.map(|h| h.join().unwrap());
}
