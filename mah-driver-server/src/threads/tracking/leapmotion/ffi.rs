#[cxx::bridge]
pub(super) mod cxx_ffi {

	#[derive(Debug)]
	struct RawTrackingCoords {
		has_hand: bool,
		x: f64,
		y: f64,
		z: f64,
	}

	unsafe extern "C++" {
		include!("lmc-track.h");

		// using rust_tracking_callback = rust::Fn<bool(const LEAP_TRACKING_EVENT*)>;
		// void OpenConnectionAndStartMessagePump(rust_tracking_callback cb_func);
		fn OpenConnectionAndStartMessagePump(cb_func: fn(&RawTrackingCoords), is_done: fn() -> bool);
	}

}