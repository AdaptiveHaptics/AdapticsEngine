use std::time::{Duration, Instant};

use pattern_evaluator::BrushAtAnimLocalTime;

use crate::{threads::pattern::playback::PatternEvalCall, DEBUG_LOG_LAG_EVENTS};

pub const USE_THREAD_SLEEP: Option<u64> = Some(1000); // still busy wait for ~1000ms, to avoid thread sleeping for too long

pub fn start_mock_emitter(
	device_update_rate: u64,
	callback_rate: f64,
	patteval_call_tx: crossbeam_channel::Sender<PatternEvalCall>,
	patteval_return_rx: crossbeam_channel::Receiver<Vec<BrushAtAnimLocalTime>>,
	end_streaming_rx: crossbeam_channel::Receiver<()>,
) {
	// println!("setting thread priority max");
	// thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();


	let device_tick_dur = Duration::from_nanos(1_000_000_000/device_update_rate);
	let ecallback_tick_dur = Duration::from_secs_f64(1.0/callback_rate);
	let deadline_offset = ecallback_tick_dur * 1;
	let mut last_tick = Instant::now();

	assert!(device_tick_dur.as_secs_f64() > 0.0, "device_tick_dur must be > 0");
	loop {
		if end_streaming_rx.try_recv().is_ok() {
			break;
		}

		if let Some(bwt) = USE_THREAD_SLEEP { std::thread::sleep(last_tick + ecallback_tick_dur - Instant::now() - Duration::from_micros(bwt)); } // supports windows high resolution sleep since rust 1.75

		while last_tick + ecallback_tick_dur > Instant::now() {}

		let curr_time = Instant::now();
		let elapsed = curr_time - last_tick;
		if DEBUG_LOG_LAG_EVENTS && elapsed > ecallback_tick_dur + Duration::from_micros(100) { println!("[WARN] elapsed > ecallback_tick_dur: {:?} > {:?}", elapsed, ecallback_tick_dur); }
		last_tick = curr_time;

		let deadline_time = curr_time + deadline_offset;

		let mut time_arr_instants = Vec::with_capacity((device_update_rate as f64 / callback_rate) as usize + 2);
		let mut future_device_tick_instant = deadline_time;
		while future_device_tick_instant < (deadline_time + ecallback_tick_dur) {
			time_arr_instants.push(future_device_tick_instant);
			future_device_tick_instant += device_tick_dur;
		}

		if patteval_call_tx.send(PatternEvalCall::EvalBatch{ time_arr_instants }).is_ok() {
			patteval_return_rx.recv().unwrap();
		} else {
			// patt eval thread exited (or panicked),
			// end_streaming_rx will be called by main thread, could exit here anyway
			break; // not sure if I want to do this or just loop until end_streaming_rx
		}

		// if let Err(e) = patteval_return_rx.recv() {
		//     // pattern eval thread exited, so we should exit
		//     break;
		// }
		// println!("remaining time {:?}", deadline_time-Instant::now());

		// both are needed because durations are always positive and subtraction saturates
		let deadline_remaining = deadline_time - Instant::now();
		let deadline_missed_by = Instant::now() - deadline_time;
		if deadline_remaining.is_zero() {
			eprintln!("missed deadline by {:?}", deadline_missed_by);
		}
	}

	println!("mock streaming exiting...");
}