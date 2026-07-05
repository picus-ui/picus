use crate::xilem::Color;
use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// Path segment used by [`UiCanvasCommand::FillPath`] and [`UiCanvasCommand::StrokePath`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiCanvasPathCommand {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    QuadTo {
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    CubicTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    ClosePath,
}

/// A color stop in a gradient brush.
///
/// `offset` is a normalized position in `[0.0, 1.0]` along the gradient axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiGradientStop {
    pub offset: f32,
    pub color: Color,
}

impl UiGradientStop {
    #[must_use]
    pub const fn new(offset: f32, color: Color) -> Self {
        Self { offset, color }
    }
}

impl From<(f32, Color)> for UiGradientStop {
    fn from((offset, color): (f32, Color)) -> Self {
        Self { offset, color }
    }
}

/// Primitive drawing command for [`UiCanvas`].
#[derive(Debug, Clone, PartialEq)]
pub enum UiCanvasCommand {
    FillCanvas {
        color: Color,
    },
    StrokeCanvas {
        color: Color,
        stroke_width: f64,
    },
    FillRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
    },
    FillRoundedRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        color: Color,
    },
    StrokeRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
        stroke_width: f64,
    },
    StrokeRoundedRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        color: Color,
        stroke_width: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: Color,
        stroke_width: f64,
    },
    FillCircle {
        cx: f64,
        cy: f64,
        radius: f64,
        color: Color,
    },
    StrokeCircle {
        cx: f64,
        cy: f64,
        radius: f64,
        color: Color,
        stroke_width: f64,
    },
    FillPath {
        commands: Vec<UiCanvasPathCommand>,
        color: Color,
    },
    StrokePath {
        commands: Vec<UiCanvasPathCommand>,
        color: Color,
        stroke_width: f64,
    },
    /// Fill a rectangle with a linear gradient between two points.
    FillLinearGradientRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        /// Start point of the gradient axis in canvas coordinates.
        start_x: f64,
        start_y: f64,
        /// End point of the gradient axis in canvas coordinates.
        end_x: f64,
        end_y: f64,
        stops: Vec<UiGradientStop>,
    },
    /// Fill a circle with a radial gradient radiating from a center point.
    FillRadialGradientCircle {
        cx: f64,
        cy: f64,
        radius: f64,
        /// Center and radius of the inner circle of the radial gradient.
        inner_radius: f64,
        stops: Vec<UiGradientStop>,
    },
}

/// Drawable surface backed by Masonry's native canvas widget.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct UiCanvas {
    pub alt_text: Option<String>,
    pub commands: Vec<UiCanvasCommand>,
    /// Logical size `(width, height)` of the canvas surface in pixels.
    ///
    /// When non-zero, enables `UiCanvasPosition::right`/`bottom` anchoring so
    /// children can be positioned relative to the far edges. When zero,
    /// only `left`/`top` offsets are applied.
    pub size: (f64, f64),
}

impl UiCanvas {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_alt_text(mut self, alt_text: impl Into<String>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }

    #[must_use]
    pub fn with_command(mut self, command: UiCanvasCommand) -> Self {
        self.commands.push(command);
        self
    }

    /// Set the logical canvas size used for `right`/`bottom` child anchoring.
    #[must_use]
    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.size = (width.max(0.0), height.max(0.0));
        self
    }

    pub fn push_command(&mut self, command: UiCanvasCommand) {
        self.commands.push(command);
    }
}

/// Absolute positioning metadata for a child inside [`UiCanvas`].
///
/// `left`/`top` are applied by the current projector. `right`/`bottom` are stored
/// as public layout intent for the custom canvas-panel layout that will be needed
/// to size children relative to the far edges.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct UiCanvasPosition {
    pub left: Option<f64>,
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
}

impl UiCanvasPosition {
    #[must_use]
    pub const fn new(left: f64, top: f64) -> Self {
        Self {
            left: Some(left),
            top: Some(top),
            right: None,
            bottom: None,
        }
    }

    #[must_use]
    pub const fn with_left(mut self, left: f64) -> Self {
        self.left = Some(left);
        self
    }

    #[must_use]
    pub const fn with_top(mut self, top: f64) -> Self {
        self.top = Some(top);
        self
    }

    #[must_use]
    pub const fn with_right(mut self, right: f64) -> Self {
        self.right = Some(right);
        self
    }

    #[must_use]
    pub const fn with_bottom(mut self, bottom: f64) -> Self {
        self.bottom = Some(bottom);
        self
    }

    /// Resolve the `(x, y)` translation for this position.
    ///
    /// When `canvas_size` is provided and `left`/`top` are unset, `right`/`bottom`
    /// anchor the child relative to the far edges of the canvas.
    #[must_use]
    pub fn offset(self, canvas_size: (f64, f64)) -> (f64, f64) {
        let (canvas_w, canvas_h) = canvas_size;
        let x = self
            .left
            .unwrap_or_else(|| (canvas_w - self.right.unwrap_or(0.0)).max(0.0));
        let y = self
            .top
            .unwrap_or_else(|| (canvas_h - self.bottom.unwrap_or(0.0)).max(0.0));
        (x, y)
    }
}

impl UiComponentTemplate for UiCanvas {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_canvas(component, ctx)
    }
}
