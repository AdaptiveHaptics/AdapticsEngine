#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::match_same_arms)]

/*!
# Adaptics Engine
Facilitates playback of adaptive mid-air ultrasound haptic sensations created in the [Adaptics Designer](https://github.com/AdaptiveHaptics/AdapticsDesigner).

## C-API Example
Example usage of the C-API to play a "loading" tacton and update its "progress" parameter from 0 to 1 over 2 seconds.
- [`adaptics_engine_init`](crate::adaptics_engine_init())
- [`adaptics_engine_play_tacton_immediate`](crate::adaptics_engine_play_tacton_immediate())
- [`adaptics_engine_update_user_parameter`](crate::adaptics_engine_update_user_parameter())
- [`adaptics_engine_deinit`](crate::adaptics_engine_deinit())

```ignore
#include "adapticsengine.h"
int main() {
    adaptics_engine_ffi_error err;
    adaptics_engine_ffi_handle* aeh;
    err = adaptics_engine_init(&aeh, true, false);
    if (err != ADAPTICS_ENGINE_FFI_ERROR_OK) { return 1; }

    // Immediately play the "loading" tacton
    err = adaptics_engine_play_tacton_immediate(aeh, read_file("loading.adaptics"));
    if (err != ADAPTICS_ENGINE_FFI_ERROR_OK) { return 1; }

    // Update tacton's "progress" parameter from 0 to 1 over 2 seconds
    for (double i = 0.0; i < 1.0; i += 0.01) {
        adaptics_engine_update_user_parameter(aeh, "progress", i);
        sleep_ms(20);
    }

    // wait for tacton to finish playing
    sleep_ms(2000);

    char err_msg[1024];
    err = adaptics_engine_deinit(&aeh, err_msg);
    if (err == ADAPTICS_ENGINE_FFI_ERROR_ERR_MSG_PROVIDED) { printf("AdapTics Error: %s\n", err_msg); }
    if (err != ADAPTICS_ENGINE_FFI_ERROR_OK) { return 1; }

    return 0;
}
```

*/

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, self};
use std::sync::RwLock;
use std::thread;
use interoptopus::patterns::slice::FFISliceMut;
use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_type, ffi_service, ffi_service_ctor, Inventory, InventoryBuilder, function};
use pattern_evaluator::{BrushAtAnimLocalTime, PatternEvaluator};


mod threads;
use threads::pattern::playback;
pub use playback::PatternEvalUpdate;
use threads::streaming;
use threads::net::websocket;
pub use websocket::AdapticsWSServerMessage;
use threads::tracking;
pub use pattern_evaluator::PatternEvaluatorParameters;
mod util;
use util::TLError;

pub mod hapticglove {
    pub type DeviceType = crate::streaming::hapticglove::DeviceType;
    pub use crate::streaming::hapticglove::get_possible_serial_ports;
}

/// The number of seconds between each playback update from the pattern evaluator.
pub const SECONDS_PER_PLAYBACK_UPDATE: f64 = 1.0 / 60.0;
const CALLBACK_RATE: f64 = 500.0;
const DEVICE_UPDATE_RATE: u64 = 20000; //20khz
const SEND_UNTRACKED_PLAYBACK_UPDATES: bool = false;

const DEBUG_LOG_LAG_EVENTS: bool = true;


/// Handle to the Adaptics Engine threads and channels.
pub struct AdapticsEngineHandle {
    end_streaming_tx: crossbeam_channel::Sender<()>,
    pattern_eval_handle: thread::JoinHandle<()>,
    patteval_update_tx: crossbeam_channel::Sender<playback::PatternEvalUpdate>,
    ulh_streaming_handle: thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    playback_updates_rx: Option<crossbeam_channel::Receiver<websocket::AdapticsWSServerMessage>>,
}

