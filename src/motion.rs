//! Time-based UI transitions, independent of the window and frame rate.

use std::time::Duration;

/// A retargetable scalar. Updating data during a transition must not restart it.
pub struct Transition {
    from: f32,
    to: f32,
    started: Duration,
    duration: Duration,
}

impl Transition {
    pub fn new(value: f32) -> Self {
        Self { from: value, to: value, started: Duration::ZERO, duration: Duration::ZERO }
    }

    pub fn retarget(&mut self, target: f32, now: Duration, duration: Duration) {
        if self.to == target { return; }
        self.from = self.value(now);
        self.to = target;
        self.started = now;
        self.duration = duration;
    }

    pub fn value(&self, now: Duration) -> f32 {
        if !self.running(now) { return self.to; }
        let progress = now.saturating_sub(self.started).as_secs_f32() / self.duration.as_secs_f32();
        self.from + (self.to - self.from) * ease(progress)
    }

    pub fn running(&self, now: Duration) -> bool {
        self.from != self.to && now.saturating_sub(self.started) < self.duration
    }
}

/// CSS cubic-bezier(0.4, 0, 0.2, 1), the legacy Material standard curve.
pub fn ease(progress: f32) -> f32 {
    let progress = progress.clamp(0., 1.);
    if progress == 0. || progress == 1. { return progress; }
    let (mut low, mut high) = (0., 1.);
    for _ in 0..20 {
        let t = (low + high) * 0.5;
        let x = 3. * (1. - t) * (1. - t) * t * 0.4 + 3. * (1. - t) * t * t * 0.2 + t * t * t;
        if x < progress { low = t; } else { high = t; }
    }
    let t = (low + high) * 0.5;
    3. * (1. - t) * t * t + t * t * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn repeated_renders_do_not_restart_a_transition() {
        let mut motion = Transition::new(0.);
        let duration = Duration::from_millis(200);
        motion.retarget(1., Duration::ZERO, duration);
        let halfway = motion.value(Duration::from_millis(100));
        assert!(halfway > 0. && halfway < 1.);
        motion.retarget(1., Duration::from_millis(100), duration);
        assert_eq!(motion.value(Duration::from_millis(200)), 1.);
        assert!(!motion.running(Duration::from_millis(200)));
    }

    #[test]
    fn reversing_keeps_the_current_position_and_finishes_exactly() {
        let mut motion = Transition::new(0.);
        let duration = Duration::from_millis(200);
        motion.retarget(1., Duration::ZERO, duration);
        let now = Duration::from_millis(80);
        let before = motion.value(now);
        motion.retarget(0., now, duration);
        assert_eq!(motion.value(now), before);
        assert!(motion.value(Duration::from_millis(120)) < before);
        assert_eq!(motion.value(Duration::from_secs(1)), 0.);
    }

    #[test]
    fn reduced_motion_reaches_the_target_without_requesting_more_frames() {
        let mut motion = Transition::new(0.);
        motion.retarget(1., Duration::ZERO, Duration::ZERO);
        assert_eq!(motion.value(Duration::ZERO), 1.);
        assert!(!motion.running(Duration::ZERO));
    }
}
