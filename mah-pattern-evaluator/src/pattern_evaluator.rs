mod shared_types;
use std::{collections::HashMap, mem::Discriminant};

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
pub use shared_types::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct PatternEvaluator {
    mah_animation: MidAirHapticsAnimationFileFormat,
}


#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatternEvaluatorParameters {
    pub time: f64,
    pub user_parameters: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct HapeV2Coords {
    x: f64,
    y: f64,
    z: f64,
}


impl PatternEvaluator {
    pub fn new(mah_animation: MidAirHapticsAnimationFileFormat) -> Self {
        let mut mah_animation = mah_animation;
        mah_animation.keyframes.sort_by(|a, b| a.time().total_cmp(b.time()));

        Self {
            mah_animation,
        }
    }

    pub fn new_from_json_string(mah_animation_json: &str) -> Self {
        let mut mah_animation: MidAirHapticsAnimationFileFormat = serde_json::from_str(mah_animation_json).unwrap();
        mah_animation.keyframes.sort_by(|a, b| a.time().total_cmp(b.time()));

        Self { mah_animation }
    }

    fn get_kf_config_type(&self, t: f64, prev: bool) -> MAHKeyframeConfig {
        let mut kfc = MAHKeyframeConfig::default();
        macro_rules! update_kfc {
            ($kf:ident, $prop:ident ?) => {
                kfc.$prop = $kf.$prop.as_ref().map(|b| PrimitiveWithTransitionAtTime {
                    time: $kf.time,
                    pwt: b,
                }).or(kfc.$prop);
            };
            ($kf:ident, $prop:ident !) => {
                kfc.$prop = Some(PrimitiveWithTransitionAtTime {
                    time: $kf.time,
                    pwt: &$kf.$prop,
                });
            };
        }
        let kf_iter: Vec<_> = if prev { self.mah_animation.keyframes.iter().collect() } else { self.mah_animation.keyframes.iter().rev().collect() };
        for kf in kf_iter {
            if prev { if kf.time() > &t { break; } }
			else { if kf.time() <= &t { break; } }
            match kf {
                MAHKeyframe::Standard(kf) => {
                    update_kfc!(kf, coords !);
                    update_kfc!(kf, brush ?);
                    update_kfc!(kf, intensity ?);
                },
                MAHKeyframe::Pause(kf) => {
                    kfc.coords = kfc.coords.map(|mut c| { c.time = kf.time; c });
                    update_kfc!(kf, brush ?);
                    update_kfc!(kf, intensity ?);
                },
                MAHKeyframe::Stop(_) => {}, // do nothing

            }
            kfc.keyframe = Some(kf.clone());
        }
        kfc
    }
    fn get_prev_kf_config(&self, t: f64) -> MAHKeyframeConfig {
        self.get_kf_config_type(t, true)
    }
    fn get_next_kf_config(&self, t: f64) -> MAHKeyframeConfig {
        self.get_kf_config_type(t, false)
    }

    /// returns (pf, nf) where pf is the factor for the previous keyframe and nf is the factor for the next keyframe
    fn perform_transition_interp(pattern_time: MAHTime, prev_time: f64, next_time: f64, transition: &MAHTransition) -> (f64, f64) {
        let dt = (pattern_time - prev_time) / (next_time - prev_time);
        match transition {
            MAHTransition::Linear {} => (1.0 - dt, dt),
            MAHTransition::Step {} => {
                if dt < 1.0 {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                }
            }
        }
    }

