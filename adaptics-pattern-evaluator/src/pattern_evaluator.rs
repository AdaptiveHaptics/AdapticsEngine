mod shared_types;
mod atformula_parser;
use std::{mem::Discriminant, collections::HashMap};

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
pub use shared_types::*;
pub use atformula_parser::parse_formula;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct PatternEvaluator {
    mah_animation: MidAirHapticsAnimationFileFormat,
}

pub type UserParameters = HashMap<String, f64>;
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PatternEvaluatorParameters {
    pub time: MAHTime,
    pub user_parameters: UserParameters,
    pub geometric_transform: GeometricTransformMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HapeV2Coords {
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

    pub fn new_from_json_string(mah_animation_json: &str) -> Result<Self, serde_json::Error> {
        let mut mah_animation: MidAirHapticsAnimationFileFormat = serde_json::from_str(mah_animation_json)?;
        mah_animation.keyframes.sort_by(|a, b| a.time().total_cmp(b.time()));

        Ok(Self { mah_animation })
    }

    fn get_kf_config_type(&self, t: MAHTime, prev: bool) -> MAHKeyframeConfig {
        let mut kfc = MAHKeyframeConfig::default();
        macro_rules! update_kfc {
            ($kf:ident, $prop:ident ?) => { // update time and value (if optional prop present)
                kfc.$prop = $kf.$prop.as_ref().map(|b| PrimitiveWithTransitionAtTime {
                    time: $kf.time,
                    pwt: b,
                }).or(kfc.$prop);
            };
            ($kf:ident, $prop:ident !) => { // update time and value with non optional prop
                kfc.$prop = Some(PrimitiveWithTransitionAtTime {
                    time: $kf.time,
                    pwt: &$kf.$prop,
                });
            };
            ($kf:ident, $prop:ident :) => { // update time only
                kfc.$prop = kfc.$prop.map(|mut c| { c.time = $kf.time; c });
            };
        }
        let kf_iter: Vec<_> = if prev { self.mah_animation.keyframes.iter().collect() } else { self.mah_animation.keyframes.iter().rev().collect() };
        for kf in kf_iter {
            if prev { if kf.time() > &t { break; } }
			else if kf.time() <= &t { break; }
            match kf {
                MAHKeyframe::Standard(kf) => {
                    update_kfc!(kf, coords !);
                    update_kfc!(kf, brush ?);
                    update_kfc!(kf, intensity ?);
                },
                MAHKeyframe::Pause(kf) => {
                    update_kfc!(kf, coords :);
                    update_kfc!(kf, brush ?);
                    update_kfc!(kf, intensity ?);
                },
                MAHKeyframe::Stop(kf) => {
                    update_kfc!(kf, coords :); // pause behavior
                },
            }
            kfc.keyframe = Some(kf.clone());
        }
        kfc
    }
    fn get_prev_kf_config(&self, t: MAHTime) -> MAHKeyframeConfig {
        self.get_kf_config_type(t, true)
    }
    fn get_next_kf_config(&self, t: MAHTime) -> MAHKeyframeConfig {
        self.get_kf_config_type(t, false)
    }

    fn get_cjumps_from_last_eval_to_current(&self, last_eval_pattern_time: MAHTime, pattern_time: MAHTime) -> impl Iterator<Item=&ConditionalJump> {
        self.mah_animation.keyframes.iter().filter_map(move |kf| {
            let kf_time = kf.time();
            if &last_eval_pattern_time < kf_time && kf_time <= &pattern_time {
                kf.cjumps()
            } else {
                None
            }
        }).flatten()
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

    fn eval_intensity(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig, dyn_up_info: &DynUserParamInfo) -> f64 {
        let prev_intensity = prev_kfc.intensity.as_ref();
        let next_intensity = next_kfc.intensity.as_ref();

        fn get_intensity_value(intensity: &MAHIntensity, dyn_up_info: &DynUserParamInfo) -> f64 {
            match &intensity {
                MAHIntensity::Constant { value } => value.to_f64(dyn_up_info),
                MAHIntensity::Random { min, max } => {
                    let min_f64 = min.to_f64(dyn_up_info);
                    let max_f64 = max.to_f64(dyn_up_info);
                    rand::random::<f64>() * (max_f64 - min_f64) + min_f64
                }
            }
        }

        if let (Some(prev_intensity), Some(next_intensity)) = (prev_intensity, next_intensity) {
            let piv = get_intensity_value(&prev_intensity.pwt.intensity, dyn_up_info);
            let niv = get_intensity_value(&next_intensity.pwt.intensity, dyn_up_info);
            let (pf, nf) = Self::perform_transition_interp(pattern_time, prev_intensity.time, next_intensity.time, &prev_intensity.pwt.transition);
            pf * piv + nf * niv
        } else if let Some(prev_intensity) = prev_intensity {
            return get_intensity_value(&prev_intensity.pwt.intensity, dyn_up_info);
        } else {
            return 1.0;
        }
    }

    fn eval_coords(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> MAHCoordsConst {
        let prev_coords_att = prev_kfc.coords.as_ref();
        let next_coords_att = next_kfc.coords.as_ref();
        let next_keyframe = next_kfc.keyframe.as_ref();
        if let (Some(prev_coords_att), Some(next_coords_att), Some(next_keyframe)) = (prev_coords_att, next_coords_att, next_keyframe) {
            match next_keyframe {
                MAHKeyframe::Stop(_) | MAHKeyframe::Pause(_) => { prev_coords_att.pwt.coords.clone() },
                MAHKeyframe::Standard(_) => {
                    let (pf, nf) = Self::perform_transition_interp(pattern_time, prev_coords_att.time, next_coords_att.time, &prev_coords_att.pwt.transition);
                    &prev_coords_att.pwt.coords * pf + &next_coords_att.pwt.coords * nf
                }
            }
        } else if let Some(prev_coords_att) = prev_coords_att {
            return prev_coords_att.pwt.coords.clone();
        } else {
            return MAHCoordsConst { x: 0.0, y: 0.0, z: 0.0 };
        }
    }

    /// converts millimeters to meters
    pub fn unit_convert_dist_to_hapev2(mahunit: &f64) -> f64 {
        mahunit / 1000.0
    }

    /// converts millimeters to meters
    fn unit_convert_dist_from_hapev2(hapev2unit: &f64) -> f64 {
        hapev2unit * 1000.0
    }

    /// converts degrees to radians
    fn unit_convert_rot_to_hapev2(mahunit: &f64) -> f64 {
        mahunit * (std::f64::consts::PI / 180.0)
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

    fn eval_brush_hapev2(pattern_time: MAHTime, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig, dyn_up_info: &DynUserParamInfo) -> BrushEvalParams {
        fn eval_mahbrush(brush: &MAHBrush, dyn_up_info: &DynUserParamInfo) -> BrushEvalParams {
            let primitive_params = PatternEvaluator::get_hapev2_primitive_params_for_brush(brush);
            match brush {
                MAHBrush::Circle { radius, am_freq } => {
                    let amplitude = PatternEvaluator::unit_convert_dist_to_hapev2(&radius.to_f64(dyn_up_info));
                    BrushEvalParams {
                        primitive_type: std::mem::discriminant(brush),
                        primitive_params,
                        painter: Painter {
                            z_rot: 0.0,
                            x_scale: amplitude,
                            y_scale: amplitude,
                        },
                        am_freq: am_freq.to_f64(dyn_up_info),
                    }
                }
                MAHBrush::Line { length, thickness, rotation, am_freq } => {
                    let length = PatternEvaluator::unit_convert_dist_to_hapev2(&length.to_f64(dyn_up_info));
                    let thickness = PatternEvaluator::unit_convert_dist_to_hapev2(&thickness.to_f64(dyn_up_info));
                    let rotation = PatternEvaluator::unit_convert_rot_to_hapev2(&rotation.to_f64(dyn_up_info));
                    BrushEvalParams {
                        primitive_type: std::mem::discriminant(brush),
                        primitive_params,
                        painter: Painter {
                            z_rot: rotation,
                            x_scale: length,
                            y_scale: thickness,
                        },
                        am_freq: am_freq.to_f64(dyn_up_info),
                    }
                }
            }
        }

        let prev_brush = prev_kfc.brush.as_ref();
        let next_brush = next_kfc.brush.as_ref();
        match (prev_brush, next_brush) {
            (Some(prev_brush), Some(next_brush)) => {
                let prev_brush_eval = eval_mahbrush(&prev_brush.pwt.brush, dyn_up_info);
                let next_brush_eval = eval_mahbrush(&next_brush.pwt.brush, dyn_up_info);
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
                        am_freq: prev_brush_eval.am_freq * pf + nf * next_brush_eval.am_freq,
                    }
                } else {
                    prev_brush_eval
                }
            }
            (Some(prev_brush), None) => eval_mahbrush(&prev_brush.pwt.brush, dyn_up_info),
            (None, _) => eval_mahbrush(&MAHBrush::Circle { radius: 0.0.into(), am_freq: 0.0.into() }, dyn_up_info),
        }




    }

    fn time_to_hapev2_brush_rads(bp: &HapeV2PrimitiveParams, time: f64) -> f64 {
        let brush_time = (time / 1000.0) * bp.draw_frequency;

        (brush_time * 2.0 * std::f64::consts::PI) % bp.max_t
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

    fn eval_hapev2_primitive_into_mah_units(pattern_time: MAHTime, brush_eval: &BrushEvalParams) -> UltraleapControlPoint {
        let brush_coords = Self::eval_hapev2_primitive_equation(&brush_eval.primitive_params, pattern_time);
        let sx = brush_coords.x * brush_eval.painter.x_scale;
        let sy = brush_coords.y * brush_eval.painter.y_scale;
        let rx = sx * brush_eval.painter.z_rot.cos() - sy * brush_eval.painter.z_rot.sin();
        let ry = sx * brush_eval.painter.z_rot.sin() + sy * brush_eval.painter.z_rot.cos();
        // apply amplitude modulation from brush_eval.am_freq (in HZ)
        let intensity = (brush_eval.am_freq * (pattern_time / 1000.0) * 2.0 * std::f64::consts::PI).cos() * 0.5 + 0.5;

        UltraleapControlPoint {
            coords: MAHCoordsConst {
                x: Self::unit_convert_dist_from_hapev2(&rx),
                y: Self::unit_convert_dist_from_hapev2(&ry),
                z: 0.0,
            },
            intensity,
        }
    }

    pub fn eval_path_at_anim_local_time(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> PathAtAnimLocalTime {
        let dyn_up_info = UserParametersConstrained::from(&p.user_parameters, &self.mah_animation.user_parameter_definitions);

        // apply playback_speed
        let (pattern_time, nep) = {
            let last_eval_pattern_time = nep.last_eval_pattern_time;
            let delta_time = p.time + nep.time_offset - last_eval_pattern_time;
            let delta_for_speed = self.mah_animation.pattern_transform.playback_speed.to_f64(&dyn_up_info) * delta_time;
            let time_offset = nep.time_offset + delta_for_speed - delta_time;
            let pattern_time = p.time + time_offset;
            (pattern_time, NextEvalParams { time_offset, last_eval_pattern_time })
        };

        // apply (one) cjump
        let (pattern_time, nep) = {
            let nep = self.get_cjumps_from_last_eval_to_current(nep.last_eval_pattern_time, pattern_time)
                .find(|cjump| cjump.condition.eval(&dyn_up_info))
                .map_or(NextEvalParams {
                    last_eval_pattern_time: pattern_time,
                    time_offset: nep.time_offset,
                }, |cjump| {
                    NextEvalParams {
                        last_eval_pattern_time: cjump.jump_to,
                        time_offset: cjump.jump_to - p.time,
                    }
                });
            let pattern_time = p.time + nep.time_offset;
            (pattern_time, nep)
        };

        let prev_kfc = self.get_prev_kf_config(pattern_time);
        let next_kfc = self.get_next_kf_config(pattern_time);

        let coords = Self::eval_coords(pattern_time, &prev_kfc, &next_kfc);
        let intensity = Self::eval_intensity(pattern_time, &prev_kfc, &next_kfc, &dyn_up_info);
        let brush = Self::eval_brush_hapev2(pattern_time, &prev_kfc, &next_kfc, &dyn_up_info);

        // apply intensity_factor
        let intensity = self.mah_animation.pattern_transform.intensity_factor.to_f64(&dyn_up_info) * intensity;

        let coords = self.mah_animation.pattern_transform.geometric_transforms.apply(&coords, &dyn_up_info);

        // apply final geometric transform (intended for hand tracking etc.)
        let coords = p.geometric_transform.projection_transform(&coords);

        let stop = matches!(prev_kfc.keyframe, Some(MAHKeyframe::Stop(_)));

        PathAtAnimLocalTime {
            ul_control_point: UltraleapControlPoint { coords, intensity, },
            pattern_time,
            stop,
            next_eval_params: nep,
            brush,
        }
    }


    pub fn eval_brush_at_anim_local_time(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> BrushAtAnimLocalTime {
        let path_eval = self.eval_path_at_anim_local_time(p, nep);

        let brush_coords_offset = Self::eval_hapev2_primitive_into_mah_units(p.time, &path_eval.brush);
        BrushAtAnimLocalTime {
            ul_control_point: UltraleapControlPoint {
                coords: MAHCoordsConst {
                    x: path_eval.ul_control_point.coords.x + brush_coords_offset.coords.x,
                    y: path_eval.ul_control_point.coords.y + brush_coords_offset.coords.y,
                    z: path_eval.ul_control_point.coords.z,
                },
                intensity: path_eval.ul_control_point.intensity * brush_coords_offset.intensity,
            },

            pattern_time: path_eval.pattern_time,
            stop: path_eval.stop,
            next_eval_params: path_eval.next_eval_params,
        }
    }

    pub fn eval_brush_at_anim_local_time_for_max_t(&self, p: &PatternEvaluatorParameters, nep: &NextEvalParams) -> Vec<BrushAtAnimLocalTime> {
        let max_number_of_points = 200;
        let device_frequency = 20000; //20khz

        let path_eval_base = self.eval_path_at_anim_local_time(p, nep);
        let bp = &path_eval_base.brush.primitive_params;
        let max_t_in_ms = 1000.0 * bp.max_t / (bp.draw_frequency * 2.0 * std::f64::consts::PI); //solve `time / 1000 * draw_frequency * 2Pi = max_t` equation for time
        // let max_t_in_ms = 16.6; // ~60fps, above calculation usually evaluates to ~10ms

        let device_step = 1000.0 / device_frequency as f64;
        let min_step = max_t_in_ms / max_number_of_points as f64;
        // if (min_step > device_step) console.warn("min_step > device_step");

        let mut evals = vec![];
        let mut i = 0.0;
        let mut last_nep = nep;
        while i < max_t_in_ms {
            let step_p = PatternEvaluatorParameters { time: p.time + i, ..p.clone() };
            let eval_res = self.eval_brush_at_anim_local_time(&step_p, last_nep);
            evals.push(eval_res);
            last_nep = &evals.last().unwrap().next_eval_params;
            i += f64::max(device_step, min_step);
        }

        evals
    }

}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl PatternEvaluator {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new_json(mah_animation_json: &str) -> Result<PatternEvaluator, JsError> {
        Ok(Self::new_from_json_string(mah_animation_json)?)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = eval_brush_at_anim_local_time))]
    pub fn eval_brush_at_anim_local_time_json(&self, p: &str, nep: &str) -> String {
        serde_json::to_string::<BrushAtAnimLocalTime>(&self.eval_brush_at_anim_local_time(&serde_json::from_str::<PatternEvaluatorParameters>(p).unwrap(), &serde_json::from_str::<NextEvalParams>(nep).unwrap())).unwrap()
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = eval_brush_at_anim_local_time_for_max_t))]
    pub fn eval_brush_at_anim_local_time_for_max_t_json(&self, p: &str, nep: &str) -> String {
        serde_json::to_string::<Vec<BrushAtAnimLocalTime>>(&self.eval_brush_at_anim_local_time_for_max_t(&serde_json::from_str::<PatternEvaluatorParameters>(p).unwrap(), &serde_json::from_str::<NextEvalParams>(nep).unwrap())).unwrap()
    }

    pub fn default_next_eval_params() -> String {
        serde_json::to_string::<NextEvalParams>(&NextEvalParams::default()).unwrap()
    }

    pub fn default_pattern_transformation() -> String {
        serde_json::to_string::<PatternTransformation>(&PatternTransformation::default()).unwrap()
    }

    pub fn default_geo_transform_matrix() -> String {
        serde_json::to_string::<GeometricTransformMatrix>(&GeometricTransformMatrix::default()).unwrap()
    }

    pub fn geo_transform_simple_apply(gts: &str, coords: &str, user_parameters: &str, user_parameter_definitions: &str) -> String {
        let gts = serde_json::from_str::<GeometricTransformsSimple>(gts).unwrap();
        let coords = serde_json::from_str::<MAHCoordsConst>(coords).unwrap();
        let user_parameters = serde_json::from_str::<UserParameters>(user_parameters).unwrap();
        let user_parameter_definitions = serde_json::from_str::<UserParameterDefinitions>(user_parameter_definitions).unwrap();
        let dyn_up_info = UserParametersConstrained::from(&user_parameters, &user_parameter_definitions);
        serde_json::to_string::<MAHCoordsConst>(&gts.apply(&coords, &dyn_up_info)).unwrap()
    }

    pub fn geo_transform_simple_inverse(gts: &str, coords: &str, user_parameters: &str, user_parameter_definitions: &str) -> String {
        let gts = serde_json::from_str::<GeometricTransformsSimple>(gts).unwrap();
        let coords = serde_json::from_str::<MAHCoordsConst>(coords).unwrap();
        let user_parameters = serde_json::from_str::<UserParameters>(user_parameters).unwrap();
        let user_parameter_definitions = serde_json::from_str::<UserParameterDefinitions>(user_parameter_definitions).unwrap();
        let dyn_up_info = UserParametersConstrained::from(&user_parameters, &user_parameter_definitions);
        serde_json::to_string::<MAHCoordsConst>(&gts.inverse(&coords, &dyn_up_info)).unwrap()
    }

    pub fn parse_formula(formula: &str) -> Result<String, JsError> {
        Ok(serde_json::to_string::<ATFormula>(&parse_formula(formula)?)?)
    }
    pub fn formula_to_string(formula: &str) -> Result<String, JsError> {
        Ok(serde_json::from_str::<ATFormula>(formula)?.to_formula_string())
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct UltraleapControlPoint {
    pub coords: MAHCoordsConst,
    // pub direction: MAHCoords,
    pub intensity: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathAtAnimLocalTime {
    pub ul_control_point: UltraleapControlPoint,
    pub pattern_time: MAHTime,
    pub stop: bool,
    pub next_eval_params: NextEvalParams,
    brush: BrushEvalParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BrushAtAnimLocalTime {
    pub ul_control_point: UltraleapControlPoint,
    pub pattern_time: MAHTime,
    pub stop: bool,
    pub next_eval_params: NextEvalParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct NextEvalParams {
    last_eval_pattern_time: MAHTime,
    time_offset: MAHTime,
}
impl Default for NextEvalParams {
    fn default() -> Self {
        NextEvalParams {
            last_eval_pattern_time: 0.0,
            time_offset: 0.0,
        }
    }
}
impl NextEvalParams {
    pub fn new(last_eval_pattern_time: MAHTime, time_offset: MAHTime) -> Self {
        Self {
            last_eval_pattern_time,
            time_offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
struct BrushEvalParams {
    primitive_type: Discriminant<MAHBrush>,
    primitive_params: HapeV2PrimitiveParams,
    painter: Painter,
    /// AM frequency in HZ
    am_freq: f64,
}
#[derive(Debug, Clone, PartialEq)]
struct Painter {
    z_rot: f64,
    x_scale: f64,
    y_scale: f64,
}

impl std::ops::Mul<f64> for &MAHCoordsConst {
    type Output = MAHCoordsConst;

    fn mul(self, rhs: f64) -> Self::Output {
        MAHCoordsConst {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}
impl std::ops::Add<&MAHCoordsConst> for &MAHCoordsConst {
    type Output = MAHCoordsConst;

    fn add(self, rhs: &MAHCoordsConst) -> Self::Output {
        MAHCoordsConst {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl std::ops::Add<MAHCoordsConst> for MAHCoordsConst {
    type Output = MAHCoordsConst;

    fn add(self, rhs: MAHCoordsConst) -> Self::Output {
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
    pub fn cjumps(&self) -> Option<&Vec<ConditionalJump>> {
        match self {
            MAHKeyframe::Standard(kf) => Some(&kf.cjumps),
            MAHKeyframe::Pause(kf) => Some(&kf.cjumps),
            MAHKeyframe::Stop(_) => None,
        }
    }
}

impl MAHCondition {
    fn eval(&self, dyn_up_info: &DynUserParamInfo) -> bool {
        if let Some(user_param_value) = dyn_up_info.0.get(&self.parameter) {
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

impl GeometricTransformMatrix {
    fn affine_transform(&self, coords: &MAHCoordsConst) -> MAHCoordsConst {
        let w = 1.0; // always assume `coords` represents a point, not a vector
        MAHCoordsConst {
            x: self[0][0] * coords.x + self[0][1] * coords.y + self[0][2] * coords.z + self[0][3] * w,
            y: self[1][0] * coords.x + self[1][1] * coords.y + self[1][2] * coords.z + self[1][3] * w,
            z: self[2][0] * coords.x + self[2][1] * coords.y + self[2][2] * coords.z + self[2][3] * w,
        }
    }
    fn projection_transform(&self, coords: &MAHCoordsConst) -> MAHCoordsConst {
        let mut new_coords = self.affine_transform(coords);
        let w = 1.0; // always assume `coords` represents a point, not a vector
        let new_w = self[3][0] * coords.x + self[3][1] * coords.y + self[3][2] * coords.z + self[3][3] * w;
        new_coords.x /= new_w;
        new_coords.y /= new_w;
        new_coords.z /= new_w;
        new_coords
    }
}

impl GeometricTransformsSimple {
    fn apply(&self, coords: &MAHCoordsConst, dyn_up_info: &DynUserParamInfo) -> MAHCoordsConst {
        let mut coords = coords.clone();

        //scale
        coords.x *= self.scale.x.to_f64(dyn_up_info);
        coords.y *= self.scale.y.to_f64(dyn_up_info);
        coords.z *= self.scale.z.to_f64(dyn_up_info);

        //rotate
        let radians = self.rotation.to_f64(dyn_up_info) / 180.0 * std::f64::consts::PI;
        coords = MAHCoordsConst {
            x: coords.x * radians.cos() - coords.y * radians.sin(),
            y: coords.x * radians.sin() + coords.y * radians.cos(),
            z: coords.z,
        };

        //translate
        coords.x += self.translate.x.to_f64(dyn_up_info);
        coords.y += self.translate.y.to_f64(dyn_up_info);
        coords.z += self.translate.z.to_f64(dyn_up_info);

        coords
    }
    #[cfg(target_arch = "wasm32")] //only used in web gui, for now
    fn inverse(&self, coords: &MAHCoordsConst, dyn_up_info: &DynUserParamInfo) -> MAHCoordsConst {
        let mut coords = coords.clone();

        //translate
        coords.x -= self.translate.x.to_f64(dyn_up_info);
        coords.y -= self.translate.y.to_f64(dyn_up_info);
        coords.z -= self.translate.z.to_f64(dyn_up_info);

        //rotate
        let radians = self.rotation.to_f64(dyn_up_info) / 180.0 * std::f64::consts::PI;
        coords = MAHCoordsConst {
            x: coords.x * radians.cos() + coords.y * radians.sin(),
            y: -coords.x * radians.sin() + coords.y * radians.cos(),
            z: coords.z,
        };

        //scale
        coords.x /= self.scale.x.to_f64(dyn_up_info);
        coords.y /= self.scale.y.to_f64(dyn_up_info);
        coords.z /= self.scale.z.to_f64(dyn_up_info);

        coords
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserParametersConstrained(HashMap<String, f64>);
impl UserParametersConstrained {
    fn from(user_parameters: &UserParameters, definitions: &UserParameterDefinitions) -> Self {
        let mut constrained_user_parameters = user_parameters.clone(); // use user_parameters as base, to keep params that do not have an explicit definition
        for (name, def) in definitions {
            constrained_user_parameters.insert(name.clone(), user_parameters.get(name)
                .unwrap_or(&def.default).clamp(
                    def.min.unwrap_or(f64::NEG_INFINITY),
                    def.max.unwrap_or(f64::INFINITY)
                )
            );
        }
        Self(constrained_user_parameters)
    }
}
type DynUserParamInfo = UserParametersConstrained;
impl MAHDynamicF64 {
    fn to_f64(&self, dyn_up_info: &DynUserParamInfo) -> f64 {
        match self {
            MAHDynamicF64::Dynamic(param) => *dyn_up_info.0.get(param).unwrap_or(&0.0),
            MAHDynamicF64::F64(f) => *f,
            MAHDynamicF64::Formula(formula) => formula.eval(dyn_up_info),
        }
    }
}
impl ATFormula {
    fn eval(&self, dyn_up_info: &DynUserParamInfo) -> f64 {
        match self {
            ATFormula::Constant(c) => *c,
            ATFormula::Parameter(p) => *dyn_up_info.0.get(p).unwrap_or(&0.0),
            ATFormula::Add(left, right) => left.eval(dyn_up_info) + right.eval(dyn_up_info),
            ATFormula::Subtract(left, right) => left.eval(dyn_up_info) - right.eval(dyn_up_info),
            ATFormula::Multiply(left, right) => left.eval(dyn_up_info) * right.eval(dyn_up_info),
            ATFormula::Divide(left, right) => left.eval(dyn_up_info) / right.eval(dyn_up_info),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pattern() -> MidAirHapticsAnimationFileFormat {
        MidAirHapticsAnimationFileFormat {
            data_format: MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
            revision: DataFormatRevision::CurrentRevision,
            name: "example".to_string(),
            keyframes: vec![
                MAHKeyframe::Standard(MAHKeyframeStandard {
                    time: 0.0,
                    brush: Some(BrushWithTransition {
                        brush: MAHBrush::Circle { radius: MAHDynamicF64::F64(10.0), am_freq: MAHDynamicF64::F64(0.0) },
                        transition: MAHTransition::Linear {  }
                    }),
                    intensity: Some(IntensityWithTransition {
                        intensity: MAHIntensity::Constant { value: MAHDynamicF64::F64(1.0) },
                        transition: MAHTransition::Linear { },
                    }),
                    coords: CoordsWithTransition {
                        coords: MAHCoordsConst { x: -10.0, y: 0.0, z: 0.0 },
                        transition: MAHTransition::Linear { },
                    },
                    cjumps: vec![],
                }),
                MAHKeyframe::Standard(MAHKeyframeStandard {
                    time: 10.0,
                    brush: Some(BrushWithTransition {
                        brush: MAHBrush::Circle { radius: MAHDynamicF64::F64(5.0), am_freq: MAHDynamicF64::F64(0.0) },
                        transition: MAHTransition::Linear {  }
                    }),
                    intensity: Some(IntensityWithTransition {
                        intensity: MAHIntensity::Constant { value: MAHDynamicF64::F64(1.0) },
                        transition: MAHTransition::Linear { },
                    }),
                    coords: CoordsWithTransition {
                        coords: MAHCoordsConst { x: 10.0, y: 0.0, z: 0.0 },
                        transition: MAHTransition::Linear { },
                    },
                    cjumps: vec![ ConditionalJump {
                        condition: MAHCondition { parameter: "param1".to_string(), operator: MAHConditionalOperator::Lt {}, value: 3.0 },
                        jump_to: 1.0,
                    }],
                }),
            ],
            pattern_transform: Default::default(),
            user_parameter_definitions: HashMap::from([
                ("param1".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: 1.0 }),
                ("param2".to_string(), MAHUserParameterDefinition { default: 20.0, min: Some(0.0), max: Some(15.0), step: 15.0 }),
                ("param3".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: -500.0 }),
                ("param4".to_string(), MAHUserParameterDefinition { default: 75.0, min: Some(-100.0), max: Some(50.0), step: 13.0 }),
                ("param5".to_string(), MAHUserParameterDefinition { default: 1.0, min: Some(0.0), max: Some(4.0), step: 0.05 }),
            ]),
        }
    }

    fn create_test_pattern_json() -> String {
        serde_json::to_string(&create_test_pattern()).unwrap()
    }

    #[test]
    fn test_constrain_user_params() {
        let user_parameters = HashMap::from_iter(vec![
            ("pA".to_string(), 5.0),
            ("pB".to_string(), 10.0),
            ("pC".to_string(), 50.0),
            ("pD".to_string(), -50.0),
        ]);
        let user_parameter_definitions = HashMap::from_iter(vec![
            ("pB".to_string(), MAHUserParameterDefinition { default: 20.0, min: Some(0.0), max: Some(15.0), step: 15.0 }),
            ("pC".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: -500.0 }),
            ("pD".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: 1.2048790 }),
            ("pE".to_string(), MAHUserParameterDefinition { default: 12.0001, min: None, max: None, step: 0.05 }),
        ]);
        let dyn_up_info = UserParametersConstrained::from(&user_parameters, &user_parameter_definitions);
        // assert_eq!(dyn_up_info.0.len(), 5);
        assert_eq!(dyn_up_info.0["pA"], 5.0);
        assert_eq!(dyn_up_info.0["pB"], 10.0);
        assert_eq!(dyn_up_info.0["pC"], 10.0);
        assert_eq!(dyn_up_info.0["pD"], 0.0);
        assert_eq!(dyn_up_info.0["pE"], 12.0001);
    }

    #[test]
    fn test_mah_condition_eval() {
        let dyn_up_info = UserParametersConstrained(HashMap::from_iter(vec![("pA".to_string(), 2.0)]));
        let cond = MAHCondition { parameter: "pA".to_string(), operator: MAHConditionalOperator::Lt {}, value: 3.0 };
        assert!(cond.eval(&dyn_up_info));
    }

    #[test]
    fn test_geometric_transform_matrix_projection_transform() {
        let matrix = GeometricTransformMatrix([
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 2.0],
            [0.0, 0.0, 1.0, 3.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let coords = MAHCoordsConst { x: 1.0, y: 2.0, z: 3.0 };
        let expected = MAHCoordsConst { x: 2.0, y: 4.0, z: 6.0 };
        assert_eq!(matrix.projection_transform(&coords), expected);
    }

    #[test]
    fn test_basic_pattern() {
        let pattern_eval = PatternEvaluator::new_from_json_string(&create_test_pattern_json()).unwrap();
        let p = PatternEvaluatorParameters {
            time: 0.0,
            user_parameters: HashMap::from_iter(vec![
                ("pA".to_string(), 2.0),
                ("pB".to_string(), 15.0),
            ]),
            geometric_transform: GeometricTransformMatrix::default(),
        };
        let nep = NextEvalParams::default();
        let eval_res = pattern_eval.eval_path_at_anim_local_time(&p, &nep);

        let expected_brush = MAHBrush::Circle { radius: 0.0.into(), am_freq: 0.0.into() };
        let primitive = PatternEvaluator::get_hapev2_primitive_params_for_brush(&expected_brush);
        assert_eq!(eval_res, PathAtAnimLocalTime {
            ul_control_point: UltraleapControlPoint { coords: MAHCoordsConst { x: -10.0, y: 0.0, z: 200.0 }, intensity: 1.0 },
            pattern_time: 0.0,
            stop: false,
            next_eval_params: NextEvalParams { time_offset: 0.0, last_eval_pattern_time: 0.0 },
            brush: BrushEvalParams {
                primitive_type: std::mem::discriminant(&expected_brush),
                primitive_params: primitive,
                painter: Painter { z_rot: 0.0, x_scale: 0.01, y_scale: 0.01 },
                am_freq: 0.0,
            }
        });
    }


    #[test]
    fn test_atformula_eval() {
        #[allow(non_snake_case)]
        let pA = 2.0; let param = 11.0; let param2 = 12.0; let param3 = 13.0; let param4 = 14.0;
        let dyn_up_info = UserParametersConstrained(HashMap::from_iter(vec![
            ("pA".to_string(), 2.0),
            ("param".to_string(), 11.0),
            ("param2".to_string(), 12.0),
            ("param3".to_string(), 13.0),
            ("param4".to_string(), 14.0),
        ]));
        let formula = parse_formula("1.0 + 2.0").unwrap();
        assert_eq!(formula.eval(&dyn_up_info), 1.0 + 2.0);
        let formula = parse_formula("1.0 + pA").unwrap();
        assert_eq!(formula.eval(&dyn_up_info), 1.0 + pA);
        let formula = parse_formula("1 * param + 2 / param2 - 3 * param3 + 4 / param4").unwrap();
        assert_eq!(formula.eval(&dyn_up_info), 1.0 * param + 2.0 / param2 - 3.0 * param3 + 4.0 / param4);
        let formula = parse_formula("1 + (2 * param - (3 / (4 - 5 * (param2 + 6))))").unwrap();
        assert_eq!(formula.eval(&dyn_up_info), 1.0 + (2.0 * param - (3.0 / (4.0 - 5.0 * (param2 + 6.0)))));
    }







    use std::time::Duration;
    use std::time::Instant;

    #[test]
    #[ignore="bench"]
    fn bench() {
        let warmup_iterations = 50;
        let mut max_time = Duration::default();
        let pe = PatternEvaluator::new_from_json_string(&create_test_pattern_json()).unwrap();
        for o in 0..3000 {
            if o == warmup_iterations {
                println!("Warmup done, starting benchmark..");
                max_time = Duration::default();
            }
            let now = Instant::now();

            let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default(), geometric_transform: Default::default() };
            let mut last_nep = NextEvalParams { last_eval_pattern_time: 0.0, time_offset: 0.0 };
            for i in 0..200 {
                let time = f64::from(i) * 0.05;
                pep.time = time;
                let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
                if eval_result.ul_control_point.coords.z != 0.0 {
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

    //#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    #[test]
    #[ignore="bench"]
    fn benchwasm() {
        let warmup_iterations = 5;
        let pe = PatternEvaluator::new_from_json_string(&create_test_pattern_json()).unwrap();
        for o in 0..3000 {
            if o == warmup_iterations {
                println!("Warmup done, starting benchmark..");
            }

            let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default(), geometric_transform: Default::default() };
            let mut last_nep = NextEvalParams { last_eval_pattern_time: 0.0, time_offset: 0.0 };
            for i in 0..200 {
                let time = f64::from(i) * 0.05;
                pep.time = time;
                let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
                if eval_result.ul_control_point.coords.z != 0.0 {
                    println!("{:?}", eval_result);
                }
                last_nep = eval_result.next_eval_params;
            }
        }
    }
}