#[cxx::bridge]
pub(super) mod cxx_ffi {

	#[derive(Debug)]
	/// This struct is used to pass tracking data from the Leap Motion C++ SDK to Rust.
	///
	/// **Units are in millimeters.**
	///
	/// **+y is height from device**, +x is right, +z is towards user.
	struct LMCRawTrackingCoords {
		has_hand: bool,
		x: f64,
		y: f64,
		z: f64,
	}

	unsafe extern "C++" {
		include!("lmc-track.h");

		// using rust_tracking_callback = rust::Fn<bool(const LEAP_TRACKING_EVENT*)>;
		// void OpenConnectionAndStartMessagePump(rust_tracking_callback cb_func);
		fn OpenConnectionAndStartMessagePump(cb_func: fn(&LMCRawTrackingCoords), is_done: fn() -> bool);
	}

}