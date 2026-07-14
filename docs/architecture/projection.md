# Projection and invalidation

Projection maps ECS authoring components to retained views. A
`UiProjectorRegistry` stores the projector for each registered component and a
root entity anchors each projected tree. Registration is explicit through
`register_ui_components!(app, ...)`; there is no inventory or linkme discovery.

`#[derive(UiComponent)]` generates registration metadata. The optional
`#[ui_component(resources(A, B))]` attribute records resources that the
component reads during projection, so changes to those resources invalidate the
right roots. `#[ui_view]` is the zero-state function form of the same contract.
Low-level integrations can use `picus::runtime::advanced`, but application code
should keep the metadata beside the component and use the batch macro.

Projection invalidation is represented by `UiProjectionInvalidation`. The
runtime marks roots dirty when a component, dependency resource, style, window,
or child structure changes, then synthesis rebuilds only the affected retained
trees. `UiProjectionDirtyDebug` records the last reasons and dirty windows for
diagnostics and headless tests; idle frames clear the debug state.

Avoid no-op mutable writes to projection-visible components and resources. Bevy
change detection is the signal that drives invalidation, so writing the same
value can cause needless rebuilds and hide the real dependency boundary. Keep
projection functions deterministic and read dependencies declared by their
metadata.

Projection helpers such as `ProjectionCtx::button`, `flex_row`, `flex_col`, and
`styled` provide the application-level path. Raw projector registration and
retained view details belong to the advanced module.
