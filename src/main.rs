use cxx::CxxVector;


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

        fn new_ulh_streaming_controller(callback_rate: f32) -> Result<UniquePtr<ULHStreamingController>>;
    }
}

use ffi::*;

pub fn streaming_emission_callback(time_arr_ms: &CxxVector<f64>) -> Vec<EvalResults> {
    todo!();
    let v = time_arr_ms.iter().map(|t| EvalResults{ coords: EvalCoords { x: 0.0, y: 0.0, z: 0.0 }, intensity: 0.0}).collect();
    return v;
}


fn main() {
    println!("Hello, world!");

    let ulh_streaming_controller = new_ulh_streaming_controller(500.0).unwrap();
}