    fn eval_intensity(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> f64 {
        let prev_intensity = prev_kfc.intensity.as_ref();
        let next_intensity = next_kfc.intensity.as_ref();

        fn get_intensity_value(intensity: &MAHIntensity) -> f64 {
            match &intensity {
                MAHIntensity::Constant { value } => *value,
                MAHIntensity::Random { min, max } => rand::random::<f64>() * (max - min) + min,
            }
        }

        if let (Some(prev_intensity), Some(next_intensity)) = (prev_intensity, next_intensity) {
            let piv = get_intensity_value(&prev_intensity.pwt.intensity);
            let niv = get_intensity_value(&next_intensity.pwt.intensity);
            let (pf, nf) = Self::perform_transition_interp(pattern_time, prev_intensity.time, next_intensity.time, &prev_intensity.pwt.transition);
            return pf * piv + nf * niv;
        } else if let Some(prev_intensity) = prev_intensity {
            return get_intensity_value(&prev_intensity.pwt.intensity);
        } else {
            return 1.0;
        }
    }

    fn eval_coords(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> MAHCoords {
        let prev_coords_att = prev_kfc.coords.as_ref();
        let next_coords_att = next_kfc.coords.as_ref();
        let next_keyframe = next_kfc.keyframe.as_ref();
        if let (Some(prev_coords_att), Some(next_coords_att), Some(next_keyframe)) = (prev_coords_att, next_coords_att, next_keyframe) {
            return match next_keyframe {
                MAHKeyframe::Stop(_) | MAHKeyframe::Pause(_) => { prev_coords_att.pwt.coords.clone() },
                MAHKeyframe::Standard(_) => {
                    let (pf, nf) = Self::perform_transition_interp(pattern_time, prev_coords_att.time, next_coords_att.time, &prev_coords_att.pwt.transition);
                    return &prev_coords_att.pwt.coords * pf + &next_coords_att.pwt.coords * nf;
                }
            };
        } else if let Some(prev_coords_att) = prev_coords_att {
            return prev_coords_att.pwt.coords.clone();
        } else {
            return MAHCoords { x: 0.0, y: 0.0, z: 0.0 };
        }
    }

    fn unit_convert_dist_to_hapev2(mahunit: &f64) -> f64 {
        mahunit / 1000.0
    }

    fn unit_convert_rot_to_hapev2(mahunit: &f64) -> f64 {
        mahunit * (std::f64::consts::PI / 180.0)
    }

    fn coords_convert_to_hapev2(coords: &MAHCoords) -> HapeV2Coords {
        HapeV2Coords {
            x: Self::unit_convert_dist_to_hapev2(&coords.x),
            y: Self::unit_convert_dist_to_hapev2(&coords.y),
            z: Self::unit_convert_dist_to_hapev2(&coords.z),
        }
    }

    fn get_hapev2_primitive_params_for_brush(brush: &MAHBrush) -> HapeV2PrimitiveParams {
        match brush {
            MAHBrush::Circle { .. } => HapeV2PrimitiveParams {
                A: 1.0,
                B: 1.0,
                a: 1.0,
                b: 1.0,
                d: std::f64::consts::PI / 2.0,
                k: 0.0,
                max_t: 2.0 * std::f64::consts::PI,
                draw_frequency: 100.0,
            },
            MAHBrush::Line { .. } => HapeV2PrimitiveParams {
                A: 1.0,
                B: 0.0,
                a: 1.0,
                b: 1.0,
                d: std::f64::consts::PI / 2.0,
                k: 0.0,
                max_t: 2.0 * std::f64::consts::PI,
                draw_frequency: 100.0,
            },
        }
    }

    fn eval_brush_hapev2(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> BrushEvalParams {
        fn eval_mahbrush(brush: &MAHBrush) -> BrushEvalParams {
            let primitive_params = PatternEvaluator::get_hapev2_primitive_params_for_brush(brush);
            match brush {
                MAHBrush::Circle { radius } => {
                    let amplitude = PatternEvaluator::unit_convert_dist_to_hapev2(radius);
                    BrushEvalParams {
                        primitive_type: std::mem::discriminant(brush),
                        primitive_params,
                        painter: Painter {
                            z_rot: 0.0,
                            x_scale: amplitude,
                            y_scale: amplitude,
                        },
                    }
                }
                MAHBrush::Line { length, thickness, rotation } => {
                    let length = PatternEvaluator::unit_convert_dist_to_hapev2(length);
                    let thickness = PatternEvaluator::unit_convert_dist_to_hapev2(thickness);
                    let rotation = PatternEvaluator::unit_convert_rot_to_hapev2(rotation);
                    BrushEvalParams {
                        primitive_type: std::mem::discriminant(brush),
                        primitive_params,
                        painter: Painter {
                            z_rot: rotation,
                            x_scale: length,
                            y_scale: thickness,
                        },
                    }
                }
            }
        }

        let prev_brush = prev_kfc.brush.as_ref();
        let next_brush = next_kfc.brush.as_ref();
        match (prev_brush, next_brush) {
            (Some(prev_brush), Some(next_brush)) => {
                let prev_brush_eval = eval_mahbrush(&prev_brush.pwt.brush);
                let next_brush_eval = eval_mahbrush(&next_brush.pwt.brush);
                if prev_brush_eval.primitive_type == next_brush_eval.primitive_type {
                    let (pf, nf) = Self::perform_transition_interp(pattern_time, prev_brush.time, next_brush.time, &prev_brush.pwt.transition);
                    BrushEvalParams {
                        painter: Painter {
                            z_rot: prev_brush_eval.painter.z_rot * pf + nf * next_brush_eval.painter.z_rot,
                            x_scale: prev_brush_eval.painter.x_scale * pf + nf * next_brush_eval.painter.x_scale,
                            y_scale: prev_brush_eval.painter.y_scale * pf + nf * next_brush_eval.painter.y_scale,
                        },
                        primitive_type: prev_brush_eval.primitive_type,
                        primitive_params: prev_brush_eval.primitive_params,
                    }
                } else {
                    prev_brush_eval
                }
            }
            (Some(prev_brush), None) => eval_mahbrush(&prev_brush.pwt.brush),
            (None, _) => eval_mahbrush(&MAHBrush::Circle { radius: 0.0 }),
        }




    }

    fn time_to_hapev2_brush_rads(bp: &HapeV2PrimitiveParams, time: f64) -> f64 {
        let brush_time = (time / 1000.0) * bp.draw_frequency;
        let brush_t_rads = (brush_time * 2.0 * std::f64::consts::PI) % bp.max_t;
        brush_t_rads
    }
    fn eval_hapev2_primitive_equation(bp: &HapeV2PrimitiveParams, time: f64) -> HapeV2Coords {
        if bp.k != 0.0 { panic!("not yet implemented"); }
        let brush_t_rads = Self::time_to_hapev2_brush_rads(bp, time);
        HapeV2Coords {
            x: bp.A * (bp.a * brush_t_rads + bp.d).sin(),
            y: bp.B * (bp.b * brush_t_rads).sin(),
            z: 0.0,
        }
    }

    fn eval_hapev2_primitive_into_mah_units(pattern_time: MAHTime, brush_eval: &BrushEvalParams) -> MAHCoords {
        let brush_coords = Self::eval_hapev2_primitive_equation(&brush_eval.primitive_params, pattern_time);
        let sx = brush_coords.x * brush_eval.painter.x_scale;
        let sy = brush_coords.y * brush_eval.painter.y_scale;
        let rx = sx * brush_eval.painter.z_rot.cos() - sy * brush_eval.painter.z_rot.sin();
        let ry = sx * brush_eval.painter.z_rot.sin() + sy * brush_eval.painter.z_rot.cos();
        MAHCoords {
            x: rx * 1000.0,
            y: ry * 1000.0,
            z: 0.0,
        }
    }

    pub fn eval_path_at_anim_local_time(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> PathAtAnimLocalTime {
        let pattern_time = p.time + nep.time_offset;
        let prev_kfc = self.get_prev_kf_config(pattern_time);
        let next_kfc = self.get_next_kf_config(pattern_time);

        let coords = Self::eval_coords(pattern_time, &prev_kfc, &next_kfc);
        let intensity = Self::eval_intensity(pattern_time, &prev_kfc, &next_kfc);
        let brush = Self::eval_brush_hapev2(pattern_time, &prev_kfc, &next_kfc);

        let stop = match prev_kfc.keyframe {
            Some(MAHKeyframe::Stop(_)) => true,
            _ => false,
        };

        let next_eval_params = (|| {
            let kf = prev_kfc.keyframe?;
            if nep.last_cjump_eval_time <= *kf.time() {
                let cjump = kf.cjump()?;
                if cjump.condition.eval(p) {
                    return Some(NextEvalParams {
                        last_cjump_eval_time: cjump.jump_to,
                        time_offset: cjump.jump_to - p.time,
                    })
                }
            }
            None
        })().unwrap_or(NextEvalParams {
            last_cjump_eval_time: pattern_time,
            time_offset: nep.time_offset,
        });

        PathAtAnimLocalTime {
            coords, intensity, brush,
            pattern_time,
            stop,
            next_eval_params,
        }
    }


    pub fn eval_brush_at_anim_local_time(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> BrushAtAnimLocalTime {
        let path_eval = self.eval_path_at_anim_local_time(p, nep);

        let brush_coords_offset = Self::eval_hapev2_primitive_into_mah_units(path_eval.pattern_time, &path_eval.brush);
        BrushAtAnimLocalTime {
            coords: MAHCoords {
                x: path_eval.coords.x + brush_coords_offset.x,
                y: path_eval.coords.y + brush_coords_offset.y,
                z: path_eval.coords.z,
            },
            intensity: path_eval.intensity,

            pattern_time: path_eval.pattern_time,
            stop: path_eval.stop,
            next_eval_params: path_eval.next_eval_params,
        }
    }

    pub fn eval_brush_at_anim_local_time_for_max_t(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> Vec<BrushAtAnimLocalTime> {
        let max_number_of_points = 200;
        let device_frequency = 20000; //20khz

        let path_eval_base = self.eval_path_at_anim_local_time(&p, nep);

        let bp = &path_eval_base.brush.primitive_params;
        let max_t_in_ms = (1000.0 * bp.max_t / (bp.draw_frequency * 2.0 * std::f64::consts::PI)) as f64; //solve `time / 1000 * draw_frequency * 2Pi = max_t` equation for time

        let device_step = 1000.0 / device_frequency as f64;
        let min_step = max_t_in_ms / max_number_of_points as f64;
        // if (min_step > device_step) console.warn("min_step > device_step");

        let mut evals = vec![];
        let mut i = 0.0;
        let mut last_nep = nep;
        while i < max_t_in_ms {
            let step_p = PatternEvaluatorParameters { time: p.time + (i as f64), ..p.clone() };
            let eval_res = self.eval_brush_at_anim_local_time(&step_p, last_nep);
            evals.push(eval_res);
            last_nep = &evals.get(evals.len() - 1).unwrap().next_eval_params;
            i += f64::max(device_step, min_step);
        }

        evals
    }

}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl PatternEvaluator {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new_json(mah_animation_json: &str) -> Self {
        Self::new_from_json_string(mah_animation_json)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = eval_brush_at_anim_local_time))]
    pub fn eval_brush_at_anim_local_time_json(&self, p: &str) -> String {
        serde_json::to_string::<BrushAtAnimLocalTime>(&self.eval_brush_at_anim_local_time(&serde_json::from_str::<PatternEvaluatorParameters>(p).unwrap())).unwrap()
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = eval_brush_at_anim_local_time_for_max_t))]
    pub fn eval_brush_at_anim_local_time_for_max_t_json(&self, p: &str) -> String {
        serde_json::to_string::<Vec<BrushAtAnimLocalTime>>(&self.eval_brush_at_anim_local_time_for_max_t(&serde_json::from_str::<PatternEvaluatorParameters>(p).unwrap())).unwrap()
    }
}

pub type PatternEvalWasmPublicTypes = (MidAirHapticsAnimationFileFormat, PatternEvaluatorParameters, BrushAtAnimLocalTime, Vec<BrushAtAnimLocalTime>);


#[derive(Debug, Clone)]
struct PrimitiveWithTransitionAtTime<'a, T> {
    time: MAHTime,
    // primitve with transition
    pwt: &'a T,
}
#[derive(Debug, Clone, Default)]
struct MAHKeyframeConfig<'a> {
    coords: Option<PrimitiveWithTransitionAtTime<'a, CoordsWithTransition>>,
    brush: Option<PrimitiveWithTransitionAtTime<'a, BrushWithTransition>>,
    intensity: Option<PrimitiveWithTransitionAtTime<'a, IntensityWithTransition>>,
    keyframe: Option<MAHKeyframe>,
}