fn create_threads(
    use_mock_streaming: bool,
    disable_playback_updates: bool,
    vib_grid: Option<hapticglove::DeviceType>,
    tracking_data_rx: Option<crossbeam_channel::Receiver<tracking::TrackingFrame>>,
) -> AdapticsEngineHandle {
    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::bounded(1);
    let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::bounded(1);
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    let (playback_updates_tx, playback_updates_rx) = if disable_playback_updates { (None, None) } else { let (t,r) = crossbeam_channel::bounded(1); (Some(t), Some(r)) };

    let (end_streaming_tx, end_streaming_rx) = crossbeam_channel::bounded(1);

    // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let res = playback::pattern_eval_loop(
                SECONDS_PER_PLAYBACK_UPDATE,
                SEND_UNTRACKED_PLAYBACK_UPDATES,
                &patteval_call_rx,
                &patteval_update_rx,
                &patteval_return_tx,
                playback_updates_tx.as_ref(),
                tracking_data_rx.as_ref(),
            );

            // res.unwrap();
            res.ok(); // ignore error, only occurs when channel disconnected

            println!("pattern-eval thread exiting...");
        })
        .unwrap();

    let ulh_streaming_handle =  if let Some(vg_device) = vib_grid {
        thread::Builder::new()
            .name("vib-grid".to_string())
            .spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                println!("vib-grid thread starting...");
                streaming::hapticglove::start_streaming_emitter(&vg_device, &patteval_call_tx, &patteval_return_rx, &end_streaming_rx)
            }).unwrap()
    } else if !use_mock_streaming {
        thread::Builder::new()
            .name("ulh-streaming".to_string())
            .spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                println!("ulhaptics streaming thread starting...");

                #[allow(clippy::cast_possible_truncation)]
                streaming::ulhaptics::start_streaming_emitter(
                    CALLBACK_RATE as f32,
                    patteval_call_tx,
                    patteval_return_rx,
                    &end_streaming_rx,
                )
            }).unwrap()
    } else {
        println!("using mock streaming");
        thread::Builder::new()
            .name("mock-streaming".to_string())
            .spawn(move || {
                println!("mock streaming thread starting...");

                streaming::mock::start_mock_emitter(
                    DEVICE_UPDATE_RATE,
                    CALLBACK_RATE,
                    &patteval_call_tx,
                    &patteval_return_rx,
                    &end_streaming_rx,
                );

                // println!("mock streaming thread exiting...");
                Ok(())
            })
            .unwrap()
    };

    AdapticsEngineHandle {
        end_streaming_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        playback_updates_rx,
    }
}


/// Runs the main threads and waits for them to exit.
/// This is the main function for the CLI.
///
/// # Panics
/// Will panic if any of the threads panic (because panic may not not be `dyn std::error::Error + Send + Sync`).
pub fn run_threads_and_wait(
    use_mock_streaming: bool,
    websocket_bind_addr: Option<String>,
    enable_tracking: bool,
    vib_grid: Option<hapticglove::DeviceType>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let (tracking_data_tx, tracking_data_rx) = if enable_tracking { let (s, r) = crossbeam_channel::bounded(1); (Some(s), Some(r)) } else { (None, None) };

    let AdapticsEngineHandle {
        end_streaming_tx,
        pattern_eval_handle,
        patteval_update_tx,
        ulh_streaming_handle,
        playback_updates_rx,
    } = create_threads(use_mock_streaming, websocket_bind_addr.is_none(), vib_grid, tracking_data_rx);

    let (net_handle_opt, tracking_data_ws_tx) = if let Some(websocket_bind_addr) = websocket_bind_addr {
        let (tracking_data_ws_tx, tracking_data_ws_rx) = if enable_tracking { let (s, r) = crossbeam_channel::bounded(1); (Some(s), Some(r)) } else { (None, None) };
        let playback_updates_rx = playback_updates_rx.ok_or(TLError::new("playback_updates_rx must be available when using the websocket server"))?;
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                websocket::start_ws_server(&websocket_bind_addr, &patteval_update_tx, playback_updates_rx, tracking_data_ws_rx);
                println!("net thread thread exiting...");
            })?;
        (Some(thread), tracking_data_ws_tx)
    } else { Default::default() };

    let (end_tracking_tx, end_tracking_rx) = crossbeam_channel::bounded(1);
    let lmc_tracking_handle = if let Some(tracking_data_tx) = tracking_data_tx {
        let thread = thread::Builder::new()
            .name("lmc-tracking".to_string())
            .spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                println!("tracking thread starting...");
                tracking::leapmotion::start_tracking_loop(tracking_data_tx, tracking_data_ws_tx, &end_tracking_rx)
            })?;
        Some(thread)
    } else { None };


    // wait for threads to exit

    pattern_eval_handle.join().unwrap();

    end_streaming_tx.send(()).ok(); // ignore send error (if thread already exited)
    ulh_streaming_handle.join().unwrap()?; // unwrap panics, return errors

    if let Some(lmc_tracking_handle) = lmc_tracking_handle {
        end_tracking_tx.send(()).ok(); // ignore send error (if thread already exited)
        lmc_tracking_handle.join().unwrap()?; // unwrap panics, return errors
    }

    println!("waiting for net thread...");
    if let Some(h) = net_handle_opt { h.join().unwrap() }

    Ok(())
}


