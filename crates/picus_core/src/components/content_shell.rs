use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// Page or section content shell: optional title + vertical child stack.
///
/// Use as a lightweight container around demo pages, settings sections, or
/// dialog bodies. Styling is entirely from the application theme
/// (`UiContentShell` / `content.shell` classes); no framework-visible defaults.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiContentShell {
    /// Optional heading rendered above children.
    pub title: Option<String>,
}

impl UiContentShell {
    #[must_use]
    pub fn new() -> Self {
        Self { title: None }
    }

    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl UiComponentTemplate for UiContentShell {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_content_shell(component, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_shell_with_title() {
        let shell = UiContentShell::new().with_title("Settings");
        assert_eq!(shell.title.as_deref(), Some("Settings"));
    }
}
