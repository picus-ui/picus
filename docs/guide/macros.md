# Macros

## `#[derive(UiComponent)]`

Generates registration metadata only. You still implement `UiComponentTemplate`
by hand.

```rust,ignore
#[derive(Component, Clone, Default, UiComponent)]
#[ui_component(resources(Count), style_name = "todo.item")]
struct CountLabel;
```

Attributes:

| Attribute | Effect |
|-----------|--------|
| `resources(A, B)` | Registers projection resource dependencies |
| `style_name = "..."` | Selector type alias |
| `runtime_only` | Skips Default+Clone authoring asserts |

## `register_ui_components!(app, A, B)`

Only regular collection entry. Expands to `UiComponentRegistration::register`
per type. Idempotent if a type is listed twice.

## `classes!("a", "b")`

Builds `StyleClass(vec![...])` for BSN/inline use.

## `#[ui_view]`

Function-component sugar for a zero-sized `UiComponent` whose `project` body is
the function body. Prefer when there is no authoring state beyond resources.

```rust,ignore
#[ui_view(resources(Count))]
fn CountLabel(ctx: ProjectionCtx<'_>) -> UiView {
    let n = ctx.world.resource::<Count>().0;
    Arc::new(label(format!("{n}")))
}

// Register like any other component:
register_ui_components!(app, CountLabel);
```

Attributes (same meaning as `#[ui_component(...)]` on derive):

| Attribute | Effect |
|-----------|--------|
| `resources(A, B)` | Projection resource deps |
| `style_name = "..."` | Selector type alias |
| `runtime_only` | Skip Default+Clone asserts (always true for generated ZST) |

Still requires explicit `register_ui_components!` — no inventory/linkme.

## Failure modes

- Missing `UiComponentTemplate` impl → compile error at register site.
- Ordinary authoring type without `Default`/`Clone` → compile assert unless
  `runtime_only`.
- `#[ui_view]` with generics or wrong signature → compile error at the attribute.
- Calling hidden `__macro_support` from application code is forbidden; use the
  macros and `AppPicusExt` only.
