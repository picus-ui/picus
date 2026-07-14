use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// Label + control row for form layouts.
///
/// Spawn the field control (and optional trailing content) as ECS children.
/// The row projects as a horizontal flex: label column, then children.
///
/// Visual treatment comes from stylesheet rules for `UiFormRow` / classes such as
/// `form.row` and `form.row.label` — missing rules stay transparent (no framework
/// default fill).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct UiFormRow {
    /// Field caption shown to the left of the control.
    pub label: String,
    /// Optional fixed label column width in logical pixels.
    pub label_width: Option<f64>,
}

impl UiFormRow {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            label_width: None,
        }
    }

    #[must_use]
    pub fn with_label_width(mut self, width: f64) -> Self {
        self.label_width = Some(width);
        self
    }
}

impl UiComponentTemplate for UiFormRow {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_form_row(component, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_row_builder_sets_label_and_width() {
        let row = UiFormRow::new("Name").with_label_width(120.0);
        assert_eq!(row.label, "Name");
        assert_eq!(row.label_width, Some(120.0));
    }
}
