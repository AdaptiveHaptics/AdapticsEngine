use std::collections::HashMap;
use std::ops::Sub;
use std::time::Instant;
use pattern_evaluator::{PatternEvaluator, PatternEvaluatorParameters, BrushAtAnimLocalTime, NextEvalParams, MAHTime};
use serde::{Deserialize, Serialize};
use crate::threads::{common::{ MilSec, instant_add_js_milliseconds }, net::websocket::PEWSServerMessage, tracking::TrackingFrame};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum PatternEvalUpdate {
	#[serde(rename="update_pattern")]
    Pattern{ pattern_json: String },
	#[serde(rename="update_playstart")]
	/// if playstart is 0.0, then the pattern is stopped. Otherwise, it is started at the time given by `now() + playstart_offset` to account for network latency/time desynchronization.
	/// Network latency would cause `rtt/2` of the pattern playback to be skipped, even if the time between machines is perfectly synced
	/// Time desynchronization would cause generally unpredicateable playback behavior
    Playstart{ playstart: MilSec, playstart_offset: MilSec },
	#[serde(rename="update_parameters")]
    Parameters{ evaluator_params: PatternEvaluatorParameters },
	#[serde(rename="update_tracking")]
	Tracking{ enabled: bool },

	//*** currently not sent over websocket, just for lib ***//
	ParameterTime { time: MAHTime },
	UserParameters { user_parameters: pattern_evaluator::UserParameters },
	GeoTransformMatrix { transform: pattern_evaluator::GeometricTransformMatrix },
}

pub enum PatternEvalCall {
    EvalBatch{ time_arr_instants: Vec<Instant>},
}

