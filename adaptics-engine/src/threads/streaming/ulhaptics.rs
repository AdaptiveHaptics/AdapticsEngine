mod ffi;
use core::panic;
use std::{pin::Pin, sync::Mutex, time::Instant, ops::Add};

#[allow(clippy::wildcard_imports)]
use ffi::*;
#[allow(clippy::wildcard_imports)]
use ffi::cxx_ffi::*;
use pattern_evaluator::{PatternEvaluator, BrushAtAnimLocalTime};

use crate::{threads::{common::{js_milliseconds_to_duration, MilSec}, pattern::playback::PatternEvalCall}, util::AdapticsError};

impl From<BrushAtAnimLocalTime> for EvalResult {
    fn from(be: BrushAtAnimLocalTime) -> EvalResult {
        EvalResult {
            coords: EvalCoords {
                x: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.x),
                y: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.y),
                z: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.z),
            },
            intensity: be.ul_control_point.intensity,
        }
    }
}

pub fn start_streaming_emitter(
	callback_rate: f32,
	patteval_call_tx: crossbeam_channel::Sender<PatternEvalCall>,
	patteval_return_rx: crossbeam_channel::Receiver<Vec<BrushAtAnimLocalTime>>,
	end_streaming_rx: &crossbeam_channel::Receiver<()>,
) -> Result<(), AdapticsError> {
	type CallbackFn = Box<dyn Fn(&CxxVector<MilSec>, Pin<&mut CxxVector<EvalResult>>) + Send>;
	static STATIC_ECALLBACK_MUTEX: Mutex<Option<CallbackFn>> = Mutex::new(None);

	// sync epochs are used to convert from chrono time to Instant
	// they both appear to use the same monotonic clock source and unix epoch, but i'd like to be agnostic of that assumption
	// I am assuming that these be called at the nearly the same time, in either order
	let sync_epoch_instant = Instant::now();
	let sync_epoch_chrono_ms = get_current_chrono_time();

	#[allow(clippy::items_after_statements)]
	fn static_streaming_emission_callback(time_arr_ms: &CxxVector<MilSec>, eval_results_arr: Pin<&mut CxxVector<EvalResult>>) {
		if let Some(f) = STATIC_ECALLBACK_MUTEX.lock().unwrap().as_ref() {
			f(time_arr_ms, eval_results_arr);
		}
	}
	let streaming_emission_callback = move |time_arr_ms: &CxxVector<MilSec>, eval_results_arr: Pin<&mut CxxVector<EvalResult>> | {
		if patteval_call_tx.send(PatternEvalCall::EvalBatch{
			time_arr_instants: time_arr_ms.iter().map(|ms| sync_epoch_instant.add(js_milliseconds_to_duration(ms-sync_epoch_chrono_ms))).collect() // convert from chrono time to Instant using epoch
		}).is_ok() {
			let eval_arr = patteval_return_rx.recv().unwrap();
			let eval_results_arr = eval_results_arr.as_mut_slice();
			for (i,eval) in eval_arr.into_iter().enumerate() {
				eval_results_arr[i] = eval.into();
			}
		} else {
			// patt eval thread exited (or panicked),
			// end_streaming_rx will be called, triggering drop() + ULHStreamingController::~ULHStreamingController()
			//# note:  its seems that ULHStreamingController::~ULHStreamingController() was already called, and this callback is still being called due to multithreading internal to Ultraleap
		}
	};
	if let Some(_cb) = STATIC_ECALLBACK_MUTEX.lock().or(Err(AdapticsError::new("STATIC_ECALLBACK_MUTEX poisoned")))?.replace(Box::new(streaming_emission_callback)) {
		panic!("cannot have multiple streaming emitters running at once");
	}

	match new_ulh_streaming_controller(callback_rate, static_streaming_emission_callback) {
		Ok(mut ulh_streaming_controller) => {
			ulh_streaming_controller.pin_mut().resume_emitter()?;
			// println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations()?); # 0
			end_streaming_rx.recv()?;
			println!("getMissedCallbackIterations: {}", ulh_streaming_controller.getMissedCallbackIterations()?);
			drop(ulh_streaming_controller);
			let cb = STATIC_ECALLBACK_MUTEX.lock().or(Err(AdapticsError::new("STATIC_ECALLBACK_MUTEX poisoned")))?.take();
			drop(cb);
			Ok(())
		},
		Err(e) => {
			let cb = STATIC_ECALLBACK_MUTEX.lock().or(Err(AdapticsError::new("STATIC_ECALLBACK_MUTEX poisoned")))?.take();
			drop(cb);
			Err(AdapticsError::new(&format!("Error creating UltraLeap Haptics streaming controller: {e}")))
		}
	}
}