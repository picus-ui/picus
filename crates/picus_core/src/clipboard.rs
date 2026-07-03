//! System clipboard integration for picus.
//!
//! Provides a [`Clipboard`] resource that wraps [`arboard::Clipboard`],
//! plus ECS components and systems for handling clipboard events in
//! a Bevy ECS application.

use std::sync::Mutex;

use arboard::Clipboard as ArboardClipboard;
use bevy_ecs::prelude::*;

/// The type of clipboard operation requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    /// Copy the selected text to the system clipboard.
    Copy,
    /// Copy the selected text to the system clipboard and remove it.
    Cut,
    /// Insert the system clipboard text at the current cursor position.
    Paste,
}

/// Component attached to entities that should receive a clipboard operation.
///
/// The system [`handle_clipboard_events`] reads this component, performs the
/// requested operation, attaches [`ClipboardText`] for Paste results, and
/// then removes the component.
#[derive(Component, Debug, Clone)]
pub struct ClipboardEvent {
    /// The kind of clipboard operation.
    pub kind: ClipboardKind,
    /// Optional text payload (used by Copy / Cut).
    pub text: Option<String>,
}

/// Component that stores clipboard text content on an entity.
///
/// Attached automatically by [`handle_clipboard_events`] when a
/// [`ClipboardKind::Paste`] event is processed.
#[derive(Component, Debug, Clone, Default)]
pub struct ClipboardText {
    /// The clipboard text content.
    pub text: String,
}

/// Global system clipboard resource.
///
/// Wraps [`arboard::Clipboard`] behind a [`Mutex`] to allow safe concurrent
/// access from ECS systems. Initialization silently falls back to
/// `None` when the system clipboard is unavailable (e.g. headless
/// environments).
#[derive(Resource)]
pub struct Clipboard {
    inner: Mutex<Option<ArboardClipboard>>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            inner: Mutex::new(ArboardClipboard::new().ok()),
        }
    }
}

impl Clipboard {
    /// Read the current text from the system clipboard, if available.
    pub fn get_text(&self) -> Option<String> {
        let mut guard = self.inner.lock().ok()?;
        guard.as_mut()?.get_text().ok()
    }

    /// Write text to the system clipboard.
    pub fn set_text(&self, text: &str) {
        #[allow(clippy::collapsible_if)]
        {
            if let Ok(mut guard) = self.inner.lock() {
                if let Some(clip) = guard.as_mut() {
                    let _ = clip.set_text(text);
                }
            }
        }
    }

    /// Returns `true` if the system clipboard is accessible.
    pub fn is_available(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }
}

/// System that processes [`ClipboardEvent`] components.
///
/// Should be scheduled in `PreUpdate` so clipboard operations are handled
/// before other frame logic.
///
/// - `Copy` / `Cut`: writes the event's text payload to the system clipboard.
/// - `Paste`: reads the system clipboard and attaches a [`ClipboardText`]
///   component to the target entity.
///
/// In all cases the [`ClipboardEvent`] component is removed after processing.
pub fn handle_clipboard_events(
    mut commands: Commands,
    clipboard: Res<Clipboard>,
    query: Query<(Entity, &ClipboardEvent)>,
) {
    for (entity, event) in query.iter() {
        match event.kind {
            ClipboardKind::Copy | ClipboardKind::Cut => {
                if let Some(ref text) = event.text {
                    clipboard.set_text(text);
                }
            }
            ClipboardKind::Paste => {
                if let Some(text) = clipboard.get_text() {
                    commands.entity(entity).insert(ClipboardText { text });
                }
            }
        }
        commands.entity(entity).remove::<ClipboardEvent>();
    }
}
