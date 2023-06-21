pub(super) use cxx::CxxVector;

#[cxx::bridge]
pub(super) mod cxx_ffi {

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
