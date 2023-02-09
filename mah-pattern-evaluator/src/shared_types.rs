use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidAirHapticsAnimationFileFormat {
    #[serde(rename = "$DATA_FORMAT")]
    pub data_format: &'static str,
    #[serde(rename = "$REVISION")]
    pub revision: &'static str,

    pub name: String,

    pub keyframes: Vec<MAHKeyframe>,

    pub update_rate: f64,

    pub projection: Projection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    Plane,
    Palm,
}



/*****              MAH Keyframe primitives              *****/

/// Time in milliseconds
pub type MAHTime = f64;

/// x and y are used for the xy coordinate system in the 2d designer.
/// z is intended to be orthogonal to the phased array
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
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
pub enum MAHBrush {
    Circle { radius: f64 },
    Line { length: f64, thickness: f64, rotation: f64 },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
pub enum MAHIntensity {
    Constant { value: f64 },
    Random { min: f64, max: f64 },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "name", content = "params")]
pub enum MAHTransition {
    Linear {},
    Step {},
}




#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct CoordsWithTransition {
    pub coords: MAHCoords,
    pub transition: MAHTransition,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct BrushWithTransition {
    pub brush: MAHBrush,
    pub transition: MAHTransition,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct IntensityWithTransition {
    pub intensity: MAHIntensity,
    pub transition: MAHTransition,
}

/// standard keyframe with coords, brush, intensity, and transitions
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MAHKeyframeStandard {
    pub time: MAHTime,
    pub brush: Option<BrushWithTransition>,
    pub intensity: Option<IntensityWithTransition>,
    pub coords: CoordsWithTransition,
}

/// Holds the coordinates of the previous keyframe until elapsed.
/// can be used to animate the brush/intensity at a static location in the path,
/// or just to create a pause in the animation path.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MAHKeyframePause {
    pub time: MAHTime,
    pub brush: Option<BrushWithTransition>,
    pub intensity: Option<IntensityWithTransition>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MAHKeyframe {
    Standard(MAHKeyframeStandard),
    Pause(MAHKeyframePause),
}

impl MAHKeyframe {
    pub fn time(&self) -> &f64 {
        match self {
            MAHKeyframe::Standard(kf) => &kf.time,
            MAHKeyframe::Pause(kf) => &kf.time,
        }
    }
}
