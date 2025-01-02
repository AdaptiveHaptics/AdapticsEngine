/* eslint-disable */
/**
 * This file was automatically generated by json-schema-to-typescript.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run json-schema-to-typescript to regenerate this file.
 */

/**
 * Messages sent to websocket clients
 */
export type AdapticsWSServerMessage =
  | {
      cmd: "playback_update";
      data: {
        evals: BrushAtAnimLocalTime[];
      };
    }
  | {
      cmd: "tracking_data";
      data: {
        tracking_frame: TrackingFrame;
      };
    };
export type TrackingFrameHandChirality = "Right" | "Left";

export interface BrushAtAnimLocalTime {
  next_eval_params: NextEvalParams;
  pattern_time: number;
  stop: boolean;
  ul_control_point: UltraleapControlPoint;
}
export interface NextEvalParams {
  last_eval_pattern_time: number;
  time_offset: number;
}
export interface UltraleapControlPoint {
  coords: MAHCoordsConst;
  intensity: number;
}
/**
 * Coordinates in millimeters.
 *
 * x and y are used for the xy coordinate system in the 2d designer.
 *
 * z is intended to be orthogonal to the phased array.
 */
export interface MAHCoordsConst {
  /**
   * in millimeters, [-100, 100]
   */
  x: number;
  /**
   * in millimeters, [-100, 100]
   */
  y: number;
  /**
   * in millimeters, [0, 100]
   */
  z: number;
}
export interface TrackingFrame {
  hand?: TrackingFrameHand | null;
}
export interface TrackingFrameHand {
  chirality: TrackingFrameHandChirality;
  /**
   * @minItems 5
   * @maxItems 5
   */
  digits: [TrackingFrameDigit, TrackingFrameDigit, TrackingFrameDigit, TrackingFrameDigit, TrackingFrameDigit];
  palm: TrackingFramePalm;
}
export interface TrackingFrameDigit {
  /**
   * @minItems 4
   * @maxItems 4
   */
  bones: [TrackingFrameBone, TrackingFrameBone, TrackingFrameBone, TrackingFrameBone];
}
export interface TrackingFrameBone {
  end: MAHCoordsConst;
  start: MAHCoordsConst;
  /**
   * The average width of the flesh around the bone in millimeters.
   */
  width: number;
}
export interface TrackingFramePalm {
  /**
   * The unit direction vector pointing from the palm position toward the fingers.
   */
  direction: MAHCoordsConst;
  /**
   * If your hand is flat, this vector will point downward, or "out" of the front surface of your palm.
   */
  normal: MAHCoordsConst;
  /**
   * The center position of the palm
   */
  position: MAHCoordsConst;
  /**
   * The estimated width of the palm when the hand is in a flat position.
   */
  width: number;
}
