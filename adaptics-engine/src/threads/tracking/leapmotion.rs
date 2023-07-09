use std::sync::atomic::AtomicBool;

use leapc_dyn_sys::*;

use super::TrackingFrame;


const LEAP_LIBRARY_NAME: &str = "LeapC";
const LEAP_LIBRARY_PATH: &str = "C:/Program Files/Ultraleap/LeapSDK/lib/x64";

pub struct LMCRawTrackingCoords {
	pub has_hand: bool,
	pub x: f64,
	pub y: f64,
	pub z: f64,
}

struct LeapCSafe {
	lib: LeapC,
}
impl LeapCSafe {
	fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let lib = unsafe { LeapC::new(LEAP_LIBRARY_NAME) }.or_else(|_e| unsafe { LeapC::new(LEAP_LIBRARY_PATH) })?;
		// I tried extracted the functions I wanted out of the `Result`s here, and keeping the reference on LeapCSafe, but this doesnt work:
		// 	Keeping just references to the functions causes the libloading __library to be dropped, causing the DLL to be unloaded (really dumb, I think this might be considered a bug on bindgen's usage of libloading).
		// 	Alternatively, keeping the reference to `lib` in the LeapCSafe causes LeapCSafe to be self referencing, which is itself a headache.
		// maybe could call mem::forget or something, but I'm just gonna leave it for now

		// verify that the library has the functions we need on initialization
		// errors when actually calling functions will cause a panic (because bindgen calls .expect)
		// we cant manually return the libloading errors later, because they will be behind the &self borrow, and libloading::Error doesnt implement Clone
		if let Err(e) = lib.LeapCreateConnection { return Err(Box::new(e)); }
		if let Err(e) = lib.LeapOpenConnection { return Err(Box::new(e)); }
		if let Err(e) = lib.LeapPollConnection { return Err(Box::new(e)); }
		if let Err(e) = lib.LeapCloseConnection { return Err(Box::new(e)); }
		if let Err(e) = lib.LeapDestroyConnection { return Err(Box::new(e)); }

		Ok(Self { lib })
	}

	fn create_connection(&self) -> Result<LEAP_CONNECTION, Box<dyn std::error::Error + Send + Sync>> {
		let mut connection_handle: LEAP_CONNECTION = std::ptr::null_mut();
		let res = unsafe { self.lib.LeapCreateConnection(std::ptr::null(), &mut connection_handle) };
		eleaprs_to_result(res)?;
		Ok(connection_handle)
	}
	fn open_connection(&self, connection_handle: LEAP_CONNECTION) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		let res = unsafe { self.lib.LeapOpenConnection(connection_handle) };
		eleaprs_to_result(res)?;
		Ok(())
	}
	/// timeout is in milliseconds
	/// Returns Ok(None) if timeout
	fn poll_connection(&self, connection_handle: LEAP_CONNECTION, timeout: u32) -> Result<Option<LEAP_CONNECTION_MESSAGE>, Box<dyn std::error::Error + Send + Sync>> {
		let mut msg: LEAP_CONNECTION_MESSAGE = unsafe { std::mem::zeroed() };
		let res = unsafe { self.lib.LeapPollConnection(connection_handle, timeout, &mut msg) };
		if res == _eLeapRS_eLeapRS_Timeout {
			Ok(None)
		} else {
			eleaprs_to_result(res)?;
			Ok(Some(msg))
		}
	}

	fn close_connection(&self, connection_handle: LEAP_CONNECTION) {
		unsafe { self.lib.LeapCloseConnection(connection_handle) };
	}

	fn destroy_connection(&self, connection_handle: LEAP_CONNECTION) {
		unsafe { self.lib.LeapDestroyConnection(connection_handle) };
	}
}

