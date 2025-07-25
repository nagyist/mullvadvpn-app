use nix::{
    sys::time::TimeSpec,
    time::{ClockId, clock_gettime},
};
use std::{ffi::c_long, time::Duration};

const NSEC_PER_SEC: c_long = 1_000_000_000;

#[cfg(any(target_os = "macos", target_os = "ios"))]
const CLOCK_ID: ClockId = ClockId::CLOCK_MONOTONIC;

#[cfg(any(target_os = "linux", target_os = "android"))]
const CLOCK_ID: ClockId = ClockId::CLOCK_BOOTTIME;

#[derive(Clone, Copy)]
pub struct Instant {
    t: TimeSpec,
}

impl Instant {
    pub fn now() -> Self {
        Self { t: now() }
    }

    fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        // Assumptions:
        // * `tv_sec >= 0`
        // * `tv_nsec < NSEC_PER_SEC`

        let (tv_sec, tv_nsec) = if self.t.tv_nsec() < earlier.t.tv_nsec() {
            (
                self.t.tv_sec() - earlier.t.tv_sec() - 1,
                NSEC_PER_SEC - earlier.t.tv_nsec() + self.t.tv_nsec(),
            )
        } else {
            (
                self.t.tv_sec() - earlier.t.tv_sec(),
                self.t.tv_nsec() - earlier.t.tv_nsec(),
            )
        };

        if tv_sec < 0 {
            return None;
        }

        Some(Duration::new(tv_sec as _, tv_nsec as _))
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier)
            .unwrap_or(Duration::ZERO)
    }
}

fn now() -> TimeSpec {
    clock_gettime(CLOCK_ID).expect("Clock ID is valid")
}
