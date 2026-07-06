use crate::helpers::{card, class, grid, note};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    UiLabel, UiMarkdown, UiMultilineTextInput,
    scene::{CommandsSceneExt, bsn, template_value},
};

const MARKDOWN_SAMPLE: &str = r#"# Markdown

Picus renders **strong**, _emphasized_, `inline code`, ~~struck text~~, and [links](https://example.com).

- [x] Parse CommonMark and GFM extensions
- [ ] Keep streaming tails cheap

> Block quotes use the same resolved text palette.

| Feature | Status |
| :-- | --: |
| Tables | Ready |
| Fenced code | Highlighted |

```rust
fn render() {
    println!("markdown");
}
```"#;

/// Text scale, CJK/Unicode, and text wrapping component examples.
///
/// Corresponds to Fluent UI's Text component with typography scale and internationalization.
pub fn spawn_typography_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let scale = card(commands, g, "Text Scale");
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Display / Hero"))
        template_value(class("gallery.typo.hero"))
        ChildOf(scale)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Title text"))
        template_value(class("gallery.typo.title"))
        ChildOf(scale)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Body text for gallery descriptions and form copy."))
        template_value(class("gallery.typo.body"))
        ChildOf(scale)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Caption / secondary metadata"))
        template_value(class("gallery.typo.caption"))
        ChildOf(scale)
    });

    let cjk = card(commands, g, "CJK / Unicode");
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Picus Gallery: 骨 / 骨 / こんにちは / 你好"))
        template_value(class("gallery.typo.title"))
        ChildOf(cjk)
    });
    note(
        commands,
        cjk,
        "Fonts are bridged through Picus; this gallery registers the bundled Noto Sans font.",
    );

    let wrapping = card(commands, g, "Text wrapping");
    commands.spawn_scene(bsn! {
        template_value(
            UiMultilineTextInput::new(
                "Covers font families, weight, wrapping, alignment, and editable text. Picus exposes most text through labels and text inputs today.",
            ).read_only(true)
        )
        ChildOf(wrapping)
    });

    let markdown = card(commands, g, "Markdown");
    commands.spawn_scene(bsn! {
        template_value(UiMarkdown::new(MARKDOWN_SAMPLE))
        template_value(class("gallery.markdown"))
        ChildOf(markdown)
    });

    parent
}
