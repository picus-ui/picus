# AGENTS.md

This file is the single working guide for automated agents and humans operating like
agents in this repository. It replaces the former standalone `DESIGN.md` and
`STYLING.md`: keep process rules, architecture, styling behavior, and public design
decisions in this file.

## 1. Non-Negotiables

1. **Single source of truth**
   - For any requested change, verify that the implementation matches this file.
   - If a change alters architecture, styling behavior, public APIs, config schema,
     admin endpoints, tunnel/activation protocol, examples, or expected UX, update
     this file in the same change.
   - Do not implement behavior that contradicts this file without updating this file
     first or in the same patch.

2. **Keep the project test-first**
   - Add or adjust tests for behavior changes.
   - Ensure `cargo test` passes before finishing.
   - Eliminate all compiler and Clippy warnings before finishing; warnings are
     required follow-up work, not optional cleanup.

3. **Rust dependency hygiene**
   - After adding a new Rust dependency in any `Cargo.toml`, check whether
     `cargo upgrade` is available.
   - If it exists, run `cargo upgrade` to see whether a newer compatible version is
     available and prefer the newest reasonable versions.
   - If it does not exist, do not check newer versions; skip this step and proceed.

4. **Avoid interactive app runs by default**
   - Do not run `cargo run` unless user interaction is required to extract runtime
     logs or reproduce an interactive issue.
   - Prefer `cargo test`, static checks, and targeted diagnostics for routine
     verification.

5. **Fluent message-id syntax**
   - In Fluent (`.ftl`) files, do not use dots (`.`) to namespace message IDs.
   - Dots are reserved for Fluent attributes.
   - Use hyphens for namespacing localized keys, such as `nav-home-title` or
     `settings-theme-toggle`.

6. **Default autonomous execution**
   - Do not ask for routine confirmations or step-by-step permission.
   - For straightforward tasks with a clear implementation path, execute directly and
     report results.
   - Ask the user only for architecture-level decisions where multiple valid options
     have meaningful trade-offs.
   - If there is one reasonable path, proceed.

7. **Fork and submodule workflow**
   - `third_party/bevy` and `third_party/xilem` are fork-backed submodules.
   - Keep `origin` pointed to the user's fork and `upstream` pointed to the official
     repository.
   - Do fork modifications only on branch `bevy-xilem-dev`, never directly on
     `main` or the default branch.
   - When syncing upstream, rebase or merge `upstream/*` into `bevy-xilem-dev`, then
     update the submodule commit in this repository.
   - For local validation before pushing fork commits, temporary Cargo `[patch]` or
     path overrides are allowed. Remove temporary overrides once validation is
     complete unless they are intentionally part of the design.

## 2. Quick Verification Checklist

- Always run `cargo test`.
- Run `cargo fmt` when Rust code changes.
- Run `cargo clippy --all-targets --all-features -- -D warnings` when Rust code
  changes.
- When public behavior changes, update this file and the relevant examples/schema
  together.

## 3. Workspace Overview

`picus` is a Bevy-first UI framework that integrates ECS state management with a
retained Xilem/Masonry UI runtime.

The workspace consists of three main crates:

- `picus_core`: the main UI framework crate providing ECS-driven UI projection,
  styling, overlay management, and built-in components.
- `picus_surface`: an external window surface bridge for Vello rendering via wgpu.
- `picus_activation`: cross-platform deep-link and activation handling, including
  single-instance gate, custom URI protocol registration, and IPC forwarding.

The workspace also includes example applications:

- `async_downloader`
- `calculator`
- `chess_game`
- `game_2048`
- `overlay_hit_routing`
- `pixcus`
- `shared_utils`
- `timer`
- `todo_list`
- `ui_showcase`

## 4. Runtime Architecture

### 4.1 Bevy-First Event Loop Ownership

Bevy owns scheduling and window/input message flow. Masonry is driven as a retained
UI runtime resource from Bevy systems. The framework does not run a separate
Xilem/Masonry event loop; GUI apps use Bevy's native `App::run()` and the
`bevy_winit` window lifecycle.

### 4.2 Headless Retained Runtime Resource

`MasonryRuntime` is a Bevy `NonSend` resource that owns:

