use crate::*;

#[test]
fn a_cover_palette_keeps_the_configured_transparent_background() {
    // `derive_theme` knows nothing of the config and always comes back opaque;
    // the flag set at startup has to survive every track change.
    let t0 = Instant::now();
    let transparent = Theme {
        transparent: true,
        ..TOKYONIGHT
    };
    let mut state = ThemeState {
        displayed: transparent,
        target: transparent,
        fade: None,
        fixed: false,
        notice: None,
    };

    state.start_fade(myx::theme::GRUVBOX, t0);

    assert!(state.target.transparent);
    assert!(state
        .fade
        .as_ref()
        .is_some_and(|f| f.current_at(t0).transparent));
}

#[test]
fn flipping_transparency_mid_fade_sticks_when_the_fade_lands() {
    // The running fade still carries the old flag; without restarting it, the
    // next frame would paint the background right back.
    let t0 = Instant::now();
    let mut state = ThemeState {
        displayed: TOKYONIGHT,
        target: TOKYONIGHT,
        fade: None,
        fixed: false,
        notice: None,
    };
    state.start_fade(myx::theme::GRUVBOX, t0);

    let t1 = t0 + Duration::from_millis(500);
    state.set_transparent(true, t1);

    assert!(state.displayed.transparent);
    assert!(state
        .fade
        .as_ref()
        .is_some_and(|f| f.current_at(t1).transparent && f.target().transparent));

    // Completing the restarted fade snaps cleanly with transparency preserved.
    state.advance(t1 + Duration::from_millis(FADE_MS));
    assert!(state.fade.is_none());
    assert_eq!(
        state.displayed,
        Theme {
            transparent: true,
            ..myx::theme::GRUVBOX
        }
    );
}

#[test]
fn a_fixed_theme_ignores_covers_and_the_album_theme_follows_them() {
    let t0 = Instant::now();
    let mut fixed = ThemeState {
        displayed: TOKYONIGHT,
        target: TOKYONIGHT,
        fade: None,
        fixed: true,
        notice: None,
    };
    fixed.follow_cover(myx::theme::GRUVBOX, t0);
    assert!(fixed.fade.is_none());
    assert_eq!(fixed.target.name, "tokyonight");

    let mut album = ThemeState {
        fixed: false,
        ..fixed
    };
    album.follow_cover(myx::theme::GRUVBOX, t0);
    assert!(album.fade.is_some());
    assert_eq!(album.target.name, "gruvbox");
}

#[test]
fn startup_theme_state_builds_from_resolved_choice_and_formats_warnings() {
    let album = myx::user_theme::Resolved {
        choice: myx::user_theme::Choice::Album,
        warnings: Vec::new(),
    };
    let state = ThemeState::from_resolved(&album, false);
    assert_eq!(state.displayed, TOKYONIGHT);
    assert_eq!(state.target, TOKYONIGHT);
    assert!(!state.fixed);
    assert_eq!(state.notice, None);

    let fixed_one_warn = myx::user_theme::Resolved {
        choice: myx::user_theme::Choice::Fixed(myx::theme::GRUVBOX),
        warnings: vec!["bad accent".to_string()],
    };
    let state = ThemeState::from_resolved(&fixed_one_warn, true);
    let want = Theme {
        transparent: true,
        ..myx::theme::GRUVBOX
    };
    assert_eq!(state.displayed, want);
    assert_eq!(state.target, want);
    assert!(state.fixed);
    assert_eq!(state.notice.as_deref(), Some("bad accent"));

    let multi_warn = myx::user_theme::Resolved {
        choice: myx::user_theme::Choice::Album,
        warnings: vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
    };
    let state = ThemeState::from_resolved(&multi_warn, false);
    assert_eq!(state.notice.as_deref(), Some("w1 (+2 more)"));
}

#[test]
fn theme_advance_interpolates_and_snaps_on_explicit_clock() {
    let t0 = Instant::now();
    let mut state = ThemeState {
        displayed: TOKYONIGHT,
        target: TOKYONIGHT,
        fade: None,
        fixed: false,
        notice: None,
    };
    state.follow_cover(myx::theme::GRUVBOX, t0);

    // Mid-fade: fade is still active and displayed has moved away from start.
    state.advance(t0 + Duration::from_millis(FADE_MS / 2));
    assert!(state.fade.is_some());
    assert_ne!(state.displayed, TOKYONIGHT);
    assert_ne!(state.displayed, myx::theme::GRUVBOX);

    // Completion: snaps exactly to target and clears the fade.
    state.advance(t0 + Duration::from_millis(FADE_MS));
    assert!(state.fade.is_none());
    assert_eq!(state.displayed, myx::theme::GRUVBOX);
}
