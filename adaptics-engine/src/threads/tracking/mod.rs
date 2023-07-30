use serde::{Serialize, Deserialize};

pub mod leapmotion;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackingFrame {
	pub hand: Option<pattern_evaluator::MAHCoordsConst>
}