/// if seconds_per_playback_update is true, send playback updates prior to applying tracking translation
pub fn pattern_eval_loop(
	seconds_per_playback_update: f64,
	send_untracked_playback_updates: bool,
	patteval_call_rx: crossbeam_channel::Receiver<PatternEvalCall>,
	patteval_update_rx: crossbeam_channel::Receiver<PatternEvalUpdate>,
	patteval_return_tx: crossbeam_channel::Sender<Vec<BrushAtAnimLocalTime>>,
	playback_updates_tx: Option<crossbeam_channel::Sender<PEWSServerMessage>>,
	tracking_data_rx: Option<crossbeam_channel::Receiver<TrackingFrame>>,
) -> Result<(), crossbeam_channel::RecvError> {
	let default_pattern = pattern_evaluator::MidAirHapticsAnimationFileFormat {
		data_format: pattern_evaluator::MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
		revision: pattern_evaluator::DataFormatRevision::CurrentRevision,
		name: "DEFAULT_PATTERN".to_string(),
		keyframes: vec![],
		pattern_transform: Default::default(),
		user_parameter_definitions: HashMap::new(),
	};

	let mut pattern_eval = PatternEvaluator::new(default_pattern);
	let mut pattern_playstart: Option<Instant> = None;
	let mut parameters = PatternEvaluatorParameters { time: 0.0, user_parameters: HashMap::new(), geometric_transform: Default::default() };
	let mut tracking_data: TrackingFrame = TrackingFrame { hand: None };
	let mut enable_tracking = false;

	let mut last_playback_update = Instant::now();
	let mut playback_update_buffer: Vec<BrushAtAnimLocalTime> = Vec::with_capacity(1024); // 20khz / 60hz = ~333.33 is the number of EvalResults sent in a batch

	let mut next_eval_params = NextEvalParams::default();

	loop {
		// not using select macro because of https://github.com/rust-lang/rust-analyzer/issues/11847
		let mut sel = crossbeam_channel::Select::new();
		let patteval_call_rx_idx = sel.recv(&patteval_call_rx);
		let patteval_update_rx_idx = sel.recv(&patteval_update_rx);
		let tracking_data_rx_idx = tracking_data_rx.as_ref().map(|tracking_data_rx| sel.recv(tracking_data_rx));
		let oper = sel.select();
		match oper.index() {
			i if i == patteval_call_rx_idx => {
				let call = oper.recv(&patteval_call_rx)?;
				match call {
					PatternEvalCall::EvalBatch{ time_arr_instants } => {
						let eval_arr: Vec<_> = time_arr_instants.iter().map(|time| {
							if let Some(playstart) = pattern_playstart {
								parameters.time = time.sub(playstart).as_nanos() as f64 / 1e6;
							} //else reuse the last parameters.time
							let eval = pattern_eval.eval_brush_at_anim_local_time(&parameters, &next_eval_params);
							next_eval_params = eval.next_eval_params.clone();
							eval
						}).collect();


						#[allow(clippy::collapsible_if)]
						if send_untracked_playback_updates {
							if pattern_playstart.is_some() { playback_update_buffer.extend_from_slice(&eval_arr); }
						}

						// apply tracking
						let eval_arr_tracking_adjusted = if enable_tracking {
							if let Some(hand_pos) = &tracking_data.hand {
								// shift evals by hand_pos
								let mut eval_arr = eval_arr;
								for e in &mut eval_arr {
									e.ul_control_point.coords.x += hand_pos.x;
									e.ul_control_point.coords.y += hand_pos.y;
									e.ul_control_point.coords.z = hand_pos.z;
								}
								eval_arr
							} else { eval_arr }
						} else { eval_arr };

						#[allow(clippy::collapsible_if)]
						if !send_untracked_playback_updates {
							if pattern_playstart.is_some() { playback_update_buffer.extend_from_slice(&eval_arr_tracking_adjusted); }
						}

						// send tracked evals to haptic device
						patteval_return_tx.send(eval_arr_tracking_adjusted).unwrap();


						if pattern_playstart.is_some() && (Instant::now() - last_playback_update).as_secs_f64() > seconds_per_playback_update {
							last_playback_update = Instant::now();
							if playback_update_buffer.is_empty() {
								println!("[warn] skipping network update (no evals)");
								continue;
							}
							// else { println!("sending network update ({} evals)", playback_update_buffer.len()); }
							if let Some(playback_updates_tx) = &playback_updates_tx {
								match playback_updates_tx.try_send(PEWSServerMessage::PlaybackUpdate{ evals: playback_update_buffer.clone() }) {
									Err(crossbeam_channel::TrySendError::Full(_)) => { println!("network thread lagged"); },
									res => res.unwrap()
								}
							}
							playback_update_buffer.clear();
						}
					},
				}
			},
			i if i == patteval_update_rx_idx => {
				let update = oper.recv(&patteval_update_rx)?;
				match update {
					PatternEvalUpdate::Pattern{ pattern_json } => {
						pattern_eval = PatternEvaluator::new_from_json_string(&pattern_json).unwrap(); //todo: handle error (not sure how to propagate it to calling thread)
					},
					PatternEvalUpdate::Parameters{ evaluator_params } => {
						parameters = evaluator_params;
					},
					PatternEvalUpdate::Playstart{ playstart, playstart_offset } => {
						// println!("playstart: {}, playstart_offset: {}", playstart, playstart_offset);
						if playstart == 0.0 {
							pattern_playstart = None;
						} else {
							// get current time in milliseconds as f64
							last_playback_update = Instant::now();
							playback_update_buffer.clear();
							pattern_playstart = Some(instant_add_js_milliseconds(Instant::now(), playstart_offset));
							next_eval_params = NextEvalParams::new(parameters.time, 0.0);
						}
					},
					PatternEvalUpdate::Tracking { enabled } => {
						if tracking_data_rx.is_none() {
							eprintln!("error: tracking requested but no tracking data channel is connected (tracking was disabled)!");
						}
						enable_tracking = enabled;
					},

					PatternEvalUpdate::ParameterTime { time } => parameters.time = time,
        			PatternEvalUpdate::UserParameters { user_parameters } => parameters.user_parameters = user_parameters,
        			PatternEvalUpdate::GeoTransformMatrix { transform } => parameters.geometric_transform = transform,
				}
			},
			i if Some(i) == tracking_data_rx_idx => {
				tracking_data = oper.recv(tracking_data_rx.as_ref().unwrap())?;
			},
			_ => unreachable!(),
		};

	}
}