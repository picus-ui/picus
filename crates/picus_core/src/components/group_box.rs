use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A titled grouping container for related content.
///
/// Place content entities as ECS children. Visual treatment is supplied by
/// theme rules or inline/app-specific styles.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiGroupBox {
    /// Title displayed at the top of the group box.
    pub title: String,
}

impl UiGroupBox {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl UiComponentTemplate for UiGroupBox {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_group_box(component, ctx)
    }
}
