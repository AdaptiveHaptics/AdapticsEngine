mod ffi;

use std::sync::{atomic::AtomicBool, Mutex};

use ffi::cxx_ffi::*;

use super::TrackingFrame;

pub static TRACKING_IS_DONE: AtomicBool = AtomicBool::new(false);
fn tracking_is_done() -> bool {
	TRACKING_IS_DONE.load(std::sync::atomic::Ordering::Relaxed)
}

fn lmc_raw_to_tracking_frame(raw: &LMCRawTrackingCoords) -> TrackingFrame {
	TrackingFrame {
		hand: if !raw.has_hand { None } else { Some(pattern_evaluator::MAHCoordsConst {
			x: raw.x,
			y: -raw.z + 121.0, // 121mm is the offset from the LMC origin to the haptic origin
			z: raw.y, // flip y and z to match the haptic coordinate system
		})}
	}
}

/// set TRACKING_IS_DONE to true to stop the tracking loop
pub fn start_tracking_loop(
	tracking_data_tx: crossbeam_channel::Sender<TrackingFrame>,
) {
	type CallbackFn = Box<dyn Fn(&LMCRawTrackingCoords) + Send>;
	static STATIC_CALLBACK_MUTEX: Mutex<Option<CallbackFn>> = Mutex::new(None);
	fn static_tracking_callback(coords: &LMCRawTrackingCoords) {
		if let Some(f) = STATIC_CALLBACK_MUTEX.lock().unwrap().as_ref() {
			f(coords);
		}
	}

	let tracking_callback = move |raw_coords: &LMCRawTrackingCoords| {
		tracking_data_tx.send(lmc_raw_to_tracking_frame(raw_coords)).unwrap();
	};

	if let Some(_cb) = STATIC_CALLBACK_MUTEX.lock().unwrap().replace(Box::new(tracking_callback)) {
		panic!("cannot have multiple tracking loops running at once");
	}

	TRACKING_IS_DONE.store(false, std::sync::atomic::Ordering::Relaxed);

	OpenConnectionAndStartMessagePump(
		static_tracking_callback,
		tracking_is_done
	); // blocks until tracking_is_done() returns true
}