#[ffi_type(patterns(ffi_error))]
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum FFIError {
    Ok = 0,
    NullPassed = 1,
    Panic = 2,
    OtherError = 3,
    AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo = 4,
    ErrMsgProvided = 5,
    EnablePlaybackUpdatesWasFalse = 6,
    //NoPlaybackUpdatesAvailable = 7,
    ParamJSONDeserializationFailed = 8,
    HandleIDNotFound = 9,
    ParamUTF8Error = 10,
    MutexPoisoned = 11,
    ParamAsciiError = 12,
    InteropUnsupported = 13,
    InteropFormatError = 14,
    InteropUnkError = 15,
    TimeError = 16,
    CastError = 17,
}
// Gives special meaning to some of your error variants.
impl interoptopus::patterns::result::FFIError for FFIError {
    const SUCCESS: Self = Self::Ok;
    const NULL: Self = Self::NullPassed;
    const PANIC: Self = Self::Panic;
}
impl FFIError {
    /// not actually exposed to FFI yet, just enforcing I write error messages for new errors
    #[must_use]
    pub fn get_msg(&self) -> &'static str {
        match self {
            FFIError::Ok => "ok",
            FFIError::NullPassed => "A null pointer was passed where an actual element (likely AdapticsEngineHandleFFI) was needed.",
            FFIError::Panic => "A panic occurred. Further error information could not be marshalled.",
            FFIError::OtherError => "An error occurred. Further error information could not be marshalled.",
            FFIError::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo => "The AdapticsEngine thread disconnected. Check deinit_adaptics_engine for more information on what caused the disconnect.",
            FFIError::ErrMsgProvided => "An error occurred. Check err_msg parameter for more information.",
            FFIError::EnablePlaybackUpdatesWasFalse => "enable_playback_updates was false. Call init_adaptics_engine with enable_playback_updates set to true to enable playback updates.",
            // FFIError::NoPlaybackUpdatesAvailable => "No playback updates available. Playback updates are available at ~(1/SECONDS_PER_PLAYBACK_UPDATE)hz while a pattern is playing.",
            FFIError::ParamJSONDeserializationFailed => "Parameter JSON deserialization failed.",
            FFIError::HandleIDNotFound => "Handle ID not found.",
            FFIError::ParamUTF8Error => "Parameter UTF8 error.",
            FFIError::MutexPoisoned => "Mutex poisoned.",
            FFIError::ParamAsciiError => "Parameter ASCII error.",
            FFIError::InteropUnsupported => "Interop unsupported error.",
            FFIError::InteropFormatError => "Interop format error.",
            FFIError::InteropUnkError => "Interop unknown error.",
            FFIError::TimeError => "Error getting or using system time.",
            FFIError::CastError => "Error casting between types (e.g. from usize to u32).",
        }
    }
}
impl<T> From<Result<(), crossbeam_channel::SendError<T>>> for FFIError {
    fn from(value: Result<(), crossbeam_channel::SendError<T>>) -> Self {
        match value {
            Ok(()) => Self::Ok,
            Err(_) => Self::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo,
        }
    }
}
impl<T> From<crossbeam_channel::SendError<T>> for FFIError {
    fn from(_value: crossbeam_channel::SendError<T>) -> Self {
        Self::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo
    }
}
impl From<interoptopus::Error> for FFIError {
    fn from(value: interoptopus::Error) -> Self {
        match value {
            interoptopus::Error::Ascii => Self::ParamAsciiError,
            interoptopus::Error::Null => Self::NullPassed,
            interoptopus::Error::UTF8(_) |
            interoptopus::Error::FromUtf8(_) => Self::ParamUTF8Error,
            interoptopus::Error::Unsupported => Self::InteropUnsupported,
            interoptopus::Error::Format(_) => Self::InteropFormatError,
            _ => Self::InteropUnkError,
        }
    }
}
impl From<std::num::TryFromIntError> for FFIError {
    fn from(_value: std::num::TryFromIntError) -> Self {
        FFIError::CastError
    }
}
// impl<T> From<Result<T, FFIError>> for FFIError {
//     fn from(value: Result<T, FFIError>) -> Self {
//         match value {
//             Ok(_) => Self::Ok,
//             Err(e) => e,
//         }
//     }
// }
// impl std::ops::FromResidual<Result<std::convert::Infallible, FFIError>> for FFIError {
//     fn from_residual(residual: Result<std::convert::Infallible, FFIError>) -> Self {
//         match residual {
//             Ok(_) => Self::Ok,
//             Err(e) => e,
//         }
//     }
// }

