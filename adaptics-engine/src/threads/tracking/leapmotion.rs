use crossbeam_channel::TrySendError;
#[allow(clippy::wildcard_imports)]
use leapc_dyn_sys::*;

use crate::{threads::net::websocket::AdapticsWSServerMessage, util::AdapticsError, DEBUG_LOG_LAG_EVENTS};

use super::{TrackingFrame, TrackingFrameHand, TrackingFrameHandChirality, TrackingFrameDigit, TrackingFrameBone, TrackingFramePalm};


#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct LMCRawTrackingVec3 {
	x: f64,
	y: f64,
	z: f64,
}
impl From<_LEAP_VECTOR> for LMCRawTrackingVec3 {
	fn from(v: _LEAP_VECTOR) -> Self {
		Self {
			x: unsafe { v.__bindgen_anon_1.__bindgen_anon_1.x }.into(),
			y: unsafe { v.__bindgen_anon_1.__bindgen_anon_1.y }.into(),
			z: unsafe { v.__bindgen_anon_1.__bindgen_anon_1.z }.into(),
		}
	}
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct LMCRawTrackingHand {
	has_hand: bool,
	left_hand: bool,
	palm: LMCRawTrackingPalm,
	digits: [LMCRawTrackingDigit; 5],
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct LMCRawTrackingPalm {
	position: LMCRawTrackingVec3,
	width: f64,
	normal: LMCRawTrackingVec3,
	direction: LMCRawTrackingVec3,
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct LMCRawTrackingDigit {
	bones: [LMCRawTrackingBone; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct LMCRawTrackingBone {
	start: LMCRawTrackingVec3,
	end: LMCRawTrackingVec3,
	width: f64,
}


struct LeapCSafe {
	lib: LeapC,
}
impl LeapCSafe {
	fn new() -> Result<Self, AdapticsError> {
		let lib = unsafe { load_leapc_library() }.map_err(|_e|
			AdapticsError::new(&format!("Failed to find and load {LIBRARY_BASENAME} dynamic library. Searched for '{LIBRARY_NAME}' and '{LIBRARY_FULLPATH}'."))
		)?;

		// I tried extracted the functions I wanted out of the `Result`s here, and keeping the reference on LeapCSafe, but this doesnt work:
		// 	Keeping just references to the functions causes the libloading __library to be dropped, causing the DLL to be unloaded (really dumb, I think this might be considered a bug on bindgen's usage of libloading).
		// 	Alternatively, keeping the reference to `lib` in the LeapCSafe causes LeapCSafe to be self referencing, which is itself a headache.
		// maybe could call mem::forget or something, but I'm just gonna leave it for now

		// verify that the library has the functions we need on initialization
		// errors when actually calling functions will cause a panic (because bindgen calls .expect)
		// we cant manually return the libloading errors later, because they will be behind the &self borrow, and libloading::Error doesnt implement Clone
		if let Err(e) = lib.LeapCreateConnection { return Err(e)?; }
		if let Err(e) = lib.LeapOpenConnection { return Err(e)?; }
		if let Err(e) = lib.LeapPollConnection { return Err(e)?; }
		if let Err(e) = lib.LeapCloseConnection { return Err(e)?; }
		if let Err(e) = lib.LeapDestroyConnection { return Err(e)?; }

		Ok(Self { lib })
	}

	fn create_connection(&self) -> Result<LEAP_CONNECTION, AdapticsError> {
		let mut connection_handle: LEAP_CONNECTION = std::ptr::null_mut();
		let res = unsafe { self.lib.LeapCreateConnection(std::ptr::null(), &mut connection_handle) };
		eleaprs_to_result(res)?;
		Ok(connection_handle)
	}
	fn open_connection(&self, connection_handle: LEAP_CONNECTION) -> Result<(), AdapticsError> {
		let res = unsafe { self.lib.LeapOpenConnection(connection_handle) };
		eleaprs_to_result(res)?;
		Ok(())
	}
	/// timeout is in milliseconds
	/// Returns Ok(None) if timeout
	fn poll_connection(&self, connection_handle: LEAP_CONNECTION, timeout: u32) -> Result<Option<LEAP_CONNECTION_MESSAGE>, AdapticsError> {
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

#[allow(clippy::needless_pass_by_value)] // idc
fn run_loop<'a>(cb_func: Box<dyn Fn(&LMCRawTrackingHand) + 'a>, is_done: Box<dyn Fn() -> bool + 'a>) -> Result<(), AdapticsError> {
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
						let digits: [LMCRawTrackingDigit; 5] = (0..5).map(|finger_index| {
							let bones: [LMCRawTrackingBone; 4] = (0..4).map(|bone_index| {
								let bone = unsafe { &hand.__bindgen_anon_1.digits[finger_index].__bindgen_anon_1.bones[bone_index] };

								LMCRawTrackingBone {
									start: bone.prev_joint.into(),
									end: bone.next_joint.into(),
									width: bone.width.into(),
								}
							}).collect::<Vec<_>>().try_into().unwrap(); // Converting Vec to fixed-size array

							LMCRawTrackingDigit { bones, }
						}).collect::<Vec<_>>().try_into().unwrap(); // Converting Vec to fixed-size array
						LMCRawTrackingHand {
							has_hand: true,
							left_hand: hand.type_ == _eLeapHandType_eLeapHandType_Left,
							palm: LMCRawTrackingPalm {
								position: hand.palm.position.into(),
								width: hand.palm.width.into(),
								normal: hand.palm.normal.into(),
								direction: hand.palm.direction.into(),
							},
							digits,
						}
					},
					_ => LMCRawTrackingHand::default()
				};
				cb_func(&lmc_raw_coords);
			} else {
				// we don't care about other events
			}
		}
		// else loop if timeout
	}

	leap_c_safe.close_connection(connection_handle);
	leap_c_safe.destroy_connection(connection_handle);

	Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ELeapRS {
	Success,
	UnknownError,
	InvalidArgument,
	InsufficientResources,
	InsufficientBuffer,
	Timeout,
	NotConnected,
	HandshakeIncomplete,
	BufferSizeOverflow,
	ProtocolError,
	InvalidClientID,
	UnexpectedClosed,
	UnknownImageFrameRequest,
	UnknownTrackingFrameID,
	RoutineIsNotSeer,
	TimestampTooEarly,
	ConcurrentPoll,
	NotAvailable,
	NotStreaming,
	CannotOpenDevice,
	Unknown,
}
impl std::error::Error for ELeapRS {}
impl std::fmt::Display for ELeapRS {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}
impl From<_eLeapRS> for ELeapRS {
	fn from(value: _eLeapRS) -> Self {
		match value {
			leapc_dyn_sys::_eLeapRS_eLeapRS_Success                  => ELeapRS::Success,
			leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownError             => ELeapRS::UnknownError,
			leapc_dyn_sys::_eLeapRS_eLeapRS_InvalidArgument          => ELeapRS::InvalidArgument,
			leapc_dyn_sys::_eLeapRS_eLeapRS_InsufficientResources    => ELeapRS::InsufficientResources,
			leapc_dyn_sys::_eLeapRS_eLeapRS_InsufficientBuffer       => ELeapRS::InsufficientBuffer,
			leapc_dyn_sys::_eLeapRS_eLeapRS_Timeout                  => ELeapRS::Timeout,
			leapc_dyn_sys::_eLeapRS_eLeapRS_NotConnected             => ELeapRS::NotConnected,
			leapc_dyn_sys::_eLeapRS_eLeapRS_HandshakeIncomplete      => ELeapRS::HandshakeIncomplete,
			leapc_dyn_sys::_eLeapRS_eLeapRS_BufferSizeOverflow       => ELeapRS::BufferSizeOverflow,
			leapc_dyn_sys::_eLeapRS_eLeapRS_ProtocolError            => ELeapRS::ProtocolError,
			leapc_dyn_sys::_eLeapRS_eLeapRS_InvalidClientID          => ELeapRS::InvalidClientID,
			leapc_dyn_sys::_eLeapRS_eLeapRS_UnexpectedClosed         => ELeapRS::UnexpectedClosed,
			leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownImageFrameRequest => ELeapRS::UnknownImageFrameRequest,
			leapc_dyn_sys::_eLeapRS_eLeapRS_UnknownTrackingFrameID   => ELeapRS::UnknownTrackingFrameID,
			leapc_dyn_sys::_eLeapRS_eLeapRS_RoutineIsNotSeer         => ELeapRS::RoutineIsNotSeer,
			leapc_dyn_sys::_eLeapRS_eLeapRS_TimestampTooEarly        => ELeapRS::TimestampTooEarly,
			leapc_dyn_sys::_eLeapRS_eLeapRS_ConcurrentPoll           => ELeapRS::ConcurrentPoll,
			leapc_dyn_sys::_eLeapRS_eLeapRS_NotAvailable             => ELeapRS::NotAvailable,
			leapc_dyn_sys::_eLeapRS_eLeapRS_NotStreaming             => ELeapRS::NotStreaming,
			leapc_dyn_sys::_eLeapRS_eLeapRS_CannotOpenDevice         => ELeapRS::CannotOpenDevice,
			_ => ELeapRS::Unknown,
		}
	}
}
impl From<ELeapRS> for Result<(), ELeapRS> {
	fn from(value: ELeapRS) -> Self {
		match value {
			ELeapRS::Success => Ok(()),
			_ => Err(value),
		}
	}
}
fn eleaprs_to_result(res: eLeapRS) -> Result<(), ELeapRS> {
	ELeapRS::from(res).into()
}

impl LMCRawTrackingVec3 {
	fn to_mah_as_coords(self) -> pattern_evaluator::MAHCoordsConst {
		pattern_evaluator::MAHCoordsConst {
			x: self.x,
			y: -self.z + 121.0, // 121mm is the offset from the LMC origin to the haptic origin
			z: self.y, // flip y and z to match the haptic coordinate system
		}
	}
	fn to_mah_as_vector(self) -> pattern_evaluator::MAHCoordsConst {
		pattern_evaluator::MAHCoordsConst {
			x: self.x,
			y: -self.z, // flip y and z to match the haptic coordinate system
			z: self.y, // flip y and z to match the haptic coordinate system
		}
	}
}


impl From<&LMCRawTrackingHand> for TrackingFrame {
	fn from(raw: &LMCRawTrackingHand) -> Self {
		TrackingFrame {
			hand: if raw.has_hand {
				Some(TrackingFrameHand {
					chirality: if raw.left_hand { TrackingFrameHandChirality::Left } else { TrackingFrameHandChirality::Right },
					palm: TrackingFramePalm {
						position: raw.palm.position.to_mah_as_coords(),
						width: raw.palm.width,
						normal: raw.palm.normal.to_mah_as_vector(),
						direction: raw.palm.direction.to_mah_as_vector(),
					},
					digits: raw.digits.iter().map(|raw_digit| {
						TrackingFrameDigit {
							bones: raw_digit.bones.iter().map(|raw_bone| {
								TrackingFrameBone {
									start: raw_bone.start.to_mah_as_coords(),
									end: raw_bone.end.to_mah_as_coords(),
									width: raw_bone.width,
								}
							}).collect::<Vec<_>>().try_into().unwrap(), // Converting Vec to fixed-size array
						}
					}).collect::<Vec<_>>().try_into().unwrap(), // Converting Vec to fixed-size array
				})
			} else { None }
		}
	}
}

pub fn start_tracking_loop(
	tracking_data_tx: crossbeam_channel::Sender<TrackingFrame>,
	tracking_data_ws_tx: Option<crossbeam_channel::Sender<AdapticsWSServerMessage>>,
	end_tracking_rx: &crossbeam_channel::Receiver<()>,
) -> Result<(), AdapticsError> {
	let tracking_callback = move |raw_coords: &LMCRawTrackingHand| {
		let tracking_frame: TrackingFrame = raw_coords.into();
		match tracking_data_tx.try_send(tracking_frame.clone()) {
			Ok(()) => {},
			Err(TrySendError::Disconnected(_)) => {}, // is_done() should return true, so the run loop will exit
			Err(TrySendError::Full(_)) => { if DEBUG_LOG_LAG_EVENTS { println!("playback thread lagged [tracking]"); } }, // we are sending too fast for playback thread, so we can just drop this frame
		}
		if let Some(tracking_data_ws_tx) = tracking_data_ws_tx.as_ref() {
			match tracking_data_ws_tx.try_send(AdapticsWSServerMessage::TrackingData { tracking_frame }) {
				Ok(()) => {},
				Err(TrySendError::Disconnected(_)) => {}, // is_done() should return true, so the run loop will exit
				Err(TrySendError::Full(_)) => { if DEBUG_LOG_LAG_EVENTS { println!("network thread lagged [tracking]"); } }, // we are sending too fast for network thread, so we can just drop this frame
			}
		}
	};
	let is_done = || end_tracking_rx.try_recv().is_ok();

	run_loop(Box::new(tracking_callback), Box::new(is_done))?;

	Ok(())
}