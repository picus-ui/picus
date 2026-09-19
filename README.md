# picus

A Bevy-first UI framework that connects ECS state management with a retained Masonry Core runtime.

---

## What is picus?

**picus** is a workspace for building desktop user interfaces with Rust. It combines Bevy's ECS architecture with a retained Masonry Core widget tree model, giving you:

- Declarative UI defined through ECS components
- Explicit, typed event handling
- A powerful styling system with CSS-like cascades
- Built-in internationalization support
- Cross-platform window management

The workspace currently contains these crates:

- **picus** — the public application-facing facade (this is the crate you depend on)
- **picus_core** — implementation crate for projection, styling, overlays, runtime integration, and built-ins
- **picus_widget** — Picus-owned retained widget/property backend
- **picus_view** — Picus-owned Xilem-compatible retained view adapter
- **picus_surface** — Vello rendering bridge for window surfaces

This README covers the `picus` crate, which provides the complete UI framework experience through grouped public modules such as `picus::app`, `picus::components`, `picus::projection`, `picus::styling`, and `picus::overlay`. The companion crates provide the retained runtime, rendering, and platform integration.

---

## Installation

Add `picus` to your `Cargo.toml`:

```toml
[dependencies]
picus = "0.1"
```

If you're working with this workspace directly, use path dependencies from the repository root.

---

## Quick start

Recommended path: explicit theme, `UiAction` messages, component macro list, and
`run_picus`. Prefer the real **`timer`** or **`calculator`** examples over inventing
a separate minimal crate. API contracts and full module documentation are available
via `cargo doc -p picus --open`.

```rust,ignore
use std::sync::Arc;

use picus::prelude::*;
use picus::{
    app::{bevy_app::{App, Startup, Update}, bevy_ecs::{message::MessageReader, prelude::*}},
    projection::xilem::{view::label, winit::{dpi::LogicalSize, error::EventLoopError}},
    scene::{CommandsSceneExt, bsn, template_value},
};

#[derive(Clone, Debug)]
enum CounterEvent {
    Increment,
}

#[derive(Resource, Default)]
struct Counter(i32);

#[derive(Component, Clone, Default, UiComponent)]
#[ui_component(resources(Counter))]
struct CounterRoot;

impl UiComponentTemplate for CounterRoot {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        let n = ctx.world.resource::<Counter>().0;
        Arc::new(label(format!("Count: {n}")))
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        UiRoot
        Children [
            CounterRoot,
            (UiButton { label: { "+".into() } } template_value(UiEmit::new(CounterEvent::Increment))),
        ]
    });
}

fn on_counter(
    mut reader: MessageReader<UiAction<CounterEvent>>,
    mut counter: ResMut<Counter>,
) {
    for UiAction { action, .. } in reader.read() {
        if matches!(action, CounterEvent::Increment) {
            counter.0 += 1;
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/app.ron"))
        .insert_resource(Counter::default())
        .add_ui_action::<CounterEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, on_counter);
    register_ui_components!(&mut app, CounterRoot);
    app.run_picus(
        "Counter",
        BevyWindowOptions::default().with_initial_inner_size(LogicalSize::new(360.0, 220.0)),
    )
}
```

1. Load a theme explicitly (no framework default dark).
2. Register `add_ui_action::<T>()` and handle `MessageReader<UiAction<T>>`.
3. Derive `UiComponent` + `register_ui_components!` for custom regions.
4. Run with `run_picus`.

## Architecture & Frame stages

Picus is a **Bevy-first** UI framework: Bevy owns scheduling, windows, and input;
Masonry Core runs as a retained runtime driven by Bevy systems.

```text
Application (depends on `picus` facade only)
    │
    ▼
picus  ──facade──►  picus_core  ──►  picus_view / picus_widget::masonry_core
                         │              └── xilem::core / xilem::winit
                         └──► picus_surface ──► picus_imaging (desktop Vello/wgpu)
```

### Frame stages

| Stage | Work |
|-------|------|
| `PreUpdate` | Input injection, retained message routing, **action dispatch** (`PicusUiSet`) |
| `Update` | Application systems, state changes, overlay lifecycle, style/theme transitions |
| `PostUpdate` | Projection invalidation, UI synthesis, retained rebuild, IME sync |
| `Last` | Vello paint and presentation for each attached window |

### Key architectural contracts

