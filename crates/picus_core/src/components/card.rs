use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A visual card container with elevation and rounded corners (Fluent v9 Card).
///
/// Children entities are laid out vertically inside the card.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UiCard;

impl UiCard {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl UiComponentTemplate for UiCard {
    fn project(_component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_card(ctx)
    }
}
