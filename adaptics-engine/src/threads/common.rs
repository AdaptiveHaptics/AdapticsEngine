use std::{time::{Duration, Instant}, ops::{Sub, Add}};

pub(super) type MilSec = f64;

pub(super) fn js_milliseconds_to_duration(ms: MilSec) -> Duration {
    assert!(!ms.is_sign_negative(), "js_milliseconds_to_duration: ms is negative");
    Duration::from_secs_f64(ms / 1000.0)
}
pub(super) fn instant_add_js_milliseconds(instant: Instant, ms: MilSec) -> Instant {
    if ms.is_sign_negative() {
        instant.sub(js_milliseconds_to_duration(-ms))
    } else {
        instant.add(js_milliseconds_to_duration(ms))
    }
}