/// `AdapticsEngineHandleFFI` is a simple opaque wrapper around `AdapticsEngineHandle`. It may also be used for error message reporting through the C API.
#[ffi_type(opaque)]
#[repr(C)]
pub struct AdapticsEngineHandleFFI {
    last_error_msg: Option<String>,
    aeh: AdapticsEngineHandle,
}
impl AdapticsEngineHandleFFI {
    fn new(aeh: AdapticsEngineHandle) -> Self {
        Self { aeh, last_error_msg: None, }
    }
}


type HandleID = u64;
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(0);
static ENGINE_HANDLE_MAP: RwLock<Option<HashMap<HandleID, AdapticsEngineHandleFFI>>> = RwLock::new(None);

#[ffi_type(opaque)]
pub struct FFIHandle {
    handle_id: HandleID,
}


/// Defines a 4x4 matrix in row-major order for FFI.
#[ffi_type]
#[repr(C)]
pub struct GeoMatrix {
    pub data: [f64; 16],
}

/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct UnityEvalCoords {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
/// !NOTE: y and z are swapped for Unity
#[ffi_type]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct UnityEvalResult {
    /// !NOTE: y and z are swapped for Unity
    pub coords: UnityEvalCoords,
    pub intensity: f64,
    pub pattern_time: f64,
    pub stop: bool,
}
impl From<BrushAtAnimLocalTime> for UnityEvalResult {
    fn from(be: BrushAtAnimLocalTime) -> UnityEvalResult {
        UnityEvalResult {
            // !NOTE: y and z are swapped for Unity
            coords: UnityEvalCoords {
                x: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.x),
                z: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.y), // !NOTE: y and z are swapped for Unity
                y: PatternEvaluator::unit_convert_dist_to_hapev2(&be.ul_control_point.coords.z), // !NOTE: y and z are swapped for Unity
            },
            intensity: be.ul_control_point.intensity,
            pattern_time: be.pattern_time,
            stop: be.stop,
        }
    }
}


macro_rules! deref_check_null {
    ($handle:expr) => {{
        if $handle.is_null() { return Err(FFIError::NullPassed); }
        unsafe { &mut *$handle }
    }};
}
macro_rules! get_handle_from_id {
    ($handle:ident <- $handle_id:expr) => {
        let rguard = ENGINE_HANDLE_MAP.read().or(Err(FFIError::MutexPoisoned))?;
        let $handle = rguard.as_ref().ok_or(FFIError::HandleIDNotFound)?.get(&$handle_id).ok_or(FFIError::HandleIDNotFound)?;
    };
}
macro_rules! deserialize_json_parameter {
    ($asciiptr:ident) => {
        if let Some(cstr) = $asciiptr.as_c_str() {
            if let Ok(value) = serde_json::from_slice(cstr.to_bytes()) { value }
            else { return Err(FFIError::ParamJSONDeserializationFailed); }
        } else { return Err(FFIError::ParamJSONDeserializationFailed); }
    };
}

