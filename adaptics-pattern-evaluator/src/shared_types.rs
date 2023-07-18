

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
// use ts_rs::TS;


// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen::prelude::*;

#[allow(deprecated)]
mod stopdeprecatedwarning {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    #[non_exhaustive]
    pub enum DataFormatRevision {
        #[serde(rename = "0.1.0-alpha.1")] CurrentRevision,

        #[deprecated]
        #[schemars(skip)]
        #[serde(skip_serializing)]
        #[serde(rename = "0.0.10-alpha.1")]
        BackwardsCompatibleRevision,
    }
}
pub use stopdeprecatedwarning::*;

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

    pub user_parameter_definitions: UserParameterDefinitions,
}

pub type UserParameterDefinitions = HashMap<String, MAHUserParameterDefinition>;
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MAHUserParameterDefinition {
    pub default: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: f64,
}



#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ATFormula {
    Constant(f64),
    Parameter(String),
    Add(Box<ATFormula>, Box<ATFormula>),
    Subtract(Box<ATFormula>, Box<ATFormula>),
    Multiply(Box<ATFormula>, Box<ATFormula>),
    Divide(Box<ATFormula>, Box<ATFormula>),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "snake_case")]
pub enum MAHDynamicF64 {
    /// Specify a parameter instead of a constant value
    Dynamic(String),
    /// Normal constant value
    F64(f64),
    /// Formula
    Formula(ATFormula)
}
impl From<f64> for MAHDynamicF64 {
    fn from(f: f64) -> Self {
        Self::F64(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MAHCoordsDynamic {
    pub x: MAHDynamicF64,
    pub y: MAHDynamicF64,
    pub z: MAHDynamicF64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeometricTransformMatrix(pub [[f64; 4]; 4]);
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
    pub translate: MAHCoordsDynamic,
    /// in degrees
    pub rotation: MAHDynamicF64,
    pub scale: MAHScaleTuple,
}
impl Default for GeometricTransformsSimple {
    fn default() -> Self {
        Self {
            translate: MAHCoordsDynamic { x: 0.0.into(), y: 0.0.into(), z: 200.0.into() }, // 200mm (~8") is the default distance above the array (playback at 100mm feels less intense)
            rotation: 0.0.into(),
            scale: MAHScaleTuple { x: 1.0.into(), y: 1.0.into(), z: 1.0.into() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatternTransformation {
    pub geometric_transforms: GeometricTransformsSimple,
    pub intensity_factor: MAHDynamicF64,
    pub playback_speed: MAHDynamicF64,
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
    pub x: MAHDynamicF64,
    pub y: MAHDynamicF64,
    pub z: MAHDynamicF64,
}

/*****              MAH Keyframe primitives              *****/

/// Time in milliseconds
pub type MAHTime = f64;

/// x and y are used for the xy coordinate system in the 2d designer.
/// z is intended to be orthogonal to the phased array
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
// #[ts(export)]
pub struct MAHCoordsConst {
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
        radius: MAHDynamicF64,
        /// AM frequency in HZ
        am_freq: MAHDynamicF64
    },
    Line {
        /// Millimeters
        length: MAHDynamicF64,
        thickness: MAHDynamicF64,
        /// Degrees
        rotation: MAHDynamicF64,
        /// AM frequency in HZ
        am_freq: MAHDynamicF64
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
#[serde(rename_all = "snake_case")]
// #[ts(export)]
pub enum MAHIntensity {
    Constant { value: MAHDynamicF64 },
    Random { min: MAHDynamicF64, max: MAHDynamicF64 },
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
    pub coords: MAHCoordsConst,
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