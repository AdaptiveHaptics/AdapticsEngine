mod glovedriver;
use std::time::{Duration, Instant};

use pattern_evaluator::BrushAtAnimLocalTime;

use crate::{threads::pattern::playback::PatternEvalCall, util::AdapticsError, DEBUG_LOG_LAG_EVENTS};

// pub const USE_THREAD_SLEEP: Option<u64> = Some(1000); // if native sleep { still busy wait for ~1000ms, to avoid thread sleeping for too long }
pub const USE_THREAD_SLEEP: Option<u64> = Some(500); // spin_sleeper still needs some buffer time (it shouldnt need any), but less than native. idk if it overtrusts the os sleep, or its some other slowdown?

pub const SAMPLE_RATE: u64 = 1000; // 1000hz
pub const CALLBACK_RATE: f64 = 100.0; // 100hz
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub const SAMPLES_PER_CALLBACK: usize = (SAMPLE_RATE as f64 / CALLBACK_RATE) as usize;

pub enum DeviceType {
	SerialPort(String),
	Mock,
	Auto,
}

#[must_use]
pub fn get_possible_serial_ports() -> Vec<serialport::SerialPortInfo> {
	glovedriver::GloveDriver::get_possible_serial_ports()
}

pub fn start_streaming_emitter(
	device_type: &DeviceType,
	patteval_call_tx: &crossbeam_channel::Sender<PatternEvalCall>,
	patteval_return_rx: &crossbeam_channel::Receiver<Vec<BrushAtAnimLocalTime>>,
	end_streaming_rx: &crossbeam_channel::Receiver<()>,
) -> Result<(), AdapticsError> {

	let mut gd = match device_type {
		DeviceType::SerialPort(port) => glovedriver::GloveDriver::new_for_serial_port(port, glovedriver::DEFAULT_LRA_LAYOUT)?,
		DeviceType::Mock => glovedriver::GloveDriver::new_mock(glovedriver::DEFAULT_LRA_LAYOUT),
		DeviceType::Auto => glovedriver::GloveDriver::new_with_auto_serial_port(glovedriver::DEFAULT_LRA_LAYOUT)?,
	};

	let device_tick_dur = Duration::from_nanos(1_000_000_000/SAMPLE_RATE);
	let ecallback_tick_dur = Duration::from_secs_f64(1.0/CALLBACK_RATE);
	let deadline_offset = ecallback_tick_dur * 1;
	let mut last_tick = Instant::now();

	let spin_sleeper = spin_sleep::SpinSleeper::default();

	assert!(device_tick_dur.as_secs_f64() > 0.0, "device_tick_dur must be > 0");
	loop {
		if end_streaming_rx.try_recv().is_ok() {
			break;
		}

		let next_tick_at = last_tick + ecallback_tick_dur;
		// if let Some(bwt) = USE_THREAD_SLEEP { std::thread::sleep(next_tick_at - Instant::now() - Duration::from_micros(bwt)); } // supports windows high resolution sleep since rust 1.75
		if let Some(bwt) = USE_THREAD_SLEEP { spin_sleeper.sleep(next_tick_at.saturating_duration_since(Instant::now()).saturating_sub(Duration::from_micros(bwt))); } // shouldnt need bwt but it does
		while next_tick_at > Instant::now() {} // busy wait remaining time
		// spin_sleeper.sleep(next_tick_at - Instant::now()); // not accurate enough by itself on windows


		let curr_time = Instant::now();
		let elapsed = curr_time - last_tick;
		if DEBUG_LOG_LAG_EVENTS && elapsed > ecallback_tick_dur + Duration::from_micros(100) { println!("[WARN] elapsed > ecallback_tick_dur: {elapsed:?} > {ecallback_tick_dur:?}"); }
		last_tick = curr_time;

		let deadline_time = curr_time + deadline_offset;

		let mut time_arr_instants = Vec::with_capacity(SAMPLES_PER_CALLBACK + 2);
		let mut future_device_tick_instant = deadline_time;
		while future_device_tick_instant < (deadline_time + ecallback_tick_dur) {
			time_arr_instants.push(future_device_tick_instant);
			future_device_tick_instant += device_tick_dur;
		}

		if patteval_call_tx.send(PatternEvalCall::EvalBatch{ time_arr_instants }).is_ok() {
			let eval_arr = patteval_return_rx.recv()?;
			gd.apply_batch(&eval_arr)?;
		} else {
			// patt eval thread exited (or panicked),
			// end_streaming_rx will be called by main thread, could exit here anyway
			break; // not sure if I want to do this or just loop until end_streaming_rx
		}

		// both are needed because durations are always positive and subtraction saturates
		let deadline_remaining = deadline_time - Instant::now();
		let deadline_missed_by = deadline_time.elapsed();
		if deadline_remaining.is_zero() {
			eprintln!("missed deadline by {deadline_missed_by:?}");
		}
	}

	println!("mock streaming exiting...");
	Ok(())
}