mod ffimacrocontainer {
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::needless_pass_by_value)]
#[allow(clippy::wildcard_imports)]
use super::*;

#[ffi_service(error="FFIError", prefix="adaptics_engine_")]
impl FFIHandle {

    /// Initializes the Adaptics Engine, returns a handle ID.
    ///
    /// `use_mock_streaming`: if true, use mock streaming. if false, use ulhaptics streaming.
    ///
    /// `enable_playback_updates`: if true, enable playback updates, `adaptics_engine_get_playback_updates` expected to be called at (1/`SECONDS_PER_PLAYBACK_UPDATE`)hz.
    ///
    /// `vib_grid`: Alpha feature: Output to a vibrotactile grid device (e.g. a vest or glove) instead of a mid-air ultrasound haptic device.
    /// If len is 0, the vibrotactile grid feature is disabled. If "auto", the device will attempt to auto-detect the device.
    ///
    #[ffi_service_ctor]
    pub fn init_experimental(use_mock_streaming: bool, enable_playback_updates: bool, vib_grid: AsciiPointer) -> Result<Self, FFIError> {
        let vg = match vib_grid.as_str() {
            Ok("") | Err(interoptopus::Error::Null) => None,
            Ok("auto") => Some(hapticglove::DeviceType::Auto),
            Ok(s) => Some(hapticglove::DeviceType::SerialPort(s.to_string())),

            Err(interoptopus::Error::UTF8(_)) => { return Err(FFIError::ParamUTF8Error) },
            Err(e) => { eprintln!("WARN(AdapticsEngine): unexpected error {e}"); None }, //unreachable!(),
        };
        let aeh = create_threads(use_mock_streaming, !enable_playback_updates, vg, None);
        let ffi_handle = AdapticsEngineHandleFFI::new(aeh);
        // get map or create new map
        let mut map = ENGINE_HANDLE_MAP.write().or(Err(FFIError::MutexPoisoned))?;
        let map = map.get_or_insert_with(HashMap::new);
        let handle_id = NEXT_HANDLE_ID.fetch_add(1, atomic::Ordering::Relaxed);
        map.insert(handle_id, ffi_handle);
        Ok(Self { handle_id })
    }

    /// Initializes the Adaptics Engine, returns a handle ID.
    ///
    /// `use_mock_streaming`: if true, use mock streaming. if false, use ulhaptics streaming.
    ///
    /// `enable_playback_updates`: if true, enable playback updates, `adaptics_engine_get_playback_updates` expected to be called at (1/`SECONDS_PER_PLAYBACK_UPDATE`)hz.
    ///
    #[ffi_service_ctor]
    pub fn init(use_mock_streaming: bool, enable_playback_updates: bool) -> Result<Self, FFIError> {
        Self::init_experimental(use_mock_streaming, enable_playback_updates, AsciiPointer::empty())
    }

