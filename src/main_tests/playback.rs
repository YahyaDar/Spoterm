//! Deterministic playhead and scrub transitions on `PlaybackState`.

use crate::*;

fn playing_at(t0: Instant, position_ms: u32, duration_ms: u32) -> PlaybackState {
    PlaybackState {
        now: Some(NowPlaying {
            uri: "spotify:track:x".to_string(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            duration_ms,
            position_ms,
            position_at: t0,
            is_playing: true,
            cover: None,
        }),
        seek_target: None,
        seek_last_step: t0,
        seek_last_input: t0,
    }
}

#[test]
fn position_extrapolates_clamps_and_handles_non_monotonic_clock() {
    let t0 = Instant::now();
    let mut playback = playing_at(t0, 10_000, 20_000);

    assert_eq!(playback.position_ms(t0), 10_000);
    assert_eq!(playback.position_ms(t0 + Duration::from_secs(4)), 14_000);
    assert_eq!(playback.position_ms(t0 + Duration::from_secs(30)), 20_000);

    if let Some(before) = t0.checked_sub(Duration::from_secs(2)) {
        assert_eq!(playback.position_ms(before), 10_000);
    }

    // Paused playhead holds its reported position regardless of elapsed time.
    playback.now.as_mut().unwrap().is_playing = false;
    assert_eq!(playback.position_ms(t0 + Duration::from_secs(9)), 10_000);
}

#[test]
fn freezing_keeps_the_position_and_stops_the_clock() {
    let t0 = Instant::now();
    let mut playback = playing_at(t0, 10_000, 200_000);
    let t1 = t0 + Duration::from_secs(5);

    assert!(playback.freeze(t1));
    assert_eq!(playback.position_ms(t1), 15_000);
    // Stopped means stopped: time passing must not move it any further.
    assert_eq!(playback.position_ms(t1 + Duration::from_secs(30)), 15_000);
}

#[test]
fn freezing_again_reports_nothing_was_playing() {
    // The watchdog repeats `Reconnecting` for every failed attempt; only
    // the first can have interrupted audio.
    let t0 = Instant::now();
    let mut playback = playing_at(t0, 0, 200_000);
    assert!(playback.freeze(t0));
    assert!(!playback.freeze(t0));
}

#[test]
fn freezing_nothing_is_a_no_op() {
    let t0 = Instant::now();
    let mut playback = playing_at(t0, 0, 200_000);
    playback.now = None;
    assert!(!playback.freeze(t0));
}

#[cfg(feature = "test-support")]
#[test]
fn scrub_step_throttles_repeats_gates_engine_positions_and_flushes_after_settle() {
    let t0 = Instant::now();
    let engine = Engine::detached();
    let mut playback = playing_at(t0, 10_000, 200_000);

    // First scrub step sets target and updates local display.
    playback.seek_step(SEEK_STEP_MS, t0);
    assert_eq!(playback.seek_target, Some(15_000));
    assert_eq!(playback.position_ms(t0), 15_000);

    // Engine position reports are ignored mid-scrub.
    playback.set_local_position(10_500, true, t0 + Duration::from_millis(50));
    assert_eq!(playback.position_ms(t0 + Duration::from_millis(50)), 15_050);

    // Repeat within SEEK_REPEAT is throttled, while pushing out the settle timer.
    let t_throttled = t0 + Duration::from_millis(100);
    playback.seek_step(SEEK_STEP_MS, t_throttled);
    assert_eq!(playback.seek_target, Some(15_000));

    // Next step after SEEK_REPEAT advances from the active scrub target.
    let t_step2 = t0 + SEEK_REPEAT;
    playback.seek_step(SEEK_STEP_MS, t_step2);
    assert_eq!(playback.seek_target, Some(20_000));

    // Non-monotonic clock and pre-settle ticks do not flush early or panic.
    playback.flush_seek(&engine, t0);
    assert_eq!(playback.seek_target, Some(20_000));
    playback.flush_seek(&engine, t_step2 + Duration::from_millis(100));
    assert_eq!(playback.seek_target, Some(20_000));

    // Once SEEK_SETTLE has elapsed, flush commits and clears the scrub target.
    let t_flush = t_step2 + SEEK_SETTLE;
    playback.flush_seek(&engine, t_flush);
    assert_eq!(playback.seek_target, None);
    assert_eq!(playback.position_ms(t_flush), 20_000);

    // Subsequent engine position updates are accepted again.
    let t_after = t_flush + Duration::from_millis(100);
    playback.set_local_position(21_000, true, t_after);
    assert_eq!(playback.position_ms(t_after), 21_000);
}
