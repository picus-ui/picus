use bevy_ecs::{entity::Entity, prelude::*};

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A collapsible container with a header that toggles content visibility.
///
/// Children entities are only rendered when [`is_expanded`] is `true`.
/// The header row displays an expand/collapse chevron before the header text.
#[derive(Component, Debug, Clone)]
pub struct UiExpander {
    /// Header text shown in the always-visible header row.
    pub header: String,
    /// When `true` the child content is visible.
    pub is_expanded: bool,
}

impl UiExpander {
    #[must_use]
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            is_expanded: false,
        }
    }

    /// Pre-expand the container.
    #[must_use]
    pub fn with_expanded(mut self) -> Self {
        self.is_expanded = true;
        self
    }
}

/// Emitted when the expander is toggled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiExpanderChanged {
    pub expander: Entity,
    pub is_expanded: bool,
}

impl UiComponentTemplate for UiExpander {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_expander(component, ctx)
    }
}
