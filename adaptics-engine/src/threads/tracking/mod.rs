#![allow(clippy::module_name_repetitions)]
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

pub mod leapmotion;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrackingFrame {
	pub hand: Option<TrackingFrameHand>
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrackingFrameHand {
	pub chirality: TrackingFrameHandChirality,
	pub palm: TrackingFramePalm,
	pub digits: Box<[TrackingFrameDigit; 5]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrackingFramePalm {
	/// The center position of the palm
	pub position: pattern_evaluator::MAHCoordsConst,
	/// The estimated width of the palm when the hand is in a flat position.
	pub width: f64,
	/// If your hand is flat, this vector will point downward, or "out" of the front surface of your palm.
	pub normal: pattern_evaluator::MAHCoordsConst,
	/// The unit direction vector pointing from the palm position toward the fingers.
	pub direction: pattern_evaluator::MAHCoordsConst,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum TrackingFrameHandChirality {
	Right = 0,
	Left = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrackingFrameDigit {
	pub bones: [TrackingFrameBone; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrackingFrameBone {
	pub start: pattern_evaluator::MAHCoordsConst,
	pub end: pattern_evaluator::MAHCoordsConst,
	/// The average width of the flesh around the bone in millimeters.
	pub width: f64,
}