fn run_loop<'a>(cb_func: Box<dyn Fn(&LMCRawTrackingCoords) + 'a>, is_done: Box<dyn Fn() -> bool + 'a>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
	let leap_c_safe = LeapCSafe::new()?;

	let connection_handle = leap_c_safe.create_connection()?;
	leap_c_safe.open_connection(connection_handle)?;

	while !is_done() {
		let timeout_ms = 1000;
		if let Some(msg) = leap_c_safe.poll_connection(connection_handle, timeout_ms)? {
			if msg.type_ == _eLeapEventType_eLeapEventType_Tracking {
				let tracking_event = unsafe { msg.__bindgen_anon_1.tracking_event.as_ref() };
				// if tracking event is null, or tracking_frame_id is 0, or nHands is 0, call cb with has_hand = false
				// else, call cb with has_hand = true, and x, y, z from the first hand
				let lmc_raw_coords = match tracking_event {
					Some(tracking_event) if tracking_event.tracking_frame_id != 0 && tracking_event.nHands > 0 => {
						let hand = unsafe { *tracking_event.pHands.offset(0) };
						LMCRawTrackingCoords {
							has_hand: true,
							x: unsafe { hand.palm.position.__bindgen_anon_1.__bindgen_anon_1.x }.into(),
							y: unsafe { hand.palm.position.__bindgen_anon_1.__bindgen_anon_1.y }.into(),
							z: unsafe { hand.palm.position.__bindgen_anon_1.__bindgen_anon_1.z }.into(),
						}
					},
					_ => LMCRawTrackingCoords {
						has_hand: false,
						x: 0.0,
						y: 0.0,
						z: 0.0,
					}
				};
				cb_func(&lmc_raw_coords);
			} else {
				// we don't care about other events
			}
		} else {
			continue; // loop if timeout
		}
	}

	leap_c_safe.close_connection(connection_handle);
	leap_c_safe.destroy_connection(connection_handle);

	Ok(())
}

fn eleaprs_to_result(res: eLeapRS) -> Result<(), &'static str> {
	match res {
		leapc_dyn_sys::_eLeapRS_eLeapRS_Success                  => Ok(()),
		leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownError             => Err("eLeapRS_UnknownError"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_InvalidArgument          => Err("eLeapRS_InvalidArgument"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_InsufficientResources    => Err("eLeapRS_InsufficientResources"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_InsufficientBuffer       => Err("eLeapRS_InsufficientBuffer"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_Timeout                  => Err("eLeapRS_Timeout"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_NotConnected             => Err("eLeapRS_NotConnected"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_HandshakeIncomplete      => Err("eLeapRS_HandshakeIncomplete"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_BufferSizeOverflow       => Err("eLeapRS_BufferSizeOverflow"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_ProtocolError            => Err("eLeapRS_ProtocolError"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_InvalidClientID          => Err("eLeapRS_InvalidClientID"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_UnexpectedClosed         => Err("eLeapRS_UnexpectedClosed"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownImageFrameRequest => Err("eLeapRS_UnknownImageFrameRequest"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownTrackingFrameID   => Err("eLeapRS_UnknownTrackingFrameID"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_RoutineIsNotSeer         => Err("eLeapRS_RoutineIsNotSeer"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_TimestampTooEarly        => Err("eLeapRS_TimestampTooEarly"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_ConcurrentPoll           => Err("eLeapRS_ConcurrentPoll"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_NotAvailable             => Err("eLeapRS_NotAvailable"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_NotStreaming             => Err("eLeapRS_NotStreaming"),
		leapc_dyn_sys::_eLeapRS_eLeapRS_CannotOpenDevice         => Err("eLeapRS_CannotOpenDevice"),
		_ => Err("Unknown eLeapRS"),
	}
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

pub fn start_tracking_loop<'a>(
	tracking_data_tx: crossbeam_channel::Sender<TrackingFrame>,
	end_tracking_rx: crossbeam_channel::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
	let tracking_callback = move |raw_coords: &LMCRawTrackingCoords| {
		tracking_data_tx.send(lmc_raw_to_tracking_frame(raw_coords)).ok(); // ignore send errors, is_done should exit
	};
	let is_done = || end_tracking_rx.try_recv().is_ok();

	run_loop(Box::new(tracking_callback), Box::new(is_done))?;

	Ok(())
}