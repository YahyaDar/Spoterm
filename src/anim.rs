//! Time-based animation helpers.
//!
//! Right now this drives one thing: the theme cross-fade on track change. The
//! design is deliberately render-clock-independent — `ThemeFade` reads wall-clock
//! elapsed, so the fade lasts the same real duration whether the UI redraws at
//! 60fps or 10fps.

use std::time::{Duration, Instant};

use crate::theme::Theme;

/// Cubic ease-in-out — slow at the ends, quick through the middle. Makes the
/// recolor feel like it *breathes* instead of ramping linearly.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = -2.0 * t + 2.0;
        1.0 - (f * f * f) / 2.0
    }
}

/// A running cross-fade between two full themes.
#[derive(Debug, Clone)]
pub struct ThemeFade {
    from: Theme,
    to: Theme,
    start: Instant,
    duration: Duration,
}

impl ThemeFade {
    pub fn new(from: Theme, to: Theme, duration: Duration) -> Self {
        Self::new_at(from, to, duration, Instant::now())
    }

    pub fn new_at(from: Theme, to: Theme, duration: Duration, now: Instant) -> Self {
        Self {
            from,
            to,
            start: now,
            duration,
        }
    }

    /// Linear progress in `[0, 1]` at `now`.
    pub fn progress_at(&self, now: Instant) -> f32 {
        let d = self.duration.as_secs_f32();
        if d <= 0.0 {
            return 1.0;
        }
        (now.saturating_duration_since(self.start).as_secs_f32() / d).clamp(0.0, 1.0)
    }

    /// Linear progress in `[0, 1]` based on wall-clock elapsed.
    pub fn progress(&self) -> f32 {
        self.progress_at(Instant::now())
    }

    pub fn is_done_at(&self, now: Instant) -> bool {
        self.progress_at(now) >= 1.0
    }

    pub fn is_done(&self) -> bool {
        self.is_done_at(Instant::now())
    }

    /// The interpolated theme at `now`, with easing applied.
    pub fn current_at(&self, now: Instant) -> Theme {
        Theme::lerp(
            &self.from,
            &self.to,
            ease_in_out_cubic(self.progress_at(now)),
        )
    }

    /// The interpolated theme for the current instant, with easing applied.
    pub fn current(&self) -> Theme {
        self.current_at(Instant::now())
    }

    /// The theme we're fading toward (used to snap cleanly when done).
    pub fn target(&self) -> Theme {
        self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_endpoints() {
        assert!((ease_in_out_cubic(0.0)).abs() < 1e-6);
        assert!((ease_in_out_cubic(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn fade_starts_at_from_ends_at_to() {
        let a = crate::theme::TOKYONIGHT;
        let b = crate::theme::GRUVBOX;
        // A zero-duration fade is immediately done and equals the target.
        let f = ThemeFade::new(a, b, Duration::from_millis(0));
        assert!(f.is_done());
        let cur = f.current();
        assert_eq!(cur.background.r, b.background.r);
    }

    #[test]
    fn explicit_clock_fade_advances_clamps_and_completes_deterministically() {
        use crate::gradient::Rgb;

        let t0 = Instant::now();
        let dur = Duration::from_millis(1000);
        let from = Theme {
            primary: Rgb::new(0, 0, 0),
            ..crate::theme::TOKYONIGHT
        };
        let to = Theme {
            primary: Rgb::new(100, 200, 50),
            ..crate::theme::GRUVBOX
        };
        assert_ne!(from, to);

        let fade = ThemeFade::new_at(from, to, dur, t0);

        // Start instant: progress 0, not done, colors match `from`.
        assert_eq!(fade.progress_at(t0), 0.0);
        assert!(!fade.is_done_at(t0));
        assert_eq!(fade.current_at(t0).primary, Rgb::new(0, 0, 0));

        // Non-monotonic clock (`now < start`): saturates to 0 without panicking.
        if let Some(before) = t0.checked_sub(Duration::from_millis(500)) {
            assert_eq!(fade.progress_at(before), 0.0);
            assert!(!fade.is_done_at(before));
            assert_eq!(fade.current_at(before).primary, Rgb::new(0, 0, 0));
        }

        // Midpoint: linear progress 0.5, ease_in_out_cubic(0.5) == 0.5 -> exact midpoint RGB.
        let mid = t0 + Duration::from_millis(500);
        assert!((fade.progress_at(mid) - 0.5).abs() < 1e-6);
        assert!(!fade.is_done_at(mid));
        assert_eq!(fade.current_at(mid).primary, Rgb::new(50, 100, 25));

        // End and past-end: done and equal to target Theme.
        let end = t0 + dur;
        assert_eq!(fade.progress_at(end), 1.0);
        assert!(fade.is_done_at(end));
        assert_eq!(fade.current_at(end), to);

        let after = end + Duration::from_secs(5);
        assert_eq!(fade.progress_at(after), 1.0);
        assert!(fade.is_done_at(after));
        assert_eq!(fade.current_at(after), to);
    }

    #[test]
    fn zero_duration_fade_is_immediately_done_at_explicit_clock() {
        let t0 = Instant::now();
        let f = ThemeFade::new_at(
            crate::theme::TOKYONIGHT,
            crate::theme::GRUVBOX,
            Duration::ZERO,
            t0,
        );
        assert_eq!(f.progress_at(t0), 1.0);
        assert!(f.is_done_at(t0));
        assert_eq!(f.current_at(t0), crate::theme::GRUVBOX);
    }
}
