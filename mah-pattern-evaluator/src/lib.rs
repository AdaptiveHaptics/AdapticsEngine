use std::collections::HashMap;
use std::cmp::Ordering;

dumping some gpt generated code for now, will redo

#[derive(Debug, Clone, Copy, PartialEq)]
struct MAHKeyframeStandard {
    time: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum MAHBrush {
    Circle,
    Square,
    Triangle,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MAHCoords {
    time: f64,
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MAHIntensity {
    time: f64,
    level: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MAHKeyframe {
    Coords(MAHCoords),
    Brush(MAHBrush),
    Intensity(MAHIntensity),
    Pause,
}

#[derive(Debug, Clone, PartialEq)]
struct MidAirHapticsAnimationFileFormat {
    keyframes: Vec<MAHKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PatternEvaluatorParameters {
    time: f64,
    user_parameters: HashMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HapeV2PrimitiveParams {
    A: f64,
    B: f64,
    a: f64,
    b: f64,
    d: f64,
    k: f64,
    max_t: f64,
    draw_frequency: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BrushEvalParams {
    primitive_type: MAHBrush,
    primitive: HapeV2PrimitiveParams,
    painter: (f64, f64, f64),
}

struct PatternEvaluator {
    mah_animation: MidAirHapticsAnimationFileFormat,
}

impl PatternEvaluator {
    fn new(mah_animation: MidAirHapticsAnimationFileFormat) -> Self {
        let mut mah_animation = mah_animation;
        mah_animation.keyframes.sort_by(|a, b| a.cmp(b));

        Self {
            mah_animation,
        }
    }

    fn get_kf_config_type(&self, t: f64, prev: bool) -> Option<(MAHKeyframe, bool)> {
        let mut kfc = None;
        let mut keyframes = self.mah_animation.keyframes.clone();
        if !prev {
            keyframes.reverse();
        }
        for kf in keyframes {
            match kf {
                MAHKeyframe::Coords(c) => {
                    if prev {
                        if c.time > t {
                            break;
                        }
                    } else