- Masonry `RenderRoot`
- Current synthesized root view
- Xilem `ViewCtx` and `ViewState`
- Pointer state required for manual event injection
- Active Bevy primary-window attachment metrics: logical size and scale factor
- External surface bridge: `picus_surface::ExternalWindowSurface`
- Vello renderer state

Scheduling:

- `PreUpdate`: input injection, font asset collection, interaction markers, overlay
  click handling, scroll view geometry sync, widget actions.
- `Update`: overlay lifecycle, overlay actions, tooltip hovers, auto-dismiss ticking,
  stylesheet asset events, active variant sync, style dirty marking, style target
  sync, tween interpolation.
- `PostUpdate`: UI synthesis, Masonry rebuild, IME state sync.
- `Last`: explicit Vello paint/present pass.

Initialization invariant:

- `initialize_masonry_runtime_from_primary_window` injects an explicit initial
  logical resize immediately after first attach so Masonry never starts hit-testing
  from a `(0, 0)` root size.

### 4.3 Explicit Masonry/Vello Paint Pass

Because Bevy renderer plugins are intentionally not required for the retained UI
path, `picus_core` performs an explicit Vello paint/present pass in `Last`:

- `RenderRoot::redraw()` produces the current scene.
- `picus_surface::ExternalWindowSurface` owns persistent surface/device state bound
  to the Bevy primary window.
- The pass renders to an intermediate texture, blits to the swapchain surface, and
  presents.
- The primary window requests another redraw to keep UI animations and visual updates
  flowing.

This avoids the "window opens but no pixels are drawn" failure mode when only
window/input plugins are active.

## 5. Input, Hit Testing, and IME

### 5.1 Input Injection Bridge

`inject_bevy_input_into_masonry` consumes Bevy messages and translates them to
Masonry events injected into `MasonryRuntime.render_root`:

- `CursorMoved` -> `PointerEvent::Move`
- `CursorLeft` -> `PointerEvent::Leave`
- `MouseButtonInput` -> `PointerEvent::Down` / `PointerEvent::Up`
- `MouseWheel` -> `PointerEvent::Scroll`
- `KeyboardInput` -> `TextEvent::Keyboard` for navigation/editing keys and
  `TextEvent::Ime::Commit` for committed text
- `Ime` -> `TextEvent::Ime::{Preedit, Commit, Enabled, Disabled}`
- `WindowFocused` -> `TextEvent::WindowFocusChange`
- `WindowResized` -> `WindowEvent::Resize`
- `WindowScaleFactorChanged` -> `WindowEvent::Rescale`

Pointer bridge invariants:

- `Window::physical_cursor_position()` from the current `PrimaryWindow` is the source
  of truth for injected Masonry pointer coordinates.
- When physical cursor data is unavailable, pointer interaction injection is skipped.
- Click-path ordering injects `PointerMove` before each `PointerDown`/`PointerUp` so
  hot/hovered state is current before activation.
- Window resize injection uses logical `Window::width()` / `Window::height()` for
  DPI-correct dimensions.

### 5.2 IME Bridge

IME signals from Masonry are captured through `RenderRoot` callbacks and translated
back to Bevy window IME state (`ime_enabled`, `ime_position`) in
`sync_masonry_ime_state_to_bevy_window`.

### 5.3 Hit-Testing Invariants

Layout-affecting styles such as padding, border, background, and corner radius are
applied directly to the target UI component widget. Masonry hit-testing must match
the structural box model users see, especially for bounded overlays/dialogs versus
global backgrounds.

## 6. ECS UI Model and Component Registration

### 6.1 Core Data Model

Core built-ins include `UiRoot`, `UiOverlayRoot`, `UiFlexColumn`, `UiFlexRow`,
`UiLabel`, `UiButton`, and `LocalizeText`. Node identities for projection context use
`entity.to_bits()`.

### 6.2 Component-Centric Encapsulation

Built-in logical UI components live under `crates/picus_core/src/components/*.rs`.
Each module owns its logical component shape, template-part policy, and registration
contract.

The unifying trait is `UiComponentTemplate`:

- `expand(world, entity)` performs one-time logical-to-template expansion.
- `project(&T, ProjectionCtx) -> UiView` performs ECS-to-Masonry projection.

