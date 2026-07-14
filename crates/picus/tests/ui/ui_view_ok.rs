//! Smoke-compile for `#[ui_view]`.
use std::sync::Arc;

use picus::{
    bevy_ecs::prelude::Resource, register_ui_components, ui_view, xilem::view::label,
};

#[derive(Resource, Default)]
struct Count(i32);

#[ui_view(resources(Count))]
fn CountLabel(ctx: ProjectionCtx<'_>) -> UiView {
    let n = ctx.world.resource::<Count>().0;
    Arc::new(label(format!("{n}")))
}

fn main() {
    let mut app = picus::bevy_app::App::new();
    app.insert_resource(Count(0));
    register_ui_components!(&mut app, CountLabel);
    let _ = std::any::type_name::<CountLabel>();
}
