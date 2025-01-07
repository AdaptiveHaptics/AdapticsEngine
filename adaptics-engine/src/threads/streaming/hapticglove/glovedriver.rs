
use std::{cell::Cell, time::{Duration, Instant}};

use pattern_evaluator::MAHCoordsConst;
use serialport::{self, SerialPort};

use crate::DEBUG_LOG_SERIAL_RTT;

const NUM_DRIVERS: usize = 16;
const COBS_DELIM: u8 = 0x88; // using unlikely byte as delim
const HEADER_LEN: usize = 1; // 1 byte COBS overhead
const FOOTER_LEN: usize = 1; // 1 byte delim
const PACKET_LEN: usize = HEADER_LEN + NUM_DRIVERS + FOOTER_LEN;
const ACK_PACKET: &[u8] = b"OKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOK\r\n";

const MAX_DIST: f64 = 30.0; // mm, distance from LRA where amp is 0%

type LRALayout = [MAHCoordsConst; NUM_DRIVERS];
pub enum LRAPositions {
	PalmTopCenter,
	PalmTopLeft,
	PalmTopRight,

	PalmBottomCenter,
	PalmBottomLeft,
	PalmBottomRight,

	Wrist,

	Thumb,

	IndexFingerBase,
	IndexFingerTip,

	MiddleFingerBase,
	MiddleFingerTip,

	RingFingerBase,
	RingFingerTip,

	LittleFingerBase,
	LittleFingerTip,
}
impl LRAPositions {
	pub const fn get_coords(&self) -> MAHCoordsConst {
		match self {
			LRAPositions::PalmTopCenter => MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 },
			LRAPositions::PalmTopLeft => MAHCoordsConst { x: -26.0, y: 0.0, z: 0.0 },
			LRAPositions::PalmTopRight => MAHCoordsConst { x: 28.0, y: 0.0, z: 0.0 },

			LRAPositions::PalmBottomCenter => MAHCoordsConst { x: 0.0, y: -39.0, z: 0.0 },
			LRAPositions::PalmBottomLeft => MAHCoordsConst { x: -26.0, y: -38.0, z: 0.0 },
			LRAPositions::PalmBottomRight => MAHCoordsConst { x: 28.0, y: -37.0, z: 0.0 },

			LRAPositions::Wrist => MAHCoordsConst { x: -1.0, y: -74.0, z: 0.0 },

			LRAPositions::Thumb => MAHCoordsConst { x: 63.0, y: -7.0, z: 0.0 },

			LRAPositions::IndexFingerBase => MAHCoordsConst { x: 33.0, y: 36.0, z: 0.0 },
			LRAPositions::IndexFingerTip => MAHCoordsConst { x: 36.0, y: 76.0, z: 0.0 },

			LRAPositions::MiddleFingerBase => MAHCoordsConst { x: 9.0, y: 44.0, z: 0.0 },
			LRAPositions::MiddleFingerTip => MAHCoordsConst { x: 11.0, y: 84.0, z: 0.0 },

			LRAPositions::RingFingerBase => MAHCoordsConst { x: -14.0, y: 41.0, z: 0.0 },
			LRAPositions::RingFingerTip => MAHCoordsConst { x: -16.0, y: 76.0, z: 0.0 },

			LRAPositions::LittleFingerBase => MAHCoordsConst { x: -35.0, y: 36.0, z: 0.0 },
			LRAPositions::LittleFingerTip => MAHCoordsConst { x: -40.0, y: 58.0, z: 0.0 },
		}
	}
	const EMPTY_LAYOUT: LRALayout = [
		MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 },
		MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 },
		MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 },
		MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 }, MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 },
	];
	pub const fn pos_to_coords(v: &[LRAPositions; NUM_DRIVERS]) -> LRALayout {
		let mut out: LRALayout = Self::EMPTY_LAYOUT;
		let mut i = 0;
		while i < NUM_DRIVERS {
			out[i] = v[i].get_coords();
			i += 1;
		}
		out
	}
}
impl From<LRAPositions> for MAHCoordsConst {
	fn from(pos: LRAPositions) -> Self {
		pos.get_coords()
	}
}
pub const DEFAULT_LRA_LAYOUT: LRALayout = LRAPositions::pos_to_coords(&[ //left hand palm down
	//CN1
	LRAPositions::PalmTopCenter,
	LRAPositions::PalmTopLeft,
	LRAPositions::LittleFingerBase,
	LRAPositions::LittleFingerTip,

	//CN2 (mux flipped)
	LRAPositions::PalmBottomCenter,
	LRAPositions::PalmTopRight,
	LRAPositions::IndexFingerBase,
	LRAPositions::IndexFingerTip,

	//CN3
	LRAPositions::MiddleFingerBase,
	LRAPositions::MiddleFingerTip,
	LRAPositions::RingFingerTip,
	LRAPositions::RingFingerBase,

	//CN4 (mux flipped)
	LRAPositions::Thumb,
	LRAPositions::PalmBottomLeft,
	LRAPositions::Wrist,
	LRAPositions::PalmBottomRight,
]);

