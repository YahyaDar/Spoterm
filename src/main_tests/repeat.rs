//! `R`, repeat (#68): off → all → one like Spotify's button, said on the
//! status line, and kept for the next load even when the player can't be told.
//!
//! Needs `--features test-support` (CI's `--all-features` has it).

#![cfg(feature = "test-support")]

use super::sizes::{chans, test_app};
use crate::*;

fn press_r(app: &mut App, mods: KeyModifiers) {
    handle_key(app, KeyCode::Char('R'), mods, &chans());
}

#[test]
fn r_steps_off_all_one_and_back() {
    let mut app = test_app();
    assert_eq!(app.transport.repeat, Repeat::Off);
    // Shift+R as legacy terminals and Windows report it: the capital, with
    // SHIFT set. Caps Lock + r arrives as the bare capital.
    for (mods, want) in [
        (KeyModifiers::SHIFT, Repeat::All),
        (KeyModifiers::empty(), Repeat::One),
        (KeyModifiers::SHIFT, Repeat::Off),
    ] {
        press_r(&mut app, mods);
        assert_eq!(app.transport.repeat, want);
    }
}

#[test]
fn r_says_what_repeat_is_now() {
    let mut app = test_app();
    for want in [Repeat::All, Repeat::One, Repeat::Off] {
        app.status.clear();
        press_r(&mut app, KeyModifiers::SHIFT);
        assert!(
            app.status.starts_with(&format!("repeat {}", want.label())),
            "{want:?}: {:?}",
            app.status
        );
    }
}

/// A detached engine has no player to tell — like a dropped connection. The
/// failure is shown, not swallowed, and the mode still holds for the next load.
#[test]
fn r_shows_why_the_player_did_not_get_it() {
    let mut app = test_app();
    press_r(&mut app, KeyModifiers::SHIFT);
    assert_eq!(app.transport.repeat, Repeat::All);
    assert!(app.status.contains("not connected"), "{:?}", app.status);
}

#[test]
fn r_works_with_the_library_hidden() {
    let mut app = test_app();
    app.view.zen = true;
    app.settle_view();
    press_r(&mut app, KeyModifiers::SHIFT);
    assert_eq!(app.transport.repeat, Repeat::All);
}

#[test]
fn each_repeat_status_names_its_mode() {
    for mode in [Repeat::Off, Repeat::All, Repeat::One] {
        assert!(repeat_status(mode).starts_with(&format!("repeat {}", mode.label())));
    }
}

/// Skipping updates `transport.repeat` immediately (even on a detached engine;
/// rolling back on a failed `Engine::next()` is deferred to the `EffectOutcome`
/// migration).
fn assert_skip_updates_repeat(label: &str, trigger: impl Fn(&mut App, &UiChannels)) {
    let chans = chans();
    for (before, after) in [
        (Repeat::One, Repeat::All),
        (Repeat::All, Repeat::All),
        (Repeat::Off, Repeat::Off),
    ] {
        let mut app = test_app();
        app.transport.repeat = before;
        trigger(&mut app, &chans);
        assert_eq!(app.transport.repeat, after, "{label} from {before:?}");
    }
}

#[test]
fn skip_key_ends_repeat_one_and_keeps_list_repeating() {
    for key in [KeyCode::Char('n'), KeyCode::Media(MediaKeyCode::TrackNext)] {
        assert_skip_updates_repeat(&format!("{key:?}"), |app, chans| {
            handle_key(app, key, KeyModifiers::empty(), chans);
        });
    }
}

#[test]
fn media_next_ends_repeat_one_and_keeps_list_repeating() {
    assert_skip_updates_repeat("MediaControlEvent::Next", |app, chans| {
        handle_media_control_event(app, MediaControlEvent::Next, &chans.radio);
    });
}

#[test]
fn first_playing_event_restores_transport_repeat() {
    let chans = chans();
    let mut app = test_app();
    app.transport.repeat = Repeat::One;
    handle_engine_event(
        &mut app,
        EngineEvent::Playing {
            uri: "spotify:track:1".into(),
            position_ms: 0,
        },
        &chans.meta,
    );
    assert!(
        app.status.contains("couldn't restore repeat one"),
        "{:?}",
        app.status
    );
}

#[test]
fn saved_state_captures_transport_repeat_as_compatible_flags() {
    for (mode, want_repeat, want_repeat_track) in [
        (Repeat::Off, false, false),
        (Repeat::All, true, false),
        (Repeat::One, true, true),
    ] {
        let mut app = test_app();
        app.transport.repeat = mode;
        let saved = SavedState::from_app(&app);
        assert_eq!(saved.repeat, want_repeat, "{mode:?}");
        assert_eq!(saved.repeat_track, want_repeat_track, "{mode:?}");
        assert_eq!(saved.repeat_mode(), mode, "{mode:?}");
    }
}