    /// Deinitializes the Adaptics Engine.
    /// Returns with an error message if available.
    ///
    /// The unity package uses a `err_msg` buffer of size 1024.
    pub fn deinit(&self, mut err_msg: FFISliceMut<u8>) -> Result<(), FFIError> {
        let mut rwlg = ENGINE_HANDLE_MAP.write().or(Err(FFIError::MutexPoisoned))?;
        let map = rwlg.as_mut().ok_or(FFIError::HandleIDNotFound)?;
        let handle = map.remove(&self.handle_id).ok_or(FFIError::HandleIDNotFound)?;
        handle.aeh.end_streaming_tx.send(()).ok(); // ignore send error (if thread already exited)
        if handle.aeh.pattern_eval_handle.join().is_err() { return Err(FFIError::Panic); }
        match handle.aeh.ulh_streaming_handle.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(res_err)) => {
                let err_msg_rv_slice = err_msg.as_slice_mut();
                let res_err_str_bytes = res_err.to_string().into_bytes();
                // copy as many bytes of res_err_str_bytes as possible into err_msg_rv_slice
                let bytes_to_copy = std::cmp::min(err_msg_rv_slice.len() - 1, res_err_str_bytes.len());
                err_msg_rv_slice[..bytes_to_copy].copy_from_slice(&res_err_str_bytes[..bytes_to_copy]);
                err_msg_rv_slice[bytes_to_copy] = 0; // null terminate
                Err(FFIError::ErrMsgProvided)
            },
            Err(_) => Err(FFIError::Panic),
        }
    }


    /// Updates the pattern to be played.
    /// For further information, see [`PatternEvalUpdate::Pattern`].
    pub fn update_pattern(&self, pattern_json: AsciiPointer) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Pattern { pattern_json: pattern_json.as_str()?.to_owned() })?;
        Ok(())
    }
    /// Alias for [`crate::adaptics_engine_update_pattern()`]
    pub fn update_tacton(&self, pattern_json: AsciiPointer) -> Result<(), FFIError> {
        self.update_pattern(pattern_json)
    }


    /// Used to start and stop playback.
    /// For further information, see [`PatternEvalUpdate::Playstart`].
    ///
    /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
    /// Use [`adaptics_engine_update_time()`] or [`adaptics_engine_update_parameters()`] to set the time parameter.
    pub fn update_playstart(&self, playstart: f64, playstart_offset: f64) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Playstart { playstart, playstart_offset })?)
    }

    /// Used to update all `evaluator_params`.
    ///
    /// Accepts a JSON string representing the evaluator parameters. See [`PatternEvaluatorParameters`].
    /// For further information, see [`PatternEvalUpdate::Parameters`].
    pub fn update_parameters(&self, evaluator_params: AsciiPointer) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        let evaluator_params = deserialize_json_parameter!(evaluator_params);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params })?)
    }

    /// Resets all evaluator parameters to their default values.
    /// For further information, see [`PatternEvalUpdate::Parameters`].
    pub fn reset_parameters(&self) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::Parameters { evaluator_params: PatternEvaluatorParameters::default() })?)
    }

    /// Updates `evaluator_params.time`.
    ///
    /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
    // This works because `next_eval_params.last_eval_pattern_time` will be updated to `evaluator_params.time` when a new playstart is received.
    ///
    /// # Notes
    /// - `evaluator_params.time` will be overwritten by the playstart time computation during playback.
    /// - Setting `evaluator_params.time` will not cause any pattern evaluation to occur (no playback updates).
    pub fn update_time(&self, time: f64) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::ParameterTime { time })?)
    }

    /// Updates all user parameters.
    /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
    /// For further information, see [`PatternEvalUpdate::UserParameters`].
    pub fn update_user_parameters(&self, user_parameters: AsciiPointer) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        let user_parameters = deserialize_json_parameter!(user_parameters);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::UserParameters { user_parameters })?)
    }

    /// Updates a single user parameter.
    /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
    /// For further information, see [`PatternEvalUpdate::UserParameters`].
    pub fn update_user_parameter(&self, name: AsciiPointer, value: f64) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::UserParameter { name: name.as_str()?.to_owned(), value })?)
    }

    /// Updates `geo_matrix`, a 4x4 matrix in row-major order, where `data[3]` is the fourth element of the first row (translate x).
    /// For further information, see [`PatternEvalUpdate::GeoTransformMatrix`].
    pub fn update_geo_transform_matrix(&self, geo_matrix: &GeoMatrix) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        let transform = {
            let g = geo_matrix.data;
            pattern_evaluator::GeometricTransformMatrix([
                [g[0], g[1], g[2], g[3]],
                [g[4], g[5], g[6], g[7]],
                [g[8], g[9], g[10], g[11]],
                [g[12], g[13], g[14], g[15]],
            ])
        };
        Ok(handle.aeh.patteval_update_tx.send(PatternEvalUpdate::GeoTransformMatrix { transform })?)
    }


    #[allow(clippy::not_unsafe_ptr_arg_deref)] // cant mark unsafe because it breaks interoptopus macro
    /// Actually Unsafe! This function is marked as unsafe because it dereferences a raw pointer.
    ///
    /// Populate `eval_results` with the latest evaluation results.
    /// `num_evals` will be set to the number of evaluations written to `eval_results`, or 0 if there are no new evaluations since the last call to this function.
    ///
    /// # Safety
    /// `num_evals` must be a valid pointer to a u32
    pub fn get_playback_updates(&self, eval_results: &mut FFISliceMut<UnityEvalResult>, num_evals: *mut u32) -> Result<(), FFIError> {
        get_handle_from_id!(handle <- self.handle_id);
        let num_evals = deref_check_null!(num_evals);
        match &handle.aeh.playback_updates_rx {
            Some(playback_updates_rx) => {
                match playback_updates_rx.try_recv() {
                    Ok(AdapticsWSServerMessage::PlaybackUpdate { evals }) => {
                        // copy as many evals as possible into eval_results
                        let eval_results_slice = eval_results.as_slice_mut();
                        let evalresults_to_copy = std::cmp::min(eval_results_slice.len(), evals.len());
                        evals.into_iter().take(evalresults_to_copy).enumerate().for_each(|(i, be)| eval_results_slice[i] = be.into());
                        *num_evals = u32::try_from(evalresults_to_copy)?;
                        Ok(())
                    },
                    Ok(AdapticsWSServerMessage::TrackingData { .. }) | // ignore tracking data
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        *num_evals = 0;
                        Ok(())
                    },
                    Err(crossbeam_channel::TryRecvError::Disconnected) => Err(FFIError::AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo),
                }
            },
            None => Err(FFIError::EnablePlaybackUpdatesWasFalse),
        }
    }


    /// Higher level function to load a new pattern and instantly start playback.
    pub fn adaptics_engine_play_tacton_immediate(&self, tacton_json: AsciiPointer) -> Result<(), FFIError> {
        self.update_pattern(tacton_json)?;
        self.reset_parameters()?;
        let playstart = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).or(Err(FFIError::TimeError))?.as_secs_f64() * 1000.0;
        let playstart_offset = 0.0;
        self.update_playstart(playstart, playstart_offset)
    }
}
}