pub trait IoPort: std::io::Write + std::io::Read {
	fn clear_rx_buf(&mut self) -> std::io::Result<()>;
}
impl<T: AsRef<dyn SerialPort> + std::io::Write + std::io::Read> IoPort for T {
	fn clear_rx_buf(&mut self) -> std::io::Result<()> {
		Ok(self.as_ref().clear(serialport::ClearBuffer::Input)?)
	}
}

struct MockIO {
	write_time: Cell<Instant>,
	device_latency: Duration
}
impl MockIO {
	pub fn new() -> Self {
		MockIO { write_time: Cell::new(Instant::now()), device_latency: Duration::from_micros(100) }
	}
}
impl std::default::Default for MockIO {
	fn default() -> Self {
		Self::new()
	}
}
impl std::io::Write for &MockIO {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.write_time.set(Instant::now());
		Ok(buf.len())
	}
	fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
impl std::io::Write for MockIO {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { (&mut &*self).write(buf) }
	fn flush(&mut self) -> std::io::Result<()> { (&mut &*self).flush() }
}
impl std::io::Read for &MockIO {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let sleep_time = self.device_latency.saturating_sub(self.write_time.get().elapsed());
		std::thread::sleep(sleep_time);
		// put OK\n in buf
		if buf.len() < 3 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Buffer too small")); }
		buf[0] = 0x4F;
		buf[1] = 0x4B;
		buf[2] = 0x0A;
		Ok(3)
	}
}
impl std::io::Read for MockIO {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { (&mut &*self).read(buf) }
}
impl IoPort for MockIO {
	fn clear_rx_buf(&mut self) -> std::io::Result<()> { Ok(()) }
}

pub struct GloveDriver {
	io_port: Box<dyn IoPort>,
	tx_buf: Vec<u8>,
	rx_buf: Vec<u8>,
	lra_layout: LRALayout,
}
pub struct DriverAmplitudes([u8; NUM_DRIVERS]);
impl DriverAmplitudes {
	pub fn get_slice(&self) -> &[u8] {
		&self.0
	}
}

impl GloveDriver {
	pub fn get_possible_serial_ports() -> std::io::Result<Vec<serialport::SerialPortInfo>> {
		Ok(serialport::available_ports()?)
	}

	pub fn new(io_port: Box<dyn IoPort>, lra_layout: LRALayout) -> Self {
		GloveDriver {
			io_port,
			tx_buf: Vec::with_capacity(PACKET_LEN),
			rx_buf: vec![0; 256],
			lra_layout
		}
	}
	pub fn new_mock(lra_layout: LRALayout) -> Self {
		GloveDriver::new(Box::new(MockIO::default()), lra_layout)
	}
	pub fn new_for_serial_port(port: &str, lra_layout: LRALayout) -> std::io::Result<Self> {
		let s_port = serialport::new(port, 115_200)
			.timeout(Duration::from_millis(100))
			.baud_rate(921_600)
			.open()?;
		let io_port: Box<dyn IoPort> = Box::new(s_port);
		Ok(GloveDriver::new(io_port, lra_layout))
	}
	pub fn new_with_auto_serial_port(lra_layout: LRALayout) -> std::io::Result<Self> {
		let ports = serialport::available_ports()?;
		match ports.iter().find(|p| matches!(p.port_type, serialport::SerialPortType::UsbPort(_))) {
			Some(p) => {
				println!("INFO: Auto-detected serial port: {p:?}");
				GloveDriver::new_for_serial_port(&p.port_name, lra_layout)
			},
			None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No serial ports found"))
		}
	}


