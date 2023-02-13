use std::collections::HashMap;
use std::ops::{Sub, Add};
use std::pin::Pin;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use cxx::CxxVector;
use pattern_evaluator::{PatternEvaluator, PatternEvaluatorParameters, BrushAtAnimLocalTime};
use crossbeam_channel;

mod network;
use network::PEWSServerMessage;


const CALLBACK_RATE: f64 = 500.0;
const ENABLE_ULH_STREAMING: bool = false;
const ENABLE_NETWORKING: bool = true;


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
use ffi::*;
pub use ffi::EvalCoords;
pub use ffi::EvalResult;

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

pub enum PatternEvalCall {
    UpdatePattern{ pattern_json: String },
    UpdatePlaystart{ playstart: MilSec, playstart_offset: MilSec },
    UpdateParameters{ evaluator_params: PatternEvaluatorParameters },
    EvalBatch{ time_arr_instants: Vec<Instant>},
}

fn js_milliseconds_to_duration(ms: f64) -> Duration {
    Duration::from_nanos((ms * 1e6) as u64)
}

fn main() {
    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded(0);
    let (network_send_tx, network_send_rx) = crossbeam_channel::unbounded();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let default_pattern = pattern_evaluator::MidAirHapticsAnimationFileFormat {
                data_format: pattern_evaluator::MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
                revision: pattern_evaluator::DataFormatRevision::CurrentRevision,
                name: "DEFAULT_PATTERN".to_string(),
                keyframes: vec![],
                update_rate: 1000.0,
                projection: pattern_evaluator::Projection::Plane,
            };

            let mut pattern_eval = PatternEvaluator::new(default_pattern);
            let mut pattern_playstart = None;
            let mut parameters = PatternEvaluatorParameters { time: 0.0, user_parameters: HashMap::new() };

            loop {
                let call = patteval_call_rx.recv().unwrap();
                match call {
                    PatternEvalCall::UpdatePattern{ pattern_json } => {
                        pattern_eval = PatternEvaluator::new_from_json_string(&pattern_json);
                    },
                    PatternEvalCall::UpdateParameters{ evaluator_params } => {
                        parameters = evaluator_params;
                    },
                    PatternEvalCall::UpdatePlaystart{ playstart, playstart_offset } => {
                        if playstart == 0.0 {
                            pattern_playstart = None;
                        } else {
                            // get current time in milliseconds as f64
                            pattern_playstart = Some(Instant::now().add(js_milliseconds_to_duration(playstart_offset)));
                        }
                    },
                    PatternEvalCall::EvalBatch{ time_arr_instants } => {
                        let eval_arr: Vec<_> = time_arr_instants.iter().map(|time| {
                            let time = if let Some(playstart) = pattern_playstart { time.sub(playstart).as_nanos() as f64 / 1e6 } else { 0.0 };
                            parameters.time = time;
                            let eval = pattern_eval.eval_brush_at_anim_local_time(&parameters);
                            eval
                        }).collect();
                        patteval_return_tx.send(eval_arr).unwrap();
                        network_send_tx.send(PEWSServerMessage::PlaybackUpdate(eval_arr)).unwrap();
                    },
                }
            }
        })
        .unwrap();

    let ulh_streaming_handle_opt = if ENABLE_ULH_STREAMING {
        let patteval_call_tx = patteval_call_tx.clone();
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

                let mut ulh_streaming_controller = new_ulh_streaming_controller(CALLBACK_RATE as f32, static_streaming_emission_callback).unwrap();
                ulh_streaming_controller.pin_mut().resume_emitter().unwrap();
                ulh_streaming_controller.pin_mut().pause_emitter().unwrap();
                println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations().unwrap());
            })
            .unwrap())
    } else {
        println!("using mock streaming");
        let patteval_call_tx = patteval_call_tx.clone();
        Some(thread::Builder::new()
            .name("mock-streaming".to_string())
            .spawn(move || {
                println!("mock streaming thread starting...");

                let device_update_rate = 20000.0; //20khz
                // let start_time = Instant::now();
                let tick_dur = Duration::from_secs_f64(1.0/CALLBACK_RATE);
                let tick_rx = crossbeam_channel::tick(tick_dur);
                let deadline_offset = tick_dur * 1;

                loop {
                    tick_rx.recv().unwrap();
                    let curr_time = Instant::now();
                    let deadline_time = curr_time + deadline_offset;
                    let num_instants = (device_update_rate / CALLBACK_RATE) as u32;
                    let time_arr_instants = (0..num_instants).map(|i| deadline_time + tick_dur*i).collect();
                    patteval_call_tx.send(PatternEvalCall::EvalBatch{ time_arr_instants }).unwrap();
                    let _eval_arr = patteval_return_rx.recv().unwrap();
                    // println!("remaining time {:?}", deadline_time-Instant::now());
                    if Instant::now() > deadline_time {
                        eprintln!("missed deadline");
                    }
                }

            })
            .unwrap())
     };


    let net_handle_opt = if ENABLE_NETWORKING {
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                network::start_ws_server(patteval_call_tx, network_send_rx);
            })
            .unwrap();
        Some(thread)
    } else { None };


    pattern_eval_handle.join().unwrap();
    ulh_streaming_handle_opt.map(|h| h.join().unwrap());
    net_handle_opt.map(|h| h.join().unwrap());
}
