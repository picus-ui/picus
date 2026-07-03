//! Keyboard accelerator / access key system for desktop keyboard shortcuts.
//!
//! Provides an ECS component-based system for binding keyboard shortcuts
//! (e.g. Ctrl+S, Ctrl+Shift+Z, Alt+F4) to entities. The system tracks
//! modifier key state and dispatches [`AcceleratorActivated`] events through
//! the global [`UiEventQueue`] when a matching accelerator is pressed.


use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{KeyCode, KeyboardInput};

use crate::events::UiEventQueue;

/// Modifier key flags for keyboard accelerators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AcceleratorModifiers {
    pub alt: bool,
    pub control: bool,
    pub shift: bool,
    pub super_key: bool,
}

/// Keyboard accelerator component.
///
/// Attach this to any entity to register a keyboard shortcut.
/// When the key+modifier combination is pressed, an [`AcceleratorActivated`]
/// event is pushed to the global [`UiEventQueue`] with this entity as the target.
#[derive(Component, Debug, Clone)]
pub struct KeyboardAccelerator {
    /// The key that triggers this accelerator.
    pub key: KeyCode,
    /// Required modifier keys.
    pub modifiers: AcceleratorModifiers,
    /// Whether this accelerator is currently active.
    pub enabled: bool,
}

impl KeyboardAccelerator {
    /// Create a new keyboard accelerator.
    #[must_use]
    pub fn new(key: KeyCode, modifiers: AcceleratorModifiers) -> Self {
        Self {
            key,
            modifiers,
            enabled: true,
        }
    }

    /// Convenience: Ctrl+{key}.
    #[must_use]
    pub fn ctrl(key: KeyCode) -> Self {
        Self::new(
            key,
            AcceleratorModifiers {
                control: true,
                ..AcceleratorModifiers::default()
            },
        )
    }

    /// Convenience: Ctrl+Shift+{key}.
    #[must_use]
    pub fn ctrl_shift(key: KeyCode) -> Self {
        Self::new(
            key,
            AcceleratorModifiers {
                control: true,
                shift: true,
                ..AcceleratorModifiers::default()
            },
        )
    }

    /// Convenience: Alt+{key}.
    #[must_use]
    pub fn alt(key: KeyCode) -> Self {
        Self::new(
            key,
            AcceleratorModifiers {
                alt: true,
                ..AcceleratorModifiers::default()
            },
        )
    }
}

/// Accelerator scope — controls when the accelerator is active.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcceleratorScope {
    /// Always active when the window has focus.
    #[default]
    Global,
    /// Only active when the window has focus.
    Window,
    /// Only active when the specific entity has focus.
    Focused,
}

/// Optional text override shown in menus for this accelerator
/// (e.g. "Ctrl+S", "Ctrl+Shift+Z").
#[derive(Component, Debug, Clone)]
pub struct AcceleratorTextOverride(pub String);

/// Event pushed to [`UiEventQueue`] when an accelerator is activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceleratorActivated {
    /// The physical key that triggered the accelerator.
    pub accelerator_key: KeyCode,
    /// The modifier state at activation time.
    pub modifiers: AcceleratorModifiers,
}

/// Tracks the currently held modifier keys.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CurrentAcceleratorModifiers(pub AcceleratorModifiers);

/// System that processes keyboard input, tracks modifier state,
/// and dispatches [`AcceleratorActivated`] events for matching accelerators.
pub fn process_keyboard_accelerators(
    mut key_events: MessageReader<KeyboardInput>,
    accelerator_query: Query<(Entity, &KeyboardAccelerator)>,
    event_queue: Res<UiEventQueue>,
    mut current_modifiers: ResMut<CurrentAcceleratorModifiers>,
) {
    for event in key_events.read() {
        match event.key_code {
            KeyCode::ControlLeft | KeyCode::ControlRight => {
                current_modifiers.0.control = event.state == bevy_input::ButtonState::Pressed;
            }
            KeyCode::AltLeft | KeyCode::AltRight => {
                current_modifiers.0.alt = event.state == bevy_input::ButtonState::Pressed;
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                current_modifiers.0.shift = event.state == bevy_input::ButtonState::Pressed;
            }
            KeyCode::SuperLeft | KeyCode::SuperRight => {
                current_modifiers.0.super_key = event.state == bevy_input::ButtonState::Pressed;
            }
            _ => {}
        }

        // Only fire on key press, not release.
        if event.state != bevy_input::ButtonState::Pressed {
            continue;
        }
        let key_code = event.key_code;

        for (entity, accelerator) in accelerator_query.iter() {
            if !accelerator.enabled {
                continue;
            }
            if accelerator.key != key_code {
                continue;
            }
            if accelerator.modifiers != current_modifiers.0 {
                continue;
            }

            event_queue.push_typed(
                entity,
                AcceleratorActivated {
                    accelerator_key: key_code,
                    modifiers: current_modifiers.0,
                },
            );
        }
    }
}

/// Helper to build a human-readable accelerator text
/// (e.g. "Ctrl+Shift+S" from KeyCode::KeyS + control+shift).
#[must_use]
pub fn format_accelerator_text(key: KeyCode, modifiers: &AcceleratorModifiers) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if modifiers.control {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.super_key {
        parts.push("Win");
    }

    let key_name = format!("{key:?}")
        .trim_start_matches("Key")
        .trim_start_matches("Digit")
        .trim_start_matches("Numpad")
        .to_string();
    parts.push(&key_name);

    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerator_format_ctrl_s() {
        let text = format_accelerator_text(
            KeyCode::KeyS,
            &AcceleratorModifiers {
                control: true,
                ..AcceleratorModifiers::default()
            },
        );
        assert_eq!(text, "Ctrl+S");
    }

    #[test]
    fn accelerator_format_ctrl_shift_z() {
        let text = format_accelerator_text(
            KeyCode::KeyZ,
            &AcceleratorModifiers {
                control: true,
                shift: true,
                ..AcceleratorModifiers::default()
            },
        );
        assert_eq!(text, "Ctrl+Shift+Z");
    }

    #[test]
    fn accelerator_equality() {
        let a = KeyboardAccelerator::ctrl(KeyCode::KeyS);
        let b = KeyboardAccelerator::ctrl(KeyCode::KeyS);
        assert_eq!(a.key, b.key);
        assert_eq!(a.modifiers, b.modifiers);
    }
}




