//! Public icon view helper rendering a Lucide glyph in a fixed-size box.
//!
//! This is the application-facing counterpart of the internal
//! `projection::elements::create_icon_view`. It lets Picus applications
//! render icons directly inside their own projection functions (e.g. for
//! custom buttons, status indicators, sidebar rows) without re-implementing
//! the Lucide-font label dance.
//!
//! The helper composes the existing public `label` view with the Lucide font
//! family and a sized-box bounding constraint, so it tracks the same
//! rendering path as every other label and needs no private widget APIs.

use picus_view::masonry_core::layout::Length;
use picus_view::picus_widget::parley::{FontFamily, FontFamilyName};
use picus_view::picus_widget::peniko::Color;
use picus_view::view::{label, sized_box};
use picus_view::style::Style as _;
use picus_view::WidgetView;

use crate::icons::{LUCIDE_FONT_FAMILY, PicusIcon};

/// Render a Lucide [`PicusIcon`] glyph as a fixed-size icon view.
///
/// `size_px` is the bounding box; the glyph itself is drawn at ~90% to leave
/// optical padding. `color` is the glyph color (use
/// [`picus_view::picus_widget::peniko::Color`] or a theme-resolved color).
///
/// # Example
/// ```
/// # use picus_core::xilem as xilem;
/// use picus_core::icon::icon;
/// use picus_core::icons::PicusIcon;
/// use xilem::palette;
/// use xilem::view::{FlexExt as _, flex_row, label};
/// # fn view() -> impl xilem::WidgetView<()> {
/// flex_row(vec![
///     icon(PicusIcon::Send, 18.0, palette::css::WHITE).into_any_flex(),
///     label("Send").into_any_flex(),
/// ])
/// # }
/// ```
#[must_use = "View values do nothing unless provided to Xilem."]
pub fn icon(
    picus_icon: PicusIcon,
    size_px: f64,
    color: Color,
) -> impl WidgetView<()> {
    icon_glyph(picus_icon.glyph(), size_px, color)
}

/// Like [`icon`] but takes a raw Lucide `char` glyph, for icons not covered
/// by [`PicusIcon`].
pub fn icon_glyph(
    glyph: char,
    size_px: f64,
    color: Color,
) -> impl WidgetView<()> {
    let icon_label = label(glyph.to_string())
        .text_size((size_px * 0.90) as f32)
        .font(FontFamily::Single(FontFamilyName::Named(
            LUCIDE_FONT_FAMILY.into(),
        )))
        .color(color);
    sized_box(icon_label)
        .fixed_width(Length::px(size_px))
        .fixed_height(Length::px(size_px))
}