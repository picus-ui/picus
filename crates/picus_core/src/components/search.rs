use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A search input field with a search icon (Fluent v9 SearchBox).
///
/// Renders as a styled text input with a magnifying-glass icon.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiSearch {
    /// Placeholder text shown when the field is empty.
    pub placeholder: String,
    /// Current text value.
    pub value: String,
}

impl UiSearch {
    #[must_use]
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            value: String::new(),
        }
    }
}

impl UiComponentTemplate for UiSearch {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_search(component, ctx)
    }
}