### 6.3 Registration API

`AppPicusExt` exposes `.register_ui_component::<T: UiComponentTemplate>()`. One call
performs projector registration, `Added<T>` expansion system hookup, and selector type
alias registration. Built-in UI components are registered centrally through
`PicusBuiltinsPlugin`, so user apps only call this for explicit custom components.

### 6.4 Built-In Components

Interactive controls:

- `UiButton`
- `UiCheckbox`
- `UiSlider`
- `UiSwitch`
- `UiTextInput`
- `UiComboBox`
- `UiDropdownMenu`
- `UiDropdownItem`
- `UiRadioGroup`
- `UiTabBar`
- `UiTreeNode`
- `UiMenuBar`
- `UiMenuBarItem`
- `UiMenuItemPanel`
- `UiColorPicker`
- `UiColorPickerPanel`
- `UiDatePicker`
- `UiDatePickerPanel`
- `UiThemePicker`
- `UiThemePickerMenu`
- `UiPopover`

Display and container widgets:

- `UiBadge`
- `UiProgressBar`
- `UiDialog`
- `UiScrollView`
- `UiTable`
- `UiTooltip`
- `UiSpinner`
- `UiGroupBox`
- `UiSplitPane`
- `UiToast`

### 6.5 Portal-Based `UiScrollView`

`UiScrollView` is a logical ECS UI component projected through a Masonry portal view,
with explicit scroll state (`scroll_offset`, `content_size`) and optional external
scrollbar parts.

Scroll view invariants:

- `PreUpdate` reads back portal geometry from Masonry and synchronizes ECS
  `viewport_size` / `content_size` each frame.
- `viewport_size` is only an initial logical seed; live viewport geometry follows
  parent layout constraints in Masonry and is synchronized back to ECS.
- `scroll_offset` is clamped to physical bounds after drag, wheel, and layout-sync
  updates.
- Wheel deltas route from deepest hit target outward and are consumed by the first
  ancestor `UiScrollView` that can move, preventing boundary desync in nested scroll
  views.

## 7. Event Handling

### 7.1 Zero-Closure ECS Button Path

To remove user-facing closure boilerplate:

- `EcsButtonView` implements `xilem_core::View` on top of a custom wrapper widget.
- `EcsButtonWithChildView` provides the same event semantics for composed button
  content.
- On interaction, these views emit structural events such as `PointerEntered` and
  `PointerPressed`, plus typed ECS actions through `UiEventQueue`.

Built-in interactive controls should stay on this unified ECS event route.

### 7.2 Typed Action Queue

`UiEventQueue` is a Bevy `Resource` backed by a lock-free `SegQueue`. Widgets push
type-erased actions. Bevy systems drain typed actions through `drain_actions::<T>()`
non-destructively for multiple consumers.

### 7.3 Pointer Event Bubbling

`UiPointerHitEvent` represents a hit-tested pointer event before ECS bubbling.
`UiPointerEvent` is emitted for each ancestor in the hierarchy with a `consumed` flag.
The `StopUiPointerPropagation` marker component stops bubbling at the tagged entity.

### 7.4 Overlay Pointer Routing

`OverlayPointerRoutingState` tracks suppressed presses/releases to prevent trigger
buttons from receiving the corresponding release after an overlay consumes a click.
This avoids sticky-pressed visual states.

## 8. Styling System Reference

The styling system is CSS-like, ECS-driven, and implemented primarily in
`crates/picus_core/src/styling.rs`.

### 8.1 Goals

The styling system is designed to be:

- Data-driven: styles are ECS components/resources, not ad-hoc widget chains.
- Composable: class styles (`StyleClass` + `StyleSheet`) combine with inline
  overrides.
- Interactive: pseudo states (`InteractionState.hovered`, `InteractionState.pressed`)
  are driven from UI interaction events.
- Animated: smooth color transitions between interaction states use `bevy_tween`.

### 8.2 Inline Style Model

Inline overrides can be represented as a consolidated `InlineStyle` component or as
legacy split components.

Preferred:

- `InlineStyle`
  - `layout: LayoutStyle`
  - `colors: ColorStyle`
  - `text: TextStyle`
  - `transition: Option<StyleTransition>`

