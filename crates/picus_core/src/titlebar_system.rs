use bevy_app::AppExit;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::*;
use bevy_window::{
    MonitorSelection, PrimaryWindow, Window, WindowMode,
};

use crate::events::UiEventQueue;
use crate::{TitleBarAction, TitleBarState};

/// Handle title bar actions emitted by window control buttons
/// (minimize, maximize, close, fullscreen).
pub fn handle_titlebar_actions(
    mut actions: ResMut<UiEventQueue>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    #[allow(unused_variables)] mut app_exit_writer: MessageWriter<AppExit>,
) {
    let Some(mut window) = window_query.iter_mut().next() else {
        return;
    };

    for event in actions.drain_actions::<TitleBarAction>() {
        match event.action {
            TitleBarAction::Minimize => {
                window.set_minimized(true);
            }
            TitleBarAction::Maximize => {
                window.set_maximized(true);
            }
            TitleBarAction::Restore => {
                window.set_maximized(false);
            }
            TitleBarAction::Close => {
                app_exit_writer.write(AppExit::Success);
            }
            TitleBarAction::FullScreen => {
                window.mode = match window.mode {
                    WindowMode::Windowed => {
                        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                    }
                    _ => WindowMode::Windowed,
                };
            }
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
