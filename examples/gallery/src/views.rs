//! UiComponentTemplate implementations for the gallery layout structure.
//!
//! In Fluent UI terms, these are the "app shell" components that define
//! the overall page layout — analogous to the Fluent UI `FluentProvider`
//! wrapping the entire application with consistent styling.

use std::sync::Arc;

use picus::{
    ProjectionCtx, UiComponentTemplate, UiView, apply_label_style, apply_widget_style,
    bevy_ecs::prelude::*,
    masonry_core::{
        layout::{Dim, Length},
        properties::Dimensions,
    },
    resolve_style, resolve_style_for_classes,
    xilem::{
        style::Style as _,
        view::{FlexExt as _, flex_col, flex_item, label, sized_box},
    },
};

use crate::state::GalleryState;

/// Root gallery component: renders a full-viewport flex column layout.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GalleryRoot;

/// Status bar component: displays the most recent user interaction event.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GalleryStatus;

impl UiComponentTemplate for GalleryRoot {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let child_entities = ctx
        .world
        .get::<Children>(ctx.entity)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let children = child_entities
        .into_iter()
        .zip(ctx.children)
        .map(|(entity, child)| {
            let flex_grow = resolve_style(ctx.world, entity).layout.flex_grow;
            if flex_grow > 0.0 {
                flex_item(child, flex_grow).into()
            } else {
                child.into_any_flex()
            }
        })
        .collect::<Vec<_>>();

    Arc::new(
        sized_box(apply_widget_style(
            flex_col(children).gap(Length::px(style.layout.gap)),
            &style,
        ))
        .dims(
            Dimensions::AUTO
                .with_width(Dim::Stretch)
                .with_height(Dim::Stretch),
        ),
    )
}
}

impl UiComponentTemplate for GalleryStatus {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let text_style = resolve_style_for_classes(ctx.world, ["gallery.note"]);
    let state = ctx.world.resource::<GalleryState>();

    Arc::new(apply_widget_style(
        apply_label_style(label(state.last_event.clone()), &text_style),
        &style,
    ))
}
}