Legacy split components remain supported:

- `LayoutStyle`
  - `padding: Option<f64>`
  - `gap: Option<f64>`
  - `corner_radius: Option<f64>`
  - `border_width: Option<f64>`
  - `justify_content`
  - `align_items`
  - `scale`
- `ColorStyle`
  - base: `bg`, `text`, `border`
  - pseudo overrides: `hover_*`, `pressed_*`
- `TextStyle`
  - `size: Option<f32>`
  - `text_align`
- `StyleTransition`
  - `duration: f32` in seconds

### 8.3 Stylesheet Model

Core stylesheet types:

- `StyleClass(pub Vec<String>)`
- `Selector::{Type, TypeName, Class, PseudoClass, And, Descendant}`
- `StyleSetter { layout, colors, text, font_family, transition }`
- `StyleValue::{Value(T), Var(String)}`
- `StyleRule { selector, setter }`
- `StyleSheet { tokens, rules }`

Convenience APIs exist for class-only rules:

- `StyleSheet::set_class("name", setter)`
- `StyleSheet::with_class("name", setter)`

Selectors support:

- `Type`: component `TypeId`
- `TypeName`: string component name resolved through `StyleTypeRegistry`
- `Class`: style class
- `PseudoClass`: `:hover`, `:pressed`
- `And`: conjunction
- `Descendant`: ancestor-descendant relationships

Style rules support token-aware values with `StyleValue::Var(String)`, allowing rules
to reference named tokens from the active `StyleSheet`.

### 8.4 Pseudo State, Cache, and Invalidation

Pseudo classes are backed by stable ECS state:

- `InteractionState { hovered: bool, pressed: bool }`

Cache and invalidation runtime state:

- `StyleDirty`: marks entities requiring recomputation.
- `ComputedStyle`: cached resolved style read by projectors.

`ComputedStyle` includes `font_family: Option<Vec<String>>`, allowing projectors to
apply font-family from stylesheet resolution without re-running cascade logic every
frame.

When descendant selectors are present, invalidation propagates from changed ancestors
to descendants so `A B`-style rules stay correct after ancestor class or pseudo-state
changes.

### 8.5 Cascade and Resolution Order

`resolve_style(world, entity)` follows this precedence from low to high:

1. Selector-matched rules from `StyleSheet`, including `Type`, `TypeName`, `Class`,
   `PseudoClass`, `And`, and `Descendant`.
2. Inline overrides from `InlineStyle` or legacy `LayoutStyle` / `ColorStyle` /
   `TextStyle` / `StyleTransition`.
3. Compatibility pseudo color overrides (`hover_*`, `pressed_*`) from `ColorStyle`.
4. Animated override from `CurrentColorStyle`, if present.

Class and inline styles define intent, pseudo state chooses the target, and the
animator provides smooth in-between values.

### 8.6 Base and Active Stylesheet Tiers

Runtime styling distinguishes two explicit tiers:

- `BaseStyleSheet`: embedded Fluent baseline.
- `ActiveStyleSheet`: runtime override tier.

Active rules cascade over baseline rules by priority. The active tier can come from a
hot-reloaded asset path (`AppPicusExt::load_style_sheet`, tracked by
`ActiveStyleSheetAsset`) or from embedded RON text
(`AppPicusExt::load_style_sheet_ron`).

Runtime selectors and tokens owned by the active tier override the baseline tier
without permanently mutating the embedded theme bundle.

Baseline Fluent theme includes a global `Type("UiRoot")` preflight rule for the app
surface background. The `UiRoot` projector stretches to the full viewport so root
background styling consistently covers the entire window.

`PicusPlugin` boots with embedded Fluent Dark by default. Built-in Fluent theming is a
single multi-variant bundle (`fluent_theme.ron`) containing `dark`, `light`, and
`high-contrast` variants.

Runtime variant selection is state-driven through `ActiveStyleVariant`. Apps set the
desired variant by name through `set_active_style_variant_by_name(...)`, and
`sync_active_style_variant` applies it to `BaseStyleSheet` and runtime `StyleSheet`.
Plugin bootstrap sets the theme file's default variant active, and the first `Update`
pass applies it automatically.

