use std::collections::HashMap;
use std::thread;
use cxx::CxxVector;
use pattern_evaluator;
use crossbeam_channel;

mod network;


#[cxx::bridge]
mod ffi {
    #[derive(Debug)]
    struct EvalCoords {
        x: f64,
        y: f64,
        z: f64,
    }
    #[derive(Debug)]
    struct EvalResults {
        coords: EvalCoords,
        intensity: f64,
    }

    extern "Rust" {
        fn streaming_emission_callback(time_arr_ms: &CxxVector<f64>) -> Vec<EvalResults>;
    }

    unsafe extern "C++" {
        include!("ulh3-streaming.h");

        type ULHStreamingController;

        fn pause_emitter(self: Pin<&mut ULHStreamingController>) -> Result<()>;
        fn resume_emitter(self: Pin<&mut ULHStreamingController>) -> Result<()>;
        fn getMissedCallbackIterations(&self) -> Result<usize>;
        fn new_ulh_streaming_controller(callback_rate: f32) -> Result<UniquePtr<ULHStreamingController>>;
    }
}

// unsafe impl ExternType for ffi::ULHStreamingController {
//     type Id = type_id!("ffi::ULHStreamingController");
//     type Kind = cxx::kind::Opaque;
// }

use ffi::*;
pub use ffi::EvalCoords;
pub use ffi::EvalResults;
use pattern_evaluator::PatternEvaluator;
use pattern_evaluator::PatternEvaluatorParameters;

type MilSec = f64;

/// I am not sure about any threading/concurrency issues
pub fn streaming_emission_callback(time_arr_ms: &CxxVector<MilSec>) -> Vec<EvalResults> {
    todo!();
    let v = time_arr_ms.iter().map(|t| EvalResults{ coords: EvalCoords { x: 0.0, y: 0.0, z: 0.0 }, intensity: 0.0}).collect();
    return v;
}


enum PatternEvalCall {
    UpdatePattern{ mah_animation_json: String },
    UpdatePlaystart{ playstart: MilSec, playstart_offset: MilSec },
    //UpdateParameters(String), TODO
    EvalBatch{ time_arr_ms: Vec<MilSec>},
}

fn main() {
    println!("Hello, world!");

    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded(0);

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread started..");

            let default_pattern = pattern_evaluator::MidAirHapticsAnimationFileFormat {
                data_format: pattern_evaluator::MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
                revision: pattern_evaluator::DataFormatRevision::CurrentRevision,
                name: "DEFAULT_PATTERN".to_string(),
                keyframes: vec![],
                update_rate: 1000.0,
                projection: pattern_evaluator::Projection::Plane,
            };

            let mut pattern_eval = PatternEvaluator::new(default_pattern);
            let mut pattern_playstart = 0.0;
            let mut parameters = PatternEvaluatorParameters { time: 0.0, user_parameters: HashMap::new() };

            loop {
                let call = patteval_call_rx.recv().unwrap();
                match call {
                    PatternEvalCall::UpdatePattern{ mah_animation_json } => {
                        pattern_eval = PatternEvaluator::new_from_json_string(&mah_animation_json);
                    },
                    PatternEvalCall::UpdatePlaystart{ playstart, playstart_offset } => {
                        if playstart == 0.0 {
                            pattern_playstart = 0.0;
                        } else {
                            // get current time in milliseconds as f64
                            let current_time_ms: f64 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as f64 / 1_000_000.0;
                            pattern_playstart = current_time_ms + playstart_offset;
                        }
                    },
                    PatternEvalCall::EvalBatch{ time_arr_ms } => {
                        let eval_arr: Vec<_> = time_arr_ms.iter().map(|time| {
                            let time = if pattern_playstart == 0.0 { 0.0 } else { time - pattern_playstart };
                            parameters.time = time;
                            let eval = pattern_eval.eval_brush_at_anim_local_time(&parameters);
                            eval
                        }).collect();
                        patteval_return_tx.send(eval_arr).unwrap();
                    },
                }
            }
        })
        .unwrap();

    let ulh_streaming_handle_opt = if false {
        Some(thread::Builder::new()
            .name("ulh-streaming".to_string())
            .spawn(|| {
                println!("ulhaptics streaming thread started..");
                let mut ulh_streaming_controller = new_ulh_streaming_controller(500.0).unwrap();
                ulh_streaming_controller.pin_mut().resume_emitter().unwrap();
                ulh_streaming_controller.pin_mut().pause_emitter().unwrap();
                println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations().unwrap());
            })
            .unwrap())
    } else { None };


    let net_handle_opt = if false {
        todo!();
        // let (net_incoming_tx, net_incoming_rx) = std::sync::mpsc::sync_channel(0);
        // let (net_outgoing_tx, net_outgoing_rx) = std::sync::mpsc::sync_channel(0);
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(|| {
                println!("net thread started..");
                network::start_ws_server();
            })
            .unwrap();
        Some(thread)
    } else { None };


    pattern_eval_handle.join().unwrap();
    ulh_streaming_handle_opt.map(|h| h.join().unwrap());
    net_handle_opt.map(|h| h.join().unwrap());
}
