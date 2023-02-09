mod shared_types;
use std::{collections::HashMap, mem::Discriminant};

use shared_types::*;


pub struct PatternEvaluator {
    mah_animation: MidAirHapticsAnimationFileFormat,
}
#[derive(Debug, Clone)]
pub struct PatternEvaluatorParameters {
    pub time: f64,
    pub user_parameters: HashMap<String, f64>,
}

impl PatternEvaluator {
    pub fn new(mah_animation: MidAirHapticsAnimationFileFormat) -> Self {
        let mut mah_animation = mah_animation;
        mah_animation.keyframes.sort_by(|a, b| a.time().total_cmp(b.time()));

        Self {
            mah_animation,
        }
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
    fn perform_transition_interp(p: &PatternEvaluatorParameters, prev_time: f64, next_time: f64, transition: &MAHTransition) -> (f64, f64) {
        let dt = (p.time - prev_time) / (next_time - prev_time);
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

    fn eval_intensity(p: &PatternEvaluatorParameters, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> f64 {
        let prev_intensity = prev_kfc.intensity.as_ref();
        let next_intensity = next_kfc.intensity.as_ref();

        fn get_intensity_value(intensity: &MAHIntensity) -> f64 {
            match &intensity {
                MAHIntensity::Constant { value } => *value,
                MAHIntensity::Random { min, max } => rand::random::<f64>() * (*max - *min) + *min,
            }
        }

        if let (Some(prev_intensity), Some(next_intensity)) = (prev_intensity, next_intensity) {
            let piv = get_intensity_value(&prev_intensity.pwt.intensity);
            let niv = get_intensity_value(&next_intensity.pwt.intensity);
            let (pf, nf) = Self::perform_transition_interp(p, prev_intensity.time, next_intensity.time, &prev_intensity.pwt.transition);
            return pf * piv + nf * niv;
        } else if let Some(prev_intensity) = prev_intensity {
            return get_intensity_value(&prev_intensity.pwt.intensity);
        } else {
            return 1.0;
        }
    }

    fn eval_coords(p: &PatternEvaluatorParameters, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> MAHCoords {
        let prev_coords_att = prev_kfc.coords.as_ref();
        let next_coords_att = next_kfc.coords.as_ref();
        let next_keyframe = next_kfc.keyframe.as_ref();
        if let (Some(prev_coords_att), Some(next_coords_att), Some(next_keyframe)) = (prev_coords_att, next_coords_att, next_keyframe) {
            return match next_keyframe {
                MAHKeyframe::Pause(_) => { prev_coords_att.pwt.coords.clone() },
                MAHKeyframe::Standard(_) => {
                    let (pf, nf) = Self::perform_transition_interp(p, prev_coords_att.time, next_coords_att.time, &prev_coords_att.pwt.transition);
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

    fn coords_convert_to_hapev2(coords: &MAHCoords) -> MAHCoords {
        MAHCoords {
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

    fn eval_brush_hapev2(p: &PatternEvaluatorParameters, prev_kfc: &MAHKeyframeConfig, next_kfc: &MAHKeyframeConfig) -> BrushEvalParams {
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
                    let (pf, nf) = Self::perform_transition_interp(p, prev_brush.time, next_brush.time, &prev_brush.pwt.transition);
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
    fn eval_hapev2_primitive_equation(bp: &HapeV2PrimitiveParams, time: f64) -> MAHCoords {
        if bp.k != 0.0 { panic!("not yet implemented"); }
        let brush_t_rads = Self::time_to_hapev2_brush_rads(bp, time);
        MAHCoords {
            x: bp.A * (bp.a * brush_t_rads + bp.d).sin(),
            y: bp.B * (bp.b * brush_t_rads).sin(),
            z: 0.0,
        }
    }

    fn eval_hapev2_primitive_into_mah_units(p: &PatternEvaluatorParameters, brush_eval: &BrushEvalParams) -> MAHCoords {
        let brush_coords = Self::eval_hapev2_primitive_equation(&brush_eval.primitive_params, p.time);
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

    pub fn eval_path_at_anim_local_time(&self, p: &PatternEvaluatorParameters) -> PathAtAnimLocalTime {
        let prev_kfc = self.get_prev_kf_config(p.time);
        let next_kfc = self.get_next_kf_config(p.time);

        let coords = Self::eval_coords(&p, &prev_kfc, &next_kfc);
        let intensity = Self::eval_intensity(&p, &prev_kfc, &next_kfc);
        let brush = Self::eval_brush_hapev2(&p, &prev_kfc, &next_kfc);

        PathAtAnimLocalTime { coords, intensity, brush }
    }


    pub fn eval_brush_at_anim_local_time(&self, p: &PatternEvaluatorParameters) -> BrushAtAnimLocalTime {
        let pattern_time = p.time;
        let path_eval = self.eval_path_at_anim_local_time(p);

        let brush_coords_offset = Self::eval_hapev2_primitive_into_mah_units(&p, &path_eval.brush);
        BrushAtAnimLocalTime {
            coords: MAHCoords {
                x: path_eval.coords.x + brush_coords_offset.x,
                y: path_eval.coords.y + brush_coords_offset.y,
                z: path_eval.coords.z,
            },
            intensity: path_eval.intensity,
            pattern_time,
        }
    }

    pub fn eval_brush_at_anim_local_time_for_max_t(&self, p: &PatternEvaluatorParameters) -> Vec<BrushAtAnimLocalTime> {
        let max_number_of_points = 200;
        let device_frequency = 20000; //20khz

        let path_eval_base = self.eval_path_at_anim_local_time(&p);

        let bp = &path_eval_base.brush.primitive_params;
        let max_t_in_ms = (1000.0 * bp.max_t / (bp.draw_frequency * 2.0 * std::f64::consts::PI)) as f64; //solve `time / 1000 * draw_frequency * 2Pi = max_t` equation for time

        let device_step = 1000.0 / device_frequency as f64;
        let min_step = max_t_in_ms / max_number_of_points as f64;
        // if (min_step > device_step) console.warn("min_step > device_step");

        let mut evals = vec![];
        let mut i = 0.0;
        while i < max_t_in_ms {
            let step_p = PatternEvaluatorParameters { time: p.time + (i as f64), ..p.clone() };
            evals.push(self.eval_brush_at_anim_local_time(&step_p));
            i += f64::max(device_step, min_step);
        }

        evals
    }

}


#[derive(Debug, Clone)]
struct PrimitiveWithTransitionAtTime<'a, T> {
    time: MAHTime,
    // primitve with transition
    pwt: &'a T,
}
#[derive(Debug, Clone, Default)]
pub struct MAHKeyframeConfig<'a> {
    coords: Option<PrimitiveWithTransitionAtTime<'a, CoordsWithTransition>>,
    brush: Option<PrimitiveWithTransitionAtTime<'a, BrushWithTransition>>,
    intensity: Option<PrimitiveWithTransitionAtTime<'a, IntensityWithTransition>>,
    keyframe: Option<MAHKeyframe>,
}

#[derive(Debug, Clone)]
pub struct PathAtAnimLocalTime {
    pub coords: MAHCoords,
    pub intensity: f64,
    brush: BrushEvalParams,
}
#[derive(Debug, Clone)]
pub struct BrushAtAnimLocalTime {
    pub coords: MAHCoords,
    pub intensity: f64,
    pub pattern_time: f64,
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