Theme activation no longer exposes `install_*` APIs. The public path is active-variant
state plus automatic sync.

Variant bundles support top-level shared `rules` / `tokens` plus per-variant
overrides. This keeps common selector graphs out of any single variant and lets each
variant focus on palette/token deltas.

When an entity has no matched selector rules and no inline style sources, style
resolution intentionally uses a transparent text fallback so UI does not inherit
Masonry/Xilem intrinsic default text appearance.

### 8.7 Supported Style Properties

Layout:

- `padding`
- `gap`
- `corner_radius`
- `border_width`
- `justify_content`
- `align_items`
- `scale`

Colors:

- `bg`
- `text`
- `border`
- `hover_*`
- `pressed_*`

Text:

- `size`
- `text_align` (`Start`, `Center`, `End`)

Other:

- `font_family: Option<Vec<String>>`
- `box_shadow`
- `transition: Option<StyleTransition>` with duration in seconds

### 8.8 Plugin Wiring

`PicusPlugin` wires the style stack:

- Initializes `StyleSheet`.
- Registers embedded Fluent variants from `crates/picus_core/src/theme/fluent_theme.ron`
  into `RegisteredStyleVariants`.
- Sets the bundle default variant as `ActiveStyleVariant`.
- Runs `collect_bevy_font_assets`, `sync_fonts_to_xilem`, and
  `sync_ui_interaction_markers` in `PreUpdate`.
- Runs `sync_active_style_variant`, `mark_style_dirty`, `sync_style_targets`, and
  transition animation systems in `Update`.
- Registers `DefaultTweenPlugins` from `bevy_tween`.

Theme switching uses active-variant state:

- `set_active_style_variant_by_name(world, "dark" | "light" | "high-contrast")`
- `set_active_style_variant_to_registered_default(world)`

### 8.9 Defining Styles

A common example/screen pattern is:

- `setup_*_styles` for style declarations.
- `setup_*_world` for ECS structure.

Example style definitions:

- `style_sheet.set_class("todo.root", setter)`
- `style_sheet.set_class("todo.add-button", setter)`
- `style_sheet.add_rule(StyleRule::new(Selector::and(...), setter))`

Style class naming may use dots because these are not Fluent message IDs. Prefer
feature namespaces such as `todo.*`, `calc.*`, and `chess.*`, with separate classes
for roots, controls, and text such as `*.root`, `*.button`, and `*.button.label`.

### 8.10 Applying Styles in Projectors

Key helpers:

- `resolve_style(world, entity)`
- `resolve_style_for_classes(world, ["class.a", "class.b"])`
- `resolve_style_for_entity_classes(world, entity, ["class.a", "class.b"])`
- `apply_widget_style(view, &style)`
- `apply_label_style(label(...), &style)`
- `apply_text_input_style(text_input(...), &style)`

Recommended projector pattern:

1. Resolve root/entity style with `resolve_style`.
2. Resolve shared class styles with `resolve_style_for_classes`.
3. Compose the widget tree using style helpers.

Keep structure and style concerns separated.

### 8.11 Interaction and Pseudo States

Interaction events are emitted by ECS-backed UI components, especially the custom ECS
button widget path:

- `PointerEntered`
- `PointerLeft`
- `PointerPressed`
- `PointerReleased`

`sync_ui_interaction_markers` consumes these events and updates `InteractionState` by
mutating the stable component in place. Do not replace this with frequent
marker-component insertion/removal; that causes unnecessary archetype churn.

### 8.12 Smooth Tween-Based Transitions

Transitions use `bevy_tween`:

- `DefaultTweenPlugins`
- `EaseKind`
- `TimeRunner`
- `TimeSpan`
- `ComponentTween<T>`
- `Interpolator`

`ColorStyleLens` implements `Interpolator<Item = CurrentColorStyle>` and interpolates
RGBA channels for background, text, and border color. Easing is applied by tween
sampling; the default interaction transition easing is `QuadraticInOut`.

`ComputedStyleLens` is available for full computed-style tweening. It treats
`font_family` as non-interpolable and switches it only on tween completion.

State-change behavior:

- `mark_style_dirty` marks entities with changed style dependencies.
- `sync_style_targets` recomputes dirty entities, updates `ComputedStyle`, and
  computes a new `TargetColorStyle`.
