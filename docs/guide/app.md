# Application guide

## Recommended path

1. Create a Bevy `App` and add `PicusPlugin`.
2. **Explicitly** load a stylesheet (`load_style_sheet` / `load_style_sheet_ron`) and/or
   select a variant (`style_variant`). Picus never auto-picks dark/light.
3. Register business actions with `add_ui_action::<T>()`.
4. Implement `UiComponentTemplate` for custom regions; derive `UiComponent` and
   register them once with `register_ui_components!(app, ...)`.
5. Handle interactions with `MessageReader<UiAction<T>>` (not an internal queue).
6. Run with `app.run_picus(title, BevyWindowOptions::default()...)`.

```rust,ignore
#[derive(Clone, Debug)]
enum AppAction { Inc, Dec }

#[derive(Resource, Default)]
struct Count(i32);

#[derive(Component, Clone, Default, UiComponent)]
#[ui_component(resources(Count))]
struct CountLabel;

impl UiComponentTemplate for CountLabel {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        let n = ctx.world.resource::<Count>().0;
        Arc::new(label(format!("{n}")))
    }
}

fn on_app_action(
    mut reader: MessageReader<UiAction<AppAction>>,
    mut count: ResMut<Count>,
) {
    for UiAction { action, .. } in reader.read() {
        match action {
            AppAction::Inc => count.0 += 1,
            AppAction::Dec => count.0 -= 1,
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/app.ron"))
        .add_ui_action::<AppAction>()
        .add_systems(Startup, setup)
        .add_systems(Update, on_app_action);
    register_ui_components!(app, CountLabel);
    app.run_picus("Counter", BevyWindowOptions::default())
}
```

## Theme contract

- **No theme / no variant** → no framework-provided visible fill or text colour.
- **Partial themes are valid**: missing rules stay transparent/empty; only structural
  RON/token errors fail loading.
- Prefer application RON that sets `default_variant`, or call `style_variant("dark")`.

## Buttons and `UiEmit`

- BSN: attach `template_value(UiEmit::new(AppAction::Inc))` on a `UiButton` entity.
- Without `UiEmit`, enabled buttons emit `BuiltinUiAction::Clicked`.
- Disabled buttons emit nothing.
- Custom projection: `ctx.button(action, label)` / `ctx.action_sender::<T>()`.

## When to use exclusive systems

Prefer ordinary `MessageReader` systems. For world-exclusive mutation, use
`picus::drain_ui_actions::<T>(world)` which reads only newly arrived messages.

See also [events-messages.md](events-messages.md) and [macros.md](macros.md).
