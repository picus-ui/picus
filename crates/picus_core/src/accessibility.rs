//! Accessibility infrastructure for picus.
//!
//! Provides ECS components for marking entities with accessibility metadata
//! (roles, labels, values, states) and systems for synchronising this
//! metadata with the AccessKit accessibility tree.

use accesskit::{HasPopup, Node, Toggled};
use bevy_ecs::prelude::*;

use crate::events::UiEventQueue;

/// Accessibility role mapped to AccessKit roles.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AccessibleRole {
    Button,
    CheckBox,
    ComboBox,
    Dialog,
    Grid,
    Image,
    Label,
    Link,
    List,
    ListItem,
    Menu,
    MenuBar,
    MenuItem,
    ProgressBar,
    RadioButton,
    ScrollBar,
    Slider,
    SpinButton,
    Tab,
    TabList,
    Table,
    TextInput,
    Tooltip,
    Tree,
    TreeItem,
    Window,
    #[default]
    Unknown,
}

impl AccessibleRole {
    pub fn to_accesskit_role(&self) -> accesskit::Role {
        match self {
            Self::Button => accesskit::Role::Button,
            Self::CheckBox => accesskit::Role::CheckBox,
            Self::ComboBox => accesskit::Role::ComboBox,
            Self::Dialog => accesskit::Role::Dialog,
            Self::Grid => accesskit::Role::Grid,
            Self::Image => accesskit::Role::Image,
            Self::Label => accesskit::Role::Label,
            Self::Link => accesskit::Role::Link,
            Self::List => accesskit::Role::List,
            Self::ListItem => accesskit::Role::ListItem,
            Self::Menu => accesskit::Role::Menu,
            Self::MenuBar => accesskit::Role::MenuBar,
            Self::MenuItem => accesskit::Role::MenuItem,
            Self::ProgressBar => accesskit::Role::ProgressIndicator,
            Self::RadioButton => accesskit::Role::RadioButton,
            Self::ScrollBar => accesskit::Role::ScrollBar,
            Self::Slider => accesskit::Role::Slider,
            Self::SpinButton => accesskit::Role::SpinButton,
            Self::Tab => accesskit::Role::Tab,
            Self::TabList => accesskit::Role::TabList,
            Self::Table => accesskit::Role::Table,
            Self::TextInput => accesskit::Role::TextInput,
            Self::Tooltip => accesskit::Role::Tooltip,
            Self::Tree => accesskit::Role::Tree,
            Self::TreeItem => accesskit::Role::TreeItem,
            Self::Window => accesskit::Role::Window,
            Self::Unknown => accesskit::Role::Unknown,
        }
    }
}

/// Screen-reader label attached to an entity.
#[derive(Component, Debug, Clone, Default)]
pub struct AccessibleLabel(pub String);

/// Longer description for screen readers.
#[derive(Component, Debug, Clone, Default)]
pub struct AccessibleDescription(pub String);

/// Numeric value and range for sliders, progress bars, etc.
#[derive(Component, Debug, Clone)]
pub struct AccessibleValue {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl Default for AccessibleValue {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
        }
    }
}

/// Interactive state flags for accessibility.
#[derive(Component, Debug, Clone, Default)]
pub struct AccessibleState {
    pub disabled: bool,
    pub selected: bool,
    pub checked: Option<bool>,
    pub expanded: Option<bool>,
    pub has_popup: bool,
}

/// Accessibility action request from assistive technology.
#[derive(Debug, Clone)]
pub enum AccessibleAction {
    Click,
    Focus,
    SetValue(String),
    Expand,
    Collapse,
}

/// Global accessibility tree snapshot.
#[derive(Resource, Debug, Default)]
pub struct AccessibilityTree {
    pub nodes: Vec<(Entity, Node)>,
}

/// Build the accessibility tree from ECS components.
#[allow(clippy::type_complexity)]
pub fn sync_accessibility_tree(
    mut tree: ResMut<AccessibilityTree>,
    role_query: Query<(
        Entity,
        &AccessibleRole,
        Option<&AccessibleLabel>,
        Option<&AccessibleDescription>,
        Option<&AccessibleValue>,
        Option<&AccessibleState>,
    )>,
) {
    tree.nodes.clear();

    for (entity, role, label, description, value, state) in role_query.iter() {
        let mut node = Node::new(role.to_accesskit_role());

        if let Some(label) = label {
            node.set_label(label.0.as_str());
        }

        if let Some(desc) = description {
            node.set_description(desc.0.as_str());
        }

        if let Some(value) = value {
            node.set_value(value.value.to_string());
            node.set_numeric_value(value.value);
            node.set_min_numeric_value(value.min);
            node.set_max_numeric_value(value.max);
            node.set_numeric_value_step(value.step);
        }

        if let Some(state) = state {
            if state.disabled {
                node.set_disabled();
            }
            if state.selected {
                node.set_selected(true);
            }
            if let Some(checked) = state.checked {
                if checked {
                    node.set_toggled(Toggled::True);
                } else {
                    node.set_toggled(Toggled::False);
                }
            }
            if state.has_popup {
                node.set_has_popup(HasPopup::Menu);
            }
        }

        tree.nodes.push((entity, node));
    }
}

/// Dispatch incoming AccessKit action requests to the internal ECS action queue.
///
/// This system should be scheduled in `Update` and reads action requests
/// from the bevy_a11y accessibility resource, converting them into
/// [`AccessibleAction`] events that the dispatcher publishes as
/// `UiAction<AccessibleAction>` messages.
///
/// This is a framework-level dispatch; concrete components (buttons,
/// sliders, text inputs) should read `MessageReader<UiAction<AccessibleAction>>`
/// and perform the appropriate mutation.
pub fn handle_accessibility_actions(queue: Res<UiEventQueue>) {
    // AccessKit action requests arrive through the bevy_a11y
    // `AccessibilityRequested` resource.  In the current bevy 0.19
    // integration this resource is read per-frame and exposes
    // `ActionRequest` instances keyed by `Entity`.
    //
    // The following block is the dispatch stub.  When bevy_a11y
    // provides the `ActionRequest` stream directly (which varies
    // by platform and backend), uncomment and connect it:
    //
    // ```
    // if let Some(accessibility) = accessibility_requested.as_ref() {
    //     for (entity, action_request) in accessibility.drain() {
    //         let action = match action_request.action {
    //             accesskit::Action::Default | accesskit::Action::Invoke =>
    //                 AccessibleAction::Click,
    //             accesskit::Action::Focus =>
    //                 AccessibleAction::Focus,
    //             accesskit::Action::SetValue =>
    //                 AccessibleAction::SetValue(action_request.value.unwrap_or_default()),
    //             accesskit::Action::Expand =>
    //                 AccessibleAction::Expand,
    //             accesskit::Action::Collapse =>
    //                 AccessibleAction::Collapse,
    //             _ => continue,
    //         };
    //         queue.push_typed(entity, action);
    //     }
    // }
    // ```
    //
    // For now this system serves as a structural placeholder that
    // compiles and can be wired into the schedule without waiting
    // for the bevy_a11y ActionRequest bridge to stabilise.
    let _ = queue;
}