- If a transition is configured and the target changed, systems insert/update
  `TimeRunner`, `TimeSpan`, and `ComponentTween<ColorStyleLens>` targeting
  `CurrentColorStyle`.
- Tweens start from the current animated value and end at the new target value.
- `DefaultTweenPlugins` sample easing and apply component tweens.
- Projectors read `ComputedStyle` plus animated `CurrentColorStyle` via
  `resolve_style`.

Recommended duration for UI micro-interactions: `0.10` to `0.18` seconds, commonly
around `0.15` seconds in examples.

### 8.13 Styling Checklist

To animate a component on interaction:

1. Define base and hover/pressed colors in `ColorStyle` or a stylesheet rule.
2. Set `transition: Some(StyleTransition { duration: ... })`.
3. Ensure the component emits UI interaction events so `InteractionState` changes and
   the entity becomes `StyleDirty`.
4. Apply style through projector helpers.

Common pitfalls:

- `resolve_style_for_classes(...)` is static and does not bind pseudo state by itself.
- Use `resolve_style_for_entity_classes(...)` when pseudo-state-dependent classes are
  needed.
- Pseudo-state transitions require an interaction event source.
- Some UI components have internal defaults that require styling the interactive path
  itself rather than only a wrapper.
- If style behavior changes, update this file, implementation, and examples together.

### 8.14 Styling Reference Files

- Core styling: `crates/picus_core/src/styling.rs`
- Plugin wiring: `crates/picus_core/src/plugin.rs`
- ECS button interaction source: `crates/picus_core/src/widgets/ecs_button_widget.rs`
- ECS button view path: `crates/picus_core/src/views/ecs_button_view.rs`
- Theme bundle: `crates/picus_core/src/theme/fluent_theme.ron`

When extending the system, such as adding `:disabled`, inherited style contexts, or
layout tweening, extend `StyleRule` first, then wire resolve/sync/animation behavior,
then update examples and this file.

## 9. Overlay and Modal System

`picus_core` includes a built-in ECS overlay model using floating/portal roots
natively stacked through Masonry.

### 9.1 Layering and Positioning

- `OverlayStack` maintains top-most order, and `sync_overlay_stack_lifecycle` keeps it
  pruned.
- `OverlayPlacement` handles center/top/bottom/left/right and start/end alignments.
- `sync_overlay_positions` calculates clamping and auto-flipping against screen edges.
- `UiPopover` centralizes anchor, placement, and auto-flip configuration for anchored
  floating surfaces. Dropdowns, tooltips, picker panels, and app-level popovers reuse
  this placement path.
- Built-in floating widgets include `UiDialog`, `UiComboBox`, `UiDropdownMenu`,
  `UiTooltip`, `UiToast`, `UiMenuItemPanel`, `UiColorPickerPanel`,
  `UiDatePickerPanel`, and `UiThemePickerMenu`.
- `UiToast` defaults to bottom-end placement and supports configurable placement,
  width, and close button.
- `AutoDismiss { timer }` supports timer-driven teardown for temporary overlays.

### 9.2 Dialog Close Contract

`UiDialog` can carry a typed close-action hook. Both the built-in Lucide X close
button in the top-right dialog chrome and outside-click dismissal route through the
same overlay helper. That helper emits the hook through `UiEventQueue` before
despawning. Dialogs without a hook keep despawn-only behavior.

### 9.3 FOUC Prevention

Overlay projectors must render with fully transparent resolved styles while
`OverlayComputedPosition.is_positioned == false`, then become visible once synchronized
placement is available.

### 9.4 Layered Dismissal and Blocking Flow

`handle_global_overlay_clicks` dynamically evaluates pointer location against the
top-most overlay using `RenderRoot::get_hit_path(physical_pos)` and all widget IDs
bound to that overlay entity, with an `OverlayComputedPosition` rectangle fallback.
This avoids false outside-click dismissal when interacting with deeply nested
portal/menu content.

Clicks outside the opaque overlay root cause dismissals without disrupting interactive
siblings. Optional `UiOverlayRoot` dimly renders full-view backgrounds without
structurally wrapping modal UI boundaries.

