use bevy_ecs::{entity::Entity, prelude::*};

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// An inline time picker that opens a clock/selector overlay panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTimePicker {
    /// Hour in 24-hour format (0–23).
    pub hour: u8,
    /// Minute (0–59).
    pub minute: u8,
    /// Second (0–59).
    pub second: u8,
    /// Whether to display and allow selection in 24-hour mode.
    /// When `false`, a 12-hour AM/PM selector is shown.
    pub use_24h: bool,
    /// Whether the time selector overlay panel is currently open.
    pub is_open: bool,
}

impl UiTimePicker {
    #[must_use]
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour: hour.min(23),
            minute: minute.min(59),
            second: second.min(59),
            use_24h: true,
            is_open: false,
        }
    }

    /// Switch to 12-hour (AM/PM) mode.
    #[must_use]
    pub fn with_12h(mut self) -> Self {
        self.use_24h = false;
        self
    }

    /// Return the hour in 12-hour format (1–12) along with whether it is PM.
    #[must_use]
    pub fn hour_12(&self) -> (u8, bool) {
        let is_pm = self.hour >= 12;
        let h12 = if self.hour == 0 {
            12
        } else if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };
        (h12, is_pm)
    }

    /// Build an `UiTimePicker` from a 12-hour value and AM/PM flag.
    #[must_use]
    pub fn from_12h(h12: u8, is_pm: bool, minute: u8, second: u8) -> Self {
        let mut h = h12.min(12).max(1);
        if is_pm {
            if h != 12 {
                h += 12;
            }
        } else if h == 12 {
            h = 0;
        }
        Self {
            hour: h.min(23),
            minute: minute.min(59),
            second: second.min(59),
            use_24h: false,
            is_open: false,
        }
    }
}

/// Floating time picker panel (rendered in the overlay layer).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTimePickerPanel {
    /// The [`UiTimePicker`] anchor entity this panel belongs to.
    pub anchor: Entity,
    /// Whether to show in 24-hour mode (matches the picker setting).
    pub use_24h: bool,
}

/// Emitted when the selected time changes in a [`UiTimePicker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTimePickerChanged {
    pub picker: Entity,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl UiComponentTemplate for UiTimePicker {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_time_picker(component, ctx)
    }
}

impl UiComponentTemplate for UiTimePickerPanel {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_time_picker_panel(component, ctx)
    }
}
