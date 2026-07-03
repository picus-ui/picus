use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate, icons::PicusIcon};

/// Button appearance matching Fluent UI v9 Button component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonAppearance {
    /// Default button with subtle background and border.
    #[default]
    Default,
    /// Filled with brand/accent color.
    Primary,
    /// Transparent background with visible border.
    Outline,
    /// Nearly transparent, minimal style.
    Subtle,
    /// Fully transparent background, no border.
    Transparent,
}

/// Button size matching Fluent UI v9 size scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    #[default]
    Medium,
    Small,
    Large,
}

/// Button shape variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonShape {
    /// Default rounded (borderRadiusMedium).
    #[default]
    Rounded,
    /// Fully circular/pill shape.
    Circular,
    /// Sharp square corners.
    Square,
}

/// Icon position relative to button label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonIconPosition {
    /// Icon before the label text.
    #[default]
    Before,
    /// Icon after the label text.
    After,
    /// Only icon, no label text.
    IconOnly,
}

/// Built-in button component.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct UiButton {
    pub label: String,
    pub appearance: ButtonAppearance,
    pub size: ButtonSize,
    pub shape: ButtonShape,
    pub icon: Option<PicusIcon>,
    pub icon_position: ButtonIconPosition,
}

impl UiButton {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            appearance: ButtonAppearance::default(),
            size: ButtonSize::default(),
            shape: ButtonShape::default(),
            icon: None,
            icon_position: ButtonIconPosition::Before,
        }
    }

    #[must_use]
    pub fn with_appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    #[must_use]
    pub fn with_size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn with_shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: PicusIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    #[must_use]
    pub fn with_icon_position(mut self, icon_position: ButtonIconPosition) -> Self {
        self.icon_position = icon_position;
        self
    }
}

impl UiComponentTemplate for UiButton {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::elements::project_button(component, ctx)
    }
}
