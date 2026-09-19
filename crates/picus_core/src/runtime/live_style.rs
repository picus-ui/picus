//! Apply interpolated [`CurrentColorStyle`] directly to the retained tree.
//!
//! Color transitions must not invalidate projection each tick: projectors keep
//! the last structural view, and this path patches Background / border / text
//! / scale on the entity's Masonry subtree.

use crate::masonry_core::{
    core::{Widget, WidgetId, WidgetMut},
    kurbo::Affine,
    properties::{Background, BorderColor},
};
use crate::styling::CurrentColorStyle;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use picus_view::picus_widget::properties::{BorderBrush, ContentColor};

use super::MasonryRuntime;

/// Patch retained widgets for entities whose interpolated colors changed.
///
/// Runs after `rebuild_masonry_runtime` so a same-frame structural rebuild
/// (hover start) wins, then later tween ticks skip synthesis entirely.
pub(crate) fn apply_live_color_styles(world: &mut World) {
    let updates: Vec<(Entity, CurrentColorStyle)> = {
        let mut query =
            world.query_filtered::<(Entity, &CurrentColorStyle), Changed<CurrentColorStyle>>();
        query
            .iter(world)
            .map(|(entity, style)| (entity, *style))
            .collect()
    };
    if updates.is_empty() {
        return;
    }
    if !world.contains_non_send::<MasonryRuntime>() {
        return;
    }

    let mut runtime = world.non_send_mut::<MasonryRuntime>();
    let windows: Vec<Entity> = runtime.window_entities().collect();
    for window in windows {
        let Some(window_runtime) = runtime.window_mut(window) else {
            continue;
        };
        for (entity, style) in &updates {
            window_runtime.apply_live_color_style(*entity, *style);
        }
    }
}

impl super::WindowRuntime {
    pub(crate) fn apply_live_color_style(&mut self, entity: Entity, style: CurrentColorStyle) {
        let Some(scope_id) = self.find_widget_id_for_entity_bits(entity.to_bits(), false) else {
            return;
        };

        let direct_children: Vec<WidgetId> = self
            .render_root
            .get_widget(scope_id)
            .map(|widget| widget.children().iter().map(|child| child.id()).collect())
            .unwrap_or_default();

        let mut stack = direct_children.clone();
        let mut subtree = Vec::new();
        while let Some(id) = stack.pop() {
            subtree.push(id);
            if let Some(widget) = self.render_root.get_widget(id) {
                stack.extend(widget.children().iter().map(|child| child.id()));
            }
        }

        for id in subtree {
            if !self.render_root.has_widget(id) {
                continue;
            }
            self.render_root.edit_widget(id, |mut widget| {
                apply_color_props(&mut widget, &style);
            });
        }

        let desired = Affine::scale(style.scale.max(0.01));
        for id in direct_children {
            if !self.render_root.has_widget(id) {
                continue;
            }
            self.render_root.edit_widget(id, |mut widget| {
                if widget.ctx.transform() != desired {
                    widget.set_transform(desired);
                }
            });
        }
    }
}

fn apply_color_props(widget: &mut WidgetMut<'_, dyn Widget>, style: &CurrentColorStyle) {
    if let Some(bg) = style.bg {
        maybe_insert(widget, Background::Color(bg));
    }
    if let Some(text) = style.text {
        maybe_insert(widget, ContentColor { color: text });
    }
    if let Some(border) = style.border {
        maybe_insert(widget, BorderColor { color: border });
        maybe_insert(widget, BorderBrush::Color(border));
    }
}

fn maybe_insert<P: crate::masonry_core::core::Property + PartialEq>(
    widget: &mut WidgetMut<'_, dyn Widget>,
    value: P,
) {
    if !widget.contains_prop::<P>() {
        return;
    }
    if widget.get_prop::<P>() == &value {
        return;
    }
    widget.insert_prop(value);
}
