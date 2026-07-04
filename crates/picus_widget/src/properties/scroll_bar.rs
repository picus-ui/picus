use crate::core::Property;

/// Declares if the scroll bar collapses when not being hovered.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Collapsible(pub bool);

impl Property for Collapsible {
    fn static_default() -> &'static Self {
        static DEFAULT: Collapsible = Collapsible(false);
        &DEFAULT
    }
}