	pub fn set_driver_amplitudes(&mut self, driver_amplitudes: &DriverAmplitudes) -> std::io::Result<()> {
		self.tx_buf.clear();
		self.tx_buf.push(COBS_DELIM);
		self.tx_buf.extend_from_slice(driver_amplitudes.get_slice());
		self.tx_buf.push(COBS_DELIM);

		// reverse iterate
		let mut last_delim_dist = 1;
		for i in (0..self.tx_buf.len()-1).rev() {
			let sym = self.tx_buf[i];
			if sym == COBS_DELIM {
				self.tx_buf[i] = u8::try_from(last_delim_dist).or(Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Message too long for COBS")))?;
				assert!(self.tx_buf[i] != COBS_DELIM, "COBS delim in message. Basically, message length cannot be longer than COBS_DELIM ({COBS_DELIM}). This should never occur in set_driver_amplitudes");
				last_delim_dist = 1;
			} else {
				last_delim_dist += 1;
			}
		}

		let mut len_read = 0;
		self.io_port.clear_rx_buf()?;

		let begin_write = Instant::now();
		self.io_port.write_all(&self.tx_buf)?;
		// println!("DEBUG: Sent packet: {:?}", &self.tx_buf);

		while !self.rx_buf[0..len_read].contains(&b'\n') { // read until newline
			match self.io_port.read(&mut self.rx_buf[len_read..]) {
				Ok(n) => len_read += n,
				Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => { std::thread::sleep(Duration::from_millis(1)); },
				Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => { eprintln!("WARN: Timed out reading from haptic glove device"); continue; },
				Err(e) => return Err(e),
			}
		}
		if DEBUG_LOG_SERIAL_RTT { println!("DEBUG: RTT: {:?}", begin_write.elapsed()); }

		if !self.rx_buf.starts_with(ACK_PACKET) {
			let response = std::str::from_utf8(&self.rx_buf[0..len_read]).or(Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Response not UTF-8")))?;
			return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("unexpected device response: {response:?}")));
		}

		Ok(())
	}

	pub fn calc_driver_amplitudes_from_brush_evals(&self, brush_evals: &[pattern_evaluator::BrushAtAnimLocalTime]) -> DriverAmplitudes {
		let mut driver_amplitudes = [0u8; NUM_DRIVERS];
		for be in brush_evals {
			let coords = &be.ul_control_point.coords;
			let intensity = be.ul_control_point.intensity;

			for (i, lra) in self.lra_layout.iter().enumerate() {
				let dist = ((coords.x - lra.x).powi(2) + (coords.y - lra.y).powi(2)).sqrt(); // ignore z coord
				let x = dist / MAX_DIST; // at 30 mm func evals to 0. Ease in-out, so 99% at 5mm, 90% at 10mm, 10% at 22mm, 1% at ~25mm
				let y = x.mul_add((-x * x) * x, 1.0).powi(7); // ease in-out 4th, 7th power: f[dist, MAX_DIST, 4] where f[x_, r_, s_] := (1 - (x/r)^s)^7
				let driver_amp = y.clamp(0.0, 1.0) * intensity;

				#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
				{ driver_amplitudes[i] = driver_amplitudes[i].max((driver_amp.clamp(0.0, 1.0) * 255.0) as u8); }
			}
		}
		DriverAmplitudes(driver_amplitudes)
	}

	pub fn apply_batch(&mut self, brush_evals: &[pattern_evaluator::BrushAtAnimLocalTime]) -> std::io::Result<()> {
		let driver_amplitudes = self.calc_driver_amplitudes_from_brush_evals(brush_evals);
		self.set_driver_amplitudes(&driver_amplitudes)
	}

	pub fn stop_all(&mut self) -> std::io::Result<()> {
		self.set_driver_amplitudes(&DriverAmplitudes([0; NUM_DRIVERS]))
	}
}


#[cfg(test)]
mod tests {
	use pattern_evaluator::PatternEvaluator;

use super::*;
	use std::io::Write;

