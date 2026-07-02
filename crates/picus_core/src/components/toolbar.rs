use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A horizontal toolbar that groups action buttons/controls (Fluent v9 Toolbar).
///
/// Children entities are laid out horizontally with compact spacing.
/// Typical children include `UiButton`, `UiDivider` (vertical), and toggle controls.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UiToolbar;

impl UiComponentTemplate for UiToolbar {
    fn project(_component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_toolbar(ctx)
    }
}