When clicking an overlay anchor to close an anchored overlay, pointer suppression is
press-only for the consumed click. This avoids stale suppressed-release state that can
leave trigger buttons visually or logically pressed.

### 9.5 Overlay Reparenting

`reparent_overlay_entities` automatically moves overlay entities such as dialogs,
dropdowns, menus, tooltips, toasts, and pickers under the global `UiOverlayRoot` to
keep them outside normal layout clipping hierarchies.

## 10. Synthesis Pipeline

UI synthesis is driven by `UiProjectorRegistry`. In `PostUpdate`:

1. `gather_ui_roots` gathers `UiRoot` and `UiOverlayRoot` entities, sorting overlays
   last.
2. `synthesize_entity` recursively projects ECS entities into views.
3. `SynthesizedUiViews` stores the result.
4. `MasonryRuntime` rebuilds the retained Masonry root.

When more than one root is present, runtime rebuild composes synthesized roots into a
full-viewport `zstack` aligned to top-left before calling Xilem Core rebuild.

`UiSynthesisStats` tracks:

- `root_count`
- `node_count`
- `cycle_count`
- `missing_entity_count`
- `unhandled_count`

## 11. Assets, Fonts, Icons, and Internationalization

### 11.1 Iconography

Built-in directional indicators, radio markers, and other icons use
`picus_core::icons`, backed by `lucide-icons` icon data/font assets. The plugin
registers bundled Lucide font bytes at startup. Icon text styling uses the upstream
Lucide family name, `"lucide"`, so rendering remains stable across locales and system
font configurations.

### 11.2 Font Bridge

`XilemFontBridge` moves Bevy `Asset<Font>` data into Masonry. It registers font bytes
from `collect_bevy_font_assets` directly to `MasonryRuntime` through
`sync_fonts_to_xilem`.

Supported registration paths:

- Asset-server loading.
- Direct byte registration through `AppPicusExt::register_xilem_font_bytes`.
- Direct path registration through `AppPicusExt::register_xilem_font_path`.

### 11.3 Internationalization

`AppI18n` is the centralized synchronous i18n registry. Setup uses
`.register_i18n_bundle()`. Declarative font stacks are applied based on locale
priorities.

`resolve_localized_text` resolves `LocalizeText` component keys through the active
bundle and falls back to the key or provided fallback text.

Remember: Fluent message IDs must use hyphens, not dots.

## 12. Developer Ergonomics

### 12.1 UI Componentization Policy

Use two levels of componentization:

- Micro-componentization: reusable fragments returned as pure Rust View helpers
  (`UiView` or `impl View`).
- Macro-componentization: UI regions mapped purely to ECS through
  `register_ui_component::<T>()`.

### 12.2 Bevy-Native Run Helpers

`run_app()` and `run_app_with_window_options()` avoid raw setup tasks and bootstrap
native `bevy_winit` safely into Bevy systems for desktop lifecycle apps.

They auto-enable Bevy's native window plugins:

- `AccessibilityPlugin`
- `InputPlugin`
- `WindowPlugin`
- `WinitPlugin`

Then they call `App::run()`.

## 13. `picus_surface`

`picus_surface` provides a Vello rendering surface attached to an externally owned
Bevy window. It manages:

- wgpu instance/device/queue lifecycle
- Surface configuration and resizing
- Scene rendering with DPI-aware scaling
- Texture blitting and presentation
- AMD Windows compatibility workaround using premultiplied alpha

The bridge is created from a `RawHandleWrapper` and synchronized with window metrics:
physical size, logical size, and scale factor.

## 14. `picus_activation`

`picus_activation` handles application activation, single-instance enforcement, and
custom URI protocol registration across platforms.

Responsibilities:

- `app-single-instance`: `notify_if_running(app_id)` detects an already-running
  primary instance, and `start_primary(app_id, ...)` keeps primary ownership alive.
- Non-macOS activation forwarding uses an `ipc-channel` one-shot rendezvous. The
  primary continuously rotates `IpcOneShotServer` endpoints and publishes the active
  server name in a per-app rendezvous file under the temp dir. Secondaries read that
  name, connect through `IpcSender`, forward URI payloads, and wait for explicit ack
  (`Ack` / `Nack`) over an embedded IPC ack channel before exiting.
