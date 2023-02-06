use std::thread;
use cxx::CxxVector;

mod jsrunner;


#[cxx::bridge]
mod ffi {
    struct EvalCoords {
        x: f64,
        y: f64,
        z: f64,
    }
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

/// I am not sure about any threading/concurrency issues
pub fn streaming_emission_callback(time_arr_ms: &CxxVector<f64>) -> Vec<EvalResults> {
    todo!();
    let v = time_arr_ms.iter().map(|t| EvalResults{ coords: EvalCoords { x: 0.0, y: 0.0, z: 0.0 }, intensity: 0.0}).collect();
    return v;
}


fn main() {
    println!("Hello, world!");

    let (js_call_tx, js_call_rx) = std::sync::mpsc::sync_channel(0);
    let (js_return_tx, js_return_rx) = std::sync::mpsc::sync_channel(0);

    let v8js_handle = thread::Builder::new()
        .name("v8js".to_string())
        .spawn(|| {
            println!("v8js thread started..");
            jsrunner::initv8(js_call_rx);
        })
        .unwrap();

    if false {
        let mut ulh_streaming_controller = new_ulh_streaming_controller(500.0).unwrap();
        ulh_streaming_controller.pin_mut().resume_emitter().unwrap();
        ulh_streaming_controller.pin_mut().pause_emitter().unwrap();
        println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations().unwrap());
    }


    v8js_handle.join().unwrap();
}