- **Projection invalidation** tracks components and resources registered as dependencies.
- **Application actions** use Bevy `Message` (`UiAction<T>`), not a public drain queue.
- **Theme contract**: missing style data draws nothing visible (transparent); no default brand palette in widgets.
- **Paint isolation**: continuous animations (Spinner, indeterminate ProgressBar) use `PaintIsolation::AnimEntry` to avoid dirtying the base present path.

---

## BSN UI description

Picus supports Bevy Scene Notation as a Rust-embedded UI description language.
`PicusPlugin` installs Bevy's `ScenePlugin`, and `picus::prelude::*`
re-exports `bsn!`, `bsn_list!`, `Scene`, `SceneList`, and the scene spawning
extension traits.

Use BSN when the shape of a UI tree is mostly static and you want to avoid
manual `commands.spawn((..., ChildOf(parent)))` wiring:

```rust,no_run
use picus::app::bevy_ecs::prelude::*;
use picus::prelude::*;

fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        UiRoot
        UiFlexColumn
        StyleClass(vec!["counter.root".to_string()])
        Children [
            UiThemePicker,
            UiLabel {
                text: { "Counter".to_string() },
            },
            (
                UiButton {
                    label: { "Increment".to_string() },
                }
                StyleClass(vec!["counter.primary".to_string()])
            ),
        ]
    });
}
```

The spawned entities are ordinary Picus ECS components. `UiComponentTemplate`
expansion, style resolution, event routing, synthesis, and retained Masonry
projection all run through the same pipeline as hand-written spawns. Picus treats
BSN as an in-code UI DSL; external `.bsn` files are not the recommended workflow.

BSN field-patch syntax, such as `UiButton { label: { "Save".to_string() } }`,
requires the patched type to be template-ready. Picus maintains its public UI
authoring components and their nested authoring values as `Default + Clone`, which
uses Bevy's blanket `FromTemplate` implementation. For application components that
you want to patch in `bsn!`, derive both:

```rust
#[derive(Component, Debug, Clone, Default)]
struct LoginPanel {
    title: String,
}
```

If a component intentionally carries runtime-only or type-erased behavior, do not
make callers guess. Use `template_value(MyComponent::new(...))` or spawn that
component from an ECS system. Picus documents this as the exception path for
event-hook components such as `UiDialogCloseAction`. Components with `Entity`
fields may use `Entity::PLACEHOLDER` only as a patching default; replace it with a
real entity reference when the value matters at runtime.

---

## Features

- **Bevy-native scheduling** — runs entirely within Bevy's update loop, no separate event loop
- **ECS-driven projection** — map components to widget views via `UiComponentTemplate`
- **BSN authoring** — describe static Picus UI trees with Rust-embedded Bevy Scene Notation
- **Typed UI actions** — `UiAction<T>` Bevy messages via `MessageReader`
- **Explicit rendering pass** — Vello paint in `Last` stage, no Bevy render graph needed
- **Built-in components** — buttons, checkboxes, sliders, text inputs, dialogs, scroll views, and more
- **Styling engine** — CSS-like cascade with class selectors, inline overrides, and smooth transitions
- **Internationalization** — synchronous `AppI18n` with `LocalizeText` component
- **Overlay system** — dialogs, tooltips, dropdowns, toasts with automatic placement
- **Helper utilities** — `run_picus()` configures window plugins for desktop apps

---

## Workspace crates

| Crate | Role |
|-------|------|
| `picus` | **Only** application dependency. Grouped modules + macros facade. |
| `picus_macros` | Proc-macros (`UiComponent`, `ui_view`). Re-exported by `picus`. |
| `picus_core` | Implementation: projection, styling, overlays, plugin, runner. |
| `picus_widget` | Lookless retained widgets/properties; owns `masonry_core` re-export module. |
| `picus_view` | Xilem-compatible view adapter on `picus_widget` (`xilem::core`). |
| `picus_surface` | wgpu/Vello surface for Bevy windows. |
| `picus_imaging` | Desktop imaging adapters (paint → WGPU texture). No wasm. |
| `picus_theme_test` | Test-only dark property sets; not for apps. |

### Upstream dependencies

- **`xilem`** facade (git-pinned until an upstream crates.io release includes imaging/layout APIs):
  - `picus_widget::masonry_core` ← `xilem::masonry`
  - `xilem::core` (reactive core)
  - `xilem::winit` (winit event loop and window integration)