	struct IBColor {
		r: u8,
		g: u8,
		b: u8,
	}
	impl IBColor {
		pub fn new_hex(hex: u32) -> Self {
			IBColor { r: ((hex >> 16) & 0xFF) as u8, g: ((hex >> 8) & 0xFF) as u8, b: (hex & 0xFF) as u8 }
		}
		#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
		pub fn scale(&self, s: f64) -> Self {
			IBColor { r: (f64::from(self.r) * s) as u8, g: (f64::from(self.g) * s) as u8, b: (f64::from(self.b) * s) as u8 }
		}
	}
	struct ImageBuffer {
		size: (usize, usize),
		data: Vec<u8>,
	}
	#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
	impl ImageBuffer {
		pub fn new(size: (usize, usize)) -> Self {
			ImageBuffer { size, data: vec![0; size.0 * size.1 * 3] }
		}

		pub fn clear(&mut self) {
			self.data.iter_mut().for_each(|x| *x = 0);
		}

		fn xy_to_idx(&self, x: usize, y: usize) -> usize {
			(y * self.size.0 + x) * 3
		}

		#[allow(dead_code)]
		fn set_pixel(&mut self, x: usize, y: usize, c: &IBColor) {
			let idx = self.xy_to_idx(x, y);
			self.data[idx  ] = c.r;
			self.data[idx+1] = c.g;
			self.data[idx+2] = c.b;
		}
		fn add_pixel(&mut self, x: usize, y: usize, c: &IBColor) {
			let idx = self.xy_to_idx(x, y);
			self.data[idx  ] = self.data[idx  ].saturating_add(c.r);
			self.data[idx+1] = self.data[idx+1].saturating_add(c.g);
			self.data[idx+2] = self.data[idx+2].saturating_add(c.b);
		}

		fn render_brush_aa(&mut self, x: f64, y: f64, radius: f64, c: &IBColor) {
			let x0 = (x - radius).floor().max(0.0).min(self.size.0 as f64 - 1.0) as usize;
			let x1 = (x + radius).floor().max(0.0).min(self.size.0 as f64 - 1.0) as usize;
			let y0 = (y - radius).floor().max(0.0).min(self.size.1 as f64 - 1.0) as usize;
			let y1 = (y + radius).floor().max(0.0).min(self.size.1 as f64 - 1.0) as usize;

			for yp in y0..=y1 {
				for xp in x0..=x1 {
					let dist = ((xp as f64 - x).powi(2) + (yp as f64 - y).powi(2)).sqrt();
					let alpha = 1.0 - dist / radius;
					if alpha > 0.0 {
						self.add_pixel(xp, yp, &c.scale(alpha));
					}
				}
			}
		}

		pub fn render_brush_aa_mahcoords(&mut self, coords: &MAHCoordsConst, radius: f64, c: &IBColor) {
			let x = (coords.x + 100.0) / 220.0 * self.size.0 as f64;
			let y = (1.0 - (coords.y + 100.0) / 220.0) * self.size.1 as f64;
			self.render_brush_aa(x, y, radius, c);
		}

	}

