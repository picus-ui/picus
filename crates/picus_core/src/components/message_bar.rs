use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// Severity level for a message bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageBarKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A message/alert bar with severity styling (Fluent v9 MessageBar).
///
/// Displays a colored banner with an icon, message text, and optional dismiss button.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiMessageBar {
    /// The message text content.
    pub message: String,
    /// Severity kind controlling the bar's colour.
    pub kind: MessageBarKind,
    /// Whether a dismiss button is shown.
    pub dismissible: bool,
}

impl UiMessageBar {
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MessageBarKind::Info,
            dismissible: true,
        }
    }

    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MessageBarKind::Success,
            dismissible: true,
        }
    }

    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MessageBarKind::Warning,
            dismissible: true,
        }
    }

    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MessageBarKind::Error,
            dismissible: true,
        }
    }
}

impl Default for UiMessageBar {
    fn default() -> Self {
        Self::info("")
    }
}

impl UiComponentTemplate for UiMessageBar {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_message_bar(component, ctx)
    }
}
