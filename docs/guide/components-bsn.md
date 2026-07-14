# Components and BSN

Picus authoring components are ordinary Bevy components. Public authoring types
and nested authoring values are `Default + Clone`, which lets Bevy Scene
Notation (`bsn!` and `bsn_list!`) create and patch them. Runtime-only values
such as `UiEmit` and hook components are explicit exceptions; use
`template_value(...)` or spawn them from a system.

Use `#[derive(UiComponent)]` for a reusable projected region and add one
`register_ui_components!(app, ...)` list during app setup. The derive records
projection resource dependencies and optional style aliases; it does not invent
the projection body. Implement `UiComponentTemplate` when the region has
authoring state. Use `#[ui_view]` when the region is a zero-state function that
only reads declared resources.

```rust,ignore
#[derive(Component, Clone, Default, UiComponent)]
#[ui_component(resources(AppState))]
struct StatusLabel;

impl UiComponentTemplate for StatusLabel {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        Arc::new(label(ctx.world.resource::<AppState>().status.clone()))
    }
}

register_ui_components!(app, StatusLabel);
```

BSN is best for static tree shape, children, classes, and template-ready field
patches. Keep dynamic collections and business state in resources or systems,
and let the component projector map that state to views. `UiComponentTemplate::expand`
remains authoritative for Picus-owned template parts; application helpers should
not bypass it with hidden registration calls.

For action-bound controls, put the non-generic `UiEmit` in a
`template_value(UiEmit::new(action))` expression. With no `UiEmit`, a button
uses the built-in clicked action. Both paths are delivered to applications as
typed `UiAction<T>` messages.
