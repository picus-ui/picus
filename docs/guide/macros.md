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

## Failure modes

- Missing `UiComponentTemplate` impl → compile error at register site.
- Ordinary authoring type without `Default`/`Clone` → compile assert unless
  `runtime_only`.
- Calling hidden `__macro_support` from application code is forbidden; use the
  macros and `AppPicusExt` only.