/// Guard function used by bindings.
///
/// Change impl version in this comment to force bump the API version.
/// `impl_version`: 1
#[ffi_function]
#[no_mangle]
pub extern "C" fn ffi_api_guard() -> interoptopus::patterns::api_guard::APIVersion {
    ffi_inventory().into()
}

#[doc(hidden)]
#[must_use]
pub fn ffi_inventory() -> Inventory {
	InventoryBuilder::new()
        .register(interoptopus::pattern!(FFIHandle))
        .register(function!(ffi_api_guard))
        .inventory()
}


#[cfg(test)]
mod test {
    use std::{ffi::CString, time::{UNIX_EPOCH, SystemTime}};

    use crate::*;

    fn assert_good_deinit(eh: &FFIHandle) {
        let err_msg_u8 = &mut [0u8; 1024];
        let err_msg = FFISliceMut::from_slice(err_msg_u8);
        let rv = eh.deinit(err_msg);
        assert_eq!(rv, Ok(()));
        assert_eq!(err_msg_u8[0], 0u8);
    }

    #[test]
    fn test_update_user_params() {
        let eh = FFIHandle::init(true, false).unwrap();
        let cstr = CString::new("{\"dist\": 74.446439743042}").unwrap();
        let ap = AsciiPointer::from_cstr(&cstr);
        let rv = eh.update_user_parameters(ap);
        assert_eq!(rv, Ok(()));
        assert_good_deinit(&eh);
    }

