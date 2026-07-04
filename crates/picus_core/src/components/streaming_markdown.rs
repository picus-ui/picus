use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A streaming Markdown document optimized for append-only LLM output.
///
/// Unlike [`crate::UiMarkdown`], which re-parses its entire `source` every
/// frame, `UiStreamingMarkdown` caches fully-parsed **completed blocks** and
/// only re-parses the trailing **in-progress segment** on each append. This
/// keeps per-frame cost roughly proportional to the number of new tokens
/// rather than the total document length.
///
/// Append tokens through [`Self::append`] (or [`Self::append_str`]) as they
/// arrive from the model. When a chunk is known to be complete (for example a
/// finished paragraph or code block), call [`Self::flush_completed`] to
/// promote it into the cached completed prefix. Call [`Self::finish`] once
/// the stream is fully delivered; this flushes any remaining in-progress text
/// and marks the document as completed.
///
/// The projection layer renders the completed prefix from cache and the
/// in-progress tail with a fresh parse, then composes both into one view.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiStreamingMarkdown {
    /// Cached, fully-parsed completed prefix source.
    completed: String,
    /// The unparsed in-progress tail source.
    in_progress: String,
    /// Whether the stream has finished delivering content.
    finished: bool,
}

impl UiStreamingMarkdown {
    /// Create an empty streaming Markdown document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a single token/chunk to the in-progress tail.
    pub fn append(&mut self, token: &str) {
        if self.finished {
            return;
        }
        self.in_progress.push_str(token);
    }

    /// Append a string slice to the in-progress tail.
    pub fn append_str(&mut self, token: &str) {
        self.append(token);
    }

    /// Promote the current in-progress tail into the completed prefix.
    ///
    /// Call this when the tail is known to contain only finished blocks (for
    /// example after a paragraph terminator or a closed code fence).
    pub fn flush_completed(&mut self) {
        if self.in_progress.is_empty() {
            return;
        }
        self.completed.push_str(&self.in_progress);
        self.in_progress.clear();
    }

    /// Mark the stream as finished and flush any remaining in-progress text.
    pub fn finish(&mut self) {
        self.flush_completed();
        self.finished = true;
    }

    /// Returns `true` once [`finish`] has been called.
    ///
    /// [`finish`]: Self::finish
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The completed prefix source text.
    #[must_use]
    pub fn completed_source(&self) -> &str {
        &self.completed
    }

    /// The in-progress tail source text.
    #[must_use]
    pub fn in_progress_source(&self) -> &str {
        &self.in_progress
    }

    /// The full source (completed prefix + in-progress tail).
    #[must_use]
    pub fn full_source(&self) -> String {
        let mut full = self.completed.clone();
        full.push_str(&self.in_progress);
        full
    }
}

impl UiComponentTemplate for UiStreamingMarkdown {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::markdown::project_streaming_markdown(component, ctx)
    }
}
