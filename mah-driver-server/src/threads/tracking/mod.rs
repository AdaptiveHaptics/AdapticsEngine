pub mod leapmotion;

pub struct TrackingFrame {
	pub hand: Option<pattern_evaluator::MAHCoordsConst>
}