- **`picus_imaging`** on crates.io `imaging*` (desktop Vello/wgpu paint adapter).

---

## Examples

| App | Cargo package | Description / Teaches |
|-----|---------------|-----------------------|
| `timer` | `example_timer` | Full DX path: `UiAction`, macros, `run_picus`, explicit theme; canvas dial, async tick task + `UiActionSender` |
| `calculator` | `example_calculator` | Keypad BSN composition + `UiAction`; engine resource projection |
| `todo_list` | `example_todo_list` | Dynamic entities, filters, text input; virtual scroll list |
| `overlay_hit_routing` | `example_overlay_hit_routing` | Builtin click vs overlay hit order; manual overlay spawn |
| `async_downloader` | `example_async_downloader` | Async tasks → `UiActionSender` / messages; dialogs, `IoTaskPool` |
| `game_2048` | `example_game_2048` | Keyboard + button actions; custom hotkey widget |
| `chess_game` | `example_chess_game` | Multi-resource projection, engine thread; board grid projection |
| `gallery` | `example_gallery` | Full Fluent control surface; NavigationView shell, backdrop picker; `UiSpinner` / indeterminate `UiProgressBar` use `PaintIsolation::AnimEntry` |
| `picuscode` | `example_picuscode` | Multi-window, streaming markdown, omp bridge |

Run any example from the repository root:

```bash
cargo run -p example_gallery
```

---

## Styling system

Picus includes a complete styling pipeline inspired by CSS:

- Define rules in a `StyleSheet` resource (loaded from RON files or set directly)
- Attach classes to entities with `StyleClass` (or `classes!("foo", "bar")`)
- Resolve styles in projectors using helper functions
- Support for hover/pressed states and smooth color transitions

### Theme contract (non-negotiable)

1. **No theme / no selected variant** → controls show **no** framework default visible fill or text colour (transparent / empty). This is not an error.
2. The framework **never** auto-selects dark or light.
3. **Partial themes are legal**: implement only the components you use. Missing component or property rules stay empty.
4. Errors are for **structure** only (bad RON, wrong value type, invalid token).

### Loading themes via `AppPicusExt`

| Method | Purpose |
|--------|---------|
| `load_style_sheet(path)` | Asset-path RON with hot-reload |
| `load_style_sheet_ron(text)` | Embedded RON string |
| `style_variant(name)` | Select registered variant (`"dark"`, `"light"`, …) |
| `theme_backdrop(material)` | Override window backdrop |
| `clear_theme_backdrop_override()` | Clear backdrop override |

Priority when resolving:
1. Explicit `style_variant` / already active variant
2. Stylesheet `default_variant`
3. **No fallback** (transparent / empty)

### Style layers

| Layer | Use |
|-------|-----|
| 0 | No theme = no visible defaults |
| 1 | Load Fluent bundle / app RON + variant |
| 2 | Inline / builder styles (`InlineStyle`, `styled`) |
| 3 | Class + app RON override |
| 4 | Full multi-brand stylesheet |

---

## API conventions

Application code depends on `picus`, not `picus_core`. Prefer grouped imports when you only need part of the framework:

```rust
use picus::prelude::*;
use picus::app::bevy_app::App;
```

Use `ProjectionCtx::button` and the other projection helpers for action-aware
controls. The grouped modules expose the application surface; raw retained
widgets are implementation details of custom projectors.

---

## Event handling model

The framework follows a clear pipeline each frame:

1. UI components enqueue typed retained actions
2. `DispatchActions` publishes them as Bevy messages before `Update`
3. You mutate ECS state/resources based on events
4. Picus synthesizes the widget tree in `PostUpdate`
5. The retained Masonry scene is painted and presented in `Last`

This keeps interaction handling explicit and fully ECS-compatible.

---

## Testing

Picus applications are designed for headless Bevy testing. Build an `App` with `PicusPlugin`,
register actions and components, and advance schedules without creating real windows:

- Test action routing with `MessageReader<UiAction<T>>` in test systems.
- Input actions are visible to `Update` readers in the same frame (`PreUpdate` dispatch).
- Sender emissions from `Update` are visible on the next frame.
- Verify component invalidation with `UiProjectionDirtyDebug`.

Common verification commands:

```bash
cargo fmt --all -- --check
cargo test -p picus_core
cargo test -p picus --test ui
cargo check --workspace --all-targets
```

---

## License

Dual-licensed under MIT OR Apache-2.0.
