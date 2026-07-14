use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_window::{MonitorSelection, PrimaryWindow, Window, WindowMode};

use crate::{TitleBarAction, TitleBarState};

/// Handle title bar actions emitted by window control buttons
/// (minimize, maximize, close, fullscreen).
pub(crate) fn apply_titlebar_action(world: &mut World, _source: Entity, action: &TitleBarAction) {
    if matches!(action, TitleBarAction::Close) {
        world.write_message(AppExit::Success);
        return;
    }

    let mut window_query = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    let Some(mut window) = window_query.iter_mut(world).next() else {
        return;
    };

    match action {
        TitleBarAction::Minimize => window.set_minimized(true),
        TitleBarAction::Maximize => window.set_maximized(true),
        TitleBarAction::Restore => window.set_maximized(false),
        TitleBarAction::Close => unreachable!("close is handled before borrowing the window"),
        TitleBarAction::FullScreen => {
            window.mode = match window.mode {
                WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                _ => WindowMode::Windowed,
            };
        }
    }
}

/// Sync the current window state into [`TitleBarState`] components every frame.
///
/// Note: Bevy 0.19 does not expose a maximized getter on `Window`, so
/// `is_maximized` is left at its existing value (defaults to `false`).
pub fn sync_titlebar_state(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut titlebar_query: Query<&mut TitleBarState>,
) {
    let Some(window) = window_query.iter().next() else {
        return;
    };

    for mut state in titlebar_query.iter_mut() {
        state.is_full_screen = matches!(
            window.mode,
            WindowMode::BorderlessFullscreen(_) | WindowMode::Fullscreen(..)
        );
    }
}