#[derive(Debug, Clone)]
pub struct PathAtAnimLocalTime {
    pub coords: MAHCoords,
    pub intensity: f64,
    pub pattern_time: MAHTime,
    pub stop: bool,
    pub next_eval_params: NextEvalParams,
    brush: BrushEvalParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrushAtAnimLocalTime {
    pub coords: MAHCoords,
    pub intensity: f64,
    pub pattern_time: MAHTime,
    pub stop: bool,
    pub next_eval_params: NextEvalParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NextEvalParams {
    last_cjump_eval_time: MAHTime,
    time_offset: MAHTime,
}
impl Default for NextEvalParams {
    fn default() -> Self {
        NextEvalParams {
            last_cjump_eval_time: 0.0,
            time_offset: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(non_snake_case)]
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
#[derive(Debug, Clone)]
struct BrushEvalParams {
    primitive_type: Discriminant<MAHBrush>,
    primitive_params: HapeV2PrimitiveParams,
    painter: Painter,
}
#[derive(Debug, Clone)]
struct Painter {
    z_rot: f64,
    x_scale: f64,
    y_scale: f64,
}

impl std::ops::Mul<f64> for &MAHCoords {
    type Output = MAHCoords;

    fn mul(self, rhs: f64) -> Self::Output {
        MAHCoords {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}
impl std::ops::Add<&MAHCoords> for &MAHCoords {
    type Output = MAHCoords;

    fn add(self, rhs: &MAHCoords) -> Self::Output {
        MAHCoords {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl std::ops::Add<MAHCoords> for MAHCoords {
    type Output = MAHCoords;

    fn add(self, rhs: MAHCoords) -> Self::Output {
        &self + &rhs
    }
}

impl MAHKeyframe {
    pub fn time(&self) -> &f64 {
        match self {
            MAHKeyframe::Standard(kf) => &kf.time,
            MAHKeyframe::Pause(kf) => &kf.time,
            MAHKeyframe::Stop(kf) => &kf.time,
        }
    }
    pub fn cjump(&self) -> Option<&ConditionalJump> {
        match self {
            MAHKeyframe::Standard(kf) => kf.cjump.as_ref(),
            MAHKeyframe::Pause(kf) => kf.cjump.as_ref(),
            MAHKeyframe::Stop(_) => None,
        }
    }
}

impl MAHCondition {
    pub fn eval(&self, params: &PatternEvaluatorParameters) -> bool {
        if let Some(user_param_value) = params.user_parameters.get(&self.parameter) {
            match self.operator {
                MAHConditionalOperator::Lt {  } => user_param_value < &self.value,
                MAHConditionalOperator::LtEq {  } => user_param_value <= &self.value,
                MAHConditionalOperator::Gt {  } => user_param_value > &self.value,
                MAHConditionalOperator::GtEq {  } => user_param_value >= &self.value,
            }
        } else {
            false
        }
    }
}


#[cfg(test)]
mod test {
    use std::time::Duration;
    use std::time::Instant;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    use crate::*;

    #[test]
    fn bench() {
        let warmup_iterations = 50;
        let mut max_time = Duration::default();
        let pattern_json_string_raw = "{\"$DATA_FORMAT\":\"MidAirHapticsAnimationFileFormat\",\"$REVISION\":\"0.0.4-alpha.1\",\"name\":\"test\",\"projection\":\"plane\",\"update_rate\":1,\"keyframes\":[{\"time\":0,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-60,\"y\":-40,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":500,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":10}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":5,\"y\":65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":1000,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2250,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-5,\"y\":-65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":2350,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2425,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":1,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2500,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":3750,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":360}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":50,\"y\":0,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}}]}";
        let pe = PatternEvaluator::new_from_json_string(pattern_json_string_raw);
        for o in 0..3000 {
            if o == warmup_iterations {
                println!("Warmup done, starting benchmark..");
                max_time = Duration::default();
            }
            let now = Instant::now();

            let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default() };
            let mut last_nep = NextEvalParams { last_cjump_eval_time: 0.0, time_offset: 0.0 };
            for i in 0..200 {
                let time = f64::from(i) * 0.05;
                pep.time = time;
                let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
                if eval_result.coords.z != 0.0 {
                    println!("{:?}", eval_result);
                }
                last_nep = eval_result.next_eval_params;
            }

            let elapsed = now.elapsed();
            if elapsed > max_time {
                max_time = elapsed;
            }
            println!("elapsed: {:.2?}", elapsed);
        }
        println!("Max elapsed: {:.2?}", max_time);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    #[test]
    fn test() {
        let warmup_iterations = 5;
        let pattern_json_string_raw = "{\"$DATA_FORMAT\":\"MidAirHapticsAnimationFileFormat\",\"$REVISION\":\"0.0.4-alpha.1\",\"name\":\"test\",\"projection\":\"plane\",\"update_rate\":1,\"keyframes\":[{\"time\":0,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-60,\"y\":-40,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":500,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":10}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":5,\"y\":65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":1000,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2250,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":15}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":-5,\"y\":-65,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}},{\"time\":2350,\"brush\":{\"brush\":{\"name\":\"circle\",\"params\":{\"radius\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2425,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":1,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":2500,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":0}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"pause\"},{\"time\":3750,\"brush\":{\"brush\":{\"name\":\"line\",\"params\":{\"length\":5,\"thickness\":1,\"rotation\":360}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"intensity\":{\"intensity\":{\"name\":\"constant\",\"params\":{\"value\":1}},\"transition\":{\"name\":\"linear\",\"params\":{}}},\"type\":\"standard\",\"coords\":{\"coords\":{\"x\":50,\"y\":0,\"z\":0},\"transition\":{\"name\":\"linear\",\"params\":{}}}}]}";
        let pe = PatternEvaluator::new_from_json_string(pattern_json_string_raw);
        for o in 0..3000 {
            if o == warmup_iterations {
                println!("Warmup done, starting benchmark..");
            }

            let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default() };
            let mut last_nep = NextEvalParams { last_cjump_eval_time: 0.0, time_offset: 0.0 };
            for i in 0..200 {
                let time = f64::from(i) * 0.05;
                pep.time = time;
                let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
                if eval_result.coords.z != 0.0 {
                    println!("{:?}", eval_result);
                }
                last_nep = eval_result.next_eval_params;
            }
        }
    }
}