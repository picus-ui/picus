use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A Markdown document component.
///
/// Renders a Markdown source string as a vertical stack of styled blocks:
/// headings, paragraphs, lists, block quotes, code blocks (with optional
/// syntax highlighting), inline emphasis/code/links, and thematic breaks.
///
/// The source is parsed with `pulldown-cmark` (CommonMark + GFM tables and
/// task lists). Code blocks are syntax-highlighted with `syntect` when a
/// language fence is present and a matching grammar is available.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiMarkdown {
    /// The Markdown source text to render.
    pub source: String,
}

impl UiMarkdown {
    /// Create a new `UiMarkdown` from the given source text.
    #[must_use]
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }
}

impl UiComponentTemplate for UiMarkdown {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::markdown::project_markdown(component, ctx)
    }
}
