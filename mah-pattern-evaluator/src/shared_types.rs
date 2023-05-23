use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
// use ts_rs::TS;


// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum DataFormatRevision {
    #[serde(rename = "0.0.8-alpha.1")] CurrentRevision
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum MidAirHapticsAnimationFileFormatDataFormatName {
    #[serde(rename = "MidAirHapticsAnimationFileFormat")] DataFormat
}


#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// #[ts(export)]
pub struct MidAirHapticsAnimationFileFormat {
    #[serde(rename = "$DATA_FORMAT")]
    pub data_format: MidAirHapticsAnimationFileFormatDataFormatName,
    #[serde(rename = "$REVISION")]
    pub revision: DataFormatRevision,

    pub name: String,

    pub keyframes: Vec<MAHKeyframe>,

    pub pattern_transform: PatternTransformation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeometricTransformMatrix([[f64; 4]; 4]);
impl Default for GeometricTransformMatrix {
    fn default() -> Self {
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}
impl std::ops::Index<usize> for GeometricTransformMatrix {
    type Output = [f64; 4];
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeometricTransformsSimple {
    pub translate: MAHCoords,
    /// in degrees
    pub rotation: f64,
    pub scale: MAHScaleTuple,
}
impl Default for GeometricTransformsSimple {
    fn default() -> Self {
        Self {
            translate: MAHCoords { x: 0.0, y: 0.0, z: 0.0 },
            rotation: 0.0,
            scale: MAHScaleTuple { x: 1.0, y: 1.0, z: 1.0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatternTransformation {
    pub geometric_transforms: GeometricTransformsSimple,
    pub intensity_factor: MAHPercentage,
    pub playback_speed: MAHPercentage,
}
impl Default for PatternTransformation {
    fn default() -> Self {
        Self {
            geometric_transforms: Default::default(),
            intensity_factor: 1.0.into(),
            playback_speed: 1.0.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MAHScaleTuple {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 100(%) becomes 1.0
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct MAHPercentage(f64);
impl From<f64> for MAHPercentage {
    fn from(f: f64) -> Self {
        Self(f * 100.0)
    }
}
impl Into<f64> for MAHPercentage {
    fn into(self) -> f64 {
        self.0 / 100.0
    }
}

/*****              MAH Keyframe primitives              *****/

/// Time in milliseconds
pub type MAHTime = f64;

/// x and y are used for the xy coordinate system in the 2d designer.
/// z is intended to be orthogonal to the phased array
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
// #[ts(export)]
pub struct MAHCoords {
    /// in millimeters, [-100, 100]
    pub x: f64,
    /// in millimeters, [-100, 100]
    pub y: f64,
    /// in millimeters, [0, 100]
    pub z: f64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
#[serde(rename_all = "snake_case")]
// #[ts(export)]
pub enum MAHBrush {
    Circle {
        /// Millimeters
        radius: f64,
        /// AM frequency in HZ
        am_freq: f64
    },
    Line {
        /// Millimeters
        length: f64,
        thickness: f64,
        /// Degrees
        rotation: f64,
        /// AM frequency in HZ
        am_freq: f64
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
#[serde(rename_all = "snake_case")]
// #[ts(export)]
pub enum MAHIntensity {
    Constant { value: f64 },
    Random { min: f64, max: f64 },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
#[serde(rename_all = "snake_case")]
// #[ts(export)]
pub enum MAHTransition {
    Linear {},
    Step {},
}




#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[ts(export)]
pub struct CoordsWithTransition {
    pub coords: MAHCoords,
    pub transition: MAHTransition,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[ts(export)]
pub struct BrushWithTransition {
    pub brush: MAHBrush,
    pub transition: MAHTransition,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[ts(export)]
pub struct IntensityWithTransition {
    pub intensity: MAHIntensity,
    pub transition: MAHTransition,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MAHCondition {
    pub parameter: String,
    pub value: f64,
    pub operator: MAHConditionalOperator,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
#[serde(rename_all = "snake_case")]
pub enum MAHConditionalOperator {
    Lt {},
    LtEq {},
    Gt {},
    GtEq {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ConditionalJump {
    pub condition: MAHCondition,
    pub jump_to: MAHTime,
}

/// standard keyframe with coords, brush, intensity, and transitions
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[ts(export)]
pub struct MAHKeyframeStandard {
    pub time: MAHTime,
    pub brush: Option<BrushWithTransition>,
    pub intensity: Option<IntensityWithTransition>,
    pub coords: CoordsWithTransition,
    pub cjumps: Vec<ConditionalJump>,
}

/// Holds the path coordinates of the previous keyframe until elapsed.
/// can be used to animate the brush/intensity at a static location in the path,
/// or just to create a pause in the animation path.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
// #[ts(export)]
pub struct MAHKeyframePause {
    pub time: MAHTime,
    pub brush: Option<BrushWithTransition>,
    pub intensity: Option<IntensityWithTransition>,
    pub cjumps: Vec<ConditionalJump>,
}

/// Stops the pattern and pauses the playback device
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MAHKeyframeStop {
    pub time: MAHTime,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
// #[ts(export)]
pub enum MAHKeyframe {
    Standard(MAHKeyframeStandard),
    Pause(MAHKeyframePause),
    Stop(MAHKeyframeStop),
}