	#[allow(clippy::unreadable_literal, clippy::cast_precision_loss)]
	#[test]
	#[ignore = "debug, creates video output"]
	fn debug_test_calc_driver_amplitudes_from_brush_evals() {
		let gd = GloveDriver::new_mock(DEFAULT_LRA_LAYOUT);

		let image_size = 440;
		let frame_rate = 100; // 100hz
		let sample_rate = 10000; // 10khz
		let num_frames = 550;

		let mut image_buffer = ImageBuffer::new((image_size, image_size));

		let mut ffmpeg = std::process::Command::new("ffmpeg")
			.args([
				"-f", "rawvideo",
				"-pixel_format", "rgb24",
				"-video_size", &format!("{image_size}x{image_size}"),
				"-framerate", &format!("{frame_rate}"),
				"-i", "pipe:0", // Read from stdin
				"-c:v", "libx264",
				"hapticglove-driver.debug-test-output.mp4",
				"-y",
			])
			.stdin(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped())
			.spawn()
			.expect("Failed to start ffmpeg");
		let ffmpeg_stdin = ffmpeg.stdin.as_mut().unwrap();


		image_buffer.clear();
		for i in 0..NUM_DRIVERS {
			image_buffer.render_brush_aa_mahcoords(&gd.lra_layout[i], 10.0, &IBColor::new_hex(0x808080));
		}
		ffmpeg_stdin.write_all(&image_buffer.data).unwrap();


		let pattern_evaluator = PatternEvaluator::new_from_json_string(r#"{"$DATA_FORMAT":"MidAirHapticsAnimationFileFormat","$REVISION":"0.1.0-alpha.3","name":"untitled","keyframes":[{"time":0,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":99},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":-55,"y":-60,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]},{"time":1000,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":-55,"y":75,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]},{"time":2000,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":-5,"y":-60,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]},{"time":3000,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":-5,"y":75,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]},{"time":4000,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":45,"y":-60,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]},{"time":5000,"type":"standard","brush":{"brush":{"name":"circle","params":{"radius":{"type":"f64","value":10},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":1}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":45,"y":75,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]}],"pattern_transform":{"geometric_transforms":{"translate":{"x":{"type":"f64","value":0},"y":{"type":"f64","value":0},"z":{"type":"f64","value":200}},"rotation":{"type":"f64","value":0},"scale":{"x":{"type":"f64","value":1},"y":{"type":"f64","value":1},"z":{"type":"f64","value":1}}},"intensity_factor":{"type":"f64","value":1},"playback_speed":{"type":"f64","value":1}},"user_parameter_definitions":{}}"#).unwrap();
		// let pattern_evaluator = PatternEvaluator::new_from_json_string(r#"{"$DATA_FORMAT":"MidAirHapticsAnimationFileFormat","$REVISION":"0.1.0-alpha.3","name":"untitled","keyframes":[{"time":0,"type":"standard","brush":{"brush":{"name":"line","params":{"length":{"type":"f64","value":30},"thickness":{"type":"f64","value":1},"rotation":{"type":"f64","value":0},"am_freq":{"type":"f64","value":0},"stm_freq":{"type":"f64","value":100}}},"transition":{"name":"linear","params":{}}},"intensity":{"intensity":{"name":"constant","params":{"value":{"type":"f64","value":0.3}}},"transition":{"name":"linear","params":{}}},"coords":{"coords":{"x":0,"y":-40,"z":0},"transition":{"name":"linear","params":{}}},"cjumps":[]}],"pattern_transform":{"geometric_transforms":{"translate":{"x":{"type":"f64","value":0},"y":{"type":"f64","value":0},"z":{"type":"f64","value":200}},"rotation":{"type":"f64","value":0},"scale":{"x":{"type":"f64","value":1},"y":{"type":"f64","value":1},"z":{"type":"f64","value":1}}},"intensity_factor":{"type":"f64","value":1},"playback_speed":{"type":"f64","value":1}},"user_parameter_definitions":{}}"#).unwrap();

		let mut next_eval_params = Default::default();
		let mut params: pattern_evaluator::PatternEvaluatorParameters = Default::default();

		let samples_per_frame = sample_rate / frame_rate; assert!(sample_rate % frame_rate == 0, "sample_rate must be divisible by frame_rate");
		for i in 0..num_frames {
			let mut brush_evals = Vec::with_capacity(samples_per_frame);
			for o in 0..samples_per_frame {
				let t = f64::from(i) * (1000.0 / frame_rate as f64) + o as f64 * (1000.0 / sample_rate as f64);
				params.time = t;
				let eval = pattern_evaluator.eval_brush_at_anim_local_time(&params, &next_eval_params);
				next_eval_params = eval.next_eval_params.clone();
				brush_evals.push(eval);
			}

			let driver_amplitudes = gd.calc_driver_amplitudes_from_brush_evals(&brush_evals);
			image_buffer.clear();

			for be in &brush_evals {
				image_buffer.render_brush_aa_mahcoords(&be.ul_control_point.coords, 10.0, &IBColor::new_hex(0x007eee).scale(0.1));
			}

			for i in 0..NUM_DRIVERS {
				image_buffer.render_brush_aa_mahcoords(&gd.lra_layout[i], 10.0, &IBColor::new_hex(0xFFFFFF).scale(f64::from(driver_amplitudes.0[i]) / 255.0 * 0.8 + 0.2));
			}

			ffmpeg_stdin.write_all(&image_buffer.data).unwrap();
		}

		ffmpeg_stdin.flush().unwrap();
		let ffmpeg_output = ffmpeg.wait_with_output().unwrap();
		assert!(ffmpeg_output.status.success(), "ffmpeg failed: {:?}", std::str::from_utf8(&ffmpeg_output.stderr).unwrap());
	}

}