    #[test]
    fn test_playback_updates_false() {
        let eh = FFIHandle::init(true, false).unwrap();
        let mut eval_results = Vec::with_capacity(1024);
        let mut eval_results = FFISliceMut::from_slice(&mut eval_results);
        let mut num_evals = 12345u32;
        let rv = eh.get_playback_updates(&mut eval_results, &mut num_evals);
        assert_eq!(rv, Err(FFIError::EnablePlaybackUpdatesWasFalse));
        assert_eq!(num_evals, 12345u32);
        assert_good_deinit(&eh);
    }

    #[test]
    fn test_playback_with_updates() {
        let eh = FFIHandle::init(true, true).unwrap();
        let mut eval_results = vec![UnityEvalResult::default(); 1024];
        let mut eval_results_slice = FFISliceMut::from_slice(&mut eval_results);
        let mut num_evals = 0u32;
        let rv = eh.get_playback_updates(&mut eval_results_slice, &mut num_evals);
        assert_eq!(rv, Ok(()));
        assert_eq!(num_evals, 0u32);


        {
            let pat = pattern_evaluator::MidAirHapticsAnimationFileFormat {
                data_format: pattern_evaluator::MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
                revision: pattern_evaluator::DataFormatRevision::CurrentRevision,
                name: "DEFAULT_PATTERN".to_string(),
                keyframes: vec![],
                pattern_transform: pattern_evaluator::PatternTransformation::default(),
                user_parameter_definitions: HashMap::new(),
            };
            let pat = serde_json::to_string(&pat).unwrap();
            let pat = CString::new(pat).unwrap();
            let pat = AsciiPointer::from_cstr(&pat);
            let rv = eh.update_pattern(pat);
            assert_eq!(rv, Ok(()));
        }

        {
            let pep = PatternEvaluatorParameters::default();
            let pep = serde_json::to_string(&pep).unwrap();
            let pep = CString::new(pep).unwrap();
            let pep = AsciiPointer::from_cstr(&pep);
            let rv = eh.update_parameters(pep);
            assert_eq!(rv, Ok(()));
        }

        let playstart = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0;
        let playstart_offset = 0.0;
        let rv = eh.update_playstart(playstart, playstart_offset);
        assert_eq!(rv, Ok(()));

        {
            let engine_map = ENGINE_HANDLE_MAP.read().unwrap();
            let handle = engine_map.as_ref().unwrap().get(&eh.handle_id).unwrap();
            let channel = handle.aeh.playback_updates_rx.as_ref().unwrap();
            let mut sel = crossbeam_channel::Select::new();
            let recv = sel.recv(channel);
            let op = sel.ready_timeout(std::time::Duration::from_secs_f64(SECONDS_PER_PLAYBACK_UPDATE * 1.5)).unwrap(); //should take ~SECONDS_PER_PLAYBACK_UPDATE
            assert_eq!(op, recv);
        }

        let rv = eh.get_playback_updates(&mut eval_results_slice, &mut num_evals);
        assert_eq!(rv, Ok(()));
        assert!(num_evals <= 1024u32); // assert did not overflow
        assert!(num_evals > 0u32); // assert got at least one eval

        // the exact value will vary to due lag (because mock emitter relies on real time) (typically only by 1 or 2 evals)
        #[allow(clippy::cast_precision_loss)]
        {
            assert!(f64::from(num_evals) > SECONDS_PER_PLAYBACK_UPDATE * DEVICE_UPDATE_RATE as f64 * 0.75); // assert got at least 75% of the evals for the time period
            assert!(f64::from(num_evals) < SECONDS_PER_PLAYBACK_UPDATE * DEVICE_UPDATE_RATE as f64 * 1.25); // assert got at most 125% of the evals for the time period
        }

        eval_results.truncate(num_evals as usize);
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(eval_results[0].coords, UnityEvalCoords { x: 0.0, y: 0.2, z: 0.0 });
            assert_eq!(eval_results[0].intensity, 1.0);
        }
        assert!(eval_results[0].pattern_time < 2.0 * 1000.0 * (1.0 / CALLBACK_RATE), "pattern_time: {} !< {}", eval_results[0].pattern_time, 1.0 / CALLBACK_RATE); // assert first pattern_time is less than 2.0 callback periods ahead


        assert_good_deinit(&eh);
    }
}