- macOS activation delivery is Apple-Event-native. `picus_activation` installs an
  `NSAppleEventManager` `kAEGetURL` handler through `objc2`, receives custom-scheme
  callbacks in the running app process, and feeds them directly into the activation
  service queue without IPC rendezvous.
- Custom URI protocol registration is crate-native. Windows uses HKCU registry
  (`Software\Classes/<scheme>`); Linux writes a
  `~/.local/share/applications/*.desktop` entry and runs `xdg-mime default` plus
  `update-desktop-database`.
- Startup URI collection scans raw process arguments directly, normalizes quoted
  values, filters callback URIs by case-insensitive scheme match, and deduplicates
  before secondary-to-primary IPC forwarding.
- macOS bundle workflow is supplied by apps through `MacosBundleConfig`.
  `picus_activation` reads that plist, creates or updates a runnable `.app` bundle
  around the current executable when needed, registers it with Launch Services
  (`lsregister`), and requests the current app bundle become the default URL-scheme
  handler through
  `NSWorkspace::setDefaultApplicationAtURL:toOpenURLsWithScheme:completionHandler:`.
- When already running from an application bundle on macOS, activation resolves that
  bundle through `NSBundle::mainBundle()` instead of inferring solely from
  `current_exe()`.

Bootstrap flow:

- `bootstrap(config)` returns `BootstrapOutcome::Primary(ActivationService)` or
  `BootstrapOutcome::SecondaryForwarded`.
- Primary instances receive `startup_uris` from command-line arguments at launch.
- Primary instances receive subsequent activation URIs through `drain_uris()`.
- Secondary instances forward URIs to the primary and exit immediately.

## 15. Plugin System

`PicusPlugin` wires the framework:

- Ensures `TaskPoolPlugin`, `AssetPlugin`, and `DefaultTweenPlugins` are present.
- Adds `TimePlugin` and `PicusBuiltinsPlugin`.
- Registers resources:
  - `UiProjectorRegistry`
  - `SynthesizedUiViews`
  - `UiSynthesisStats`
  - `UiEventQueue`
  - `StyleSheet`
  - `BaseStyleSheet`
  - `ActiveStyleSheet`
  - `ActiveStyleSheetAsset`
  - `ActiveStyleSheetSelectors`
  - `ActiveStyleSheetTokenNames`
  - `ActiveStyleVariant`
  - `AppliedStyleVariant`
  - `RegisteredStyleVariants`
  - `StyleAssetEventCursor`
  - `XilemFontBridge`
  - `AppI18n`
  - `OverlayStack`
  - `OverlayPointerRoutingState`
  - `MasonryRuntime`
- Adds Bevy message types for window/input events.
- Registers systems to `PreUpdate`, `Update`, `PostUpdate`, and `Last`.
- Registers embedded Fluent theme variants and sets the default active variant.
- Registers core projectors through `register_core_projectors`.

`PicusBuiltinsPlugin` registers all built-in UI components.

## 16. Example-Specific Design Notes

`pixcus` exposes authentication through a sidebar-footer login entry that opens a
modal overlay dialog for Pixiv OAuth inputs.

Once authenticated, the same sidebar footer switches to an avatar-based account
trigger with a compact logout popover that reuses shared anchored popover placement.

Selecting an illustration opens a `UiDialog`-backed artwork detail modal. The modal
expands to a near-fullscreen two-column layout sized from current `ViewportMetrics`
rather than a fixed `1320x880`:

- Left side: large artwork hero.
- Right side: dedicated `UiScrollView` rail stacking artwork, author, image, caption,
  and tag metadata.
- Long captions and tags scroll independently.
- The built-in Lucide X close affordance remains visible in the top-right chrome.

## 17. Non-Goals

- Custom render-graph bridging is out of scope.
- Bevy render-graph integration is out of scope for the retained UI path; use the
  explicit Vello surface through `picus_surface`.
- Automatic closure-based event handling is out of scope; ECS queues are the unified
  event/action path.
- CSS cascade complexity beyond selector-based rules and inline overrides is out of
  scope.
- Inherited style contexts are not supported; styles are per-entity with descendant
  selector matching.
