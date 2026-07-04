//! Icon view helper rendering a Lucide glyph in a fixed-size box.
//!
//! This is the public application-facing counterpart of the internal
//! `picus_core::projection::elements::create_icon_view`. It lets Picus
//! applications render icons directly inside their own projection functions
//! (e.g. for custom buttons, status indicators, sidebar rows) without
//! re-implementing the Lucide-font label dance.
//!
//! The helper composes the existing public `label` view with the Lucide font
//! family and a sized-box bounding constraint, so it tracks the same rendering
//! path as every other label and needs no private widget APIs.

use masonry_core::layout::{Dim, Length};
use masonry_core::palette::Color;
use picus_core::icons::{LUCIDE_FONT_FAMILY, PicusIcon};

use crate::view::{label, sized_box};
use crate::style::Style as _;

/// Preferred family name exposed by the bundled Lucide font.
pub use picus_core::icons::LUCIDE_FONT_FAMILY;

/// Render a Lucide [`PicusIcon`] glyph as a fixed-size icon view.
///
/// `size_px` is the bounding box; the glyph itself is drawn at ~90% to leave
/// optical padding. `color` is the glyph color (use
/// [`masonry_core::palette::css::WHITE`] or a theme-resolved color).
///
/// # Example
/// ```
/// # use picus_view as xilem;
/// # use xilem::picus_widget::palette;
/// use xilem::view::{icon, FlexExt as _, flex_row, label};
/// use picus_core::icons::PicusIcon;
/// # fn view() -> impl xilem::WidgetView<()> {
/// flex_row(vec![
///     icon(PicusIcon::Send, 18.0, palette::css::WHITE).into_any_flex(),
///     label("Send").into_any_flex(),
/// ])
/// # }
/// ```
#[must_use = "View values do nothing unless provided to Xilem."]
pub fn icon(picus_icon: PicusIcon, size_px: f64, color: Color) -> crate::view::SizedBox<crate::view::Label> {
    icon_glyph(picus_icon.glyph(), size_px, color)
}

/// Like [`icon`] but takes a raw Lucide `char` glyph, for icons not covered
/// by [`PicusIcon`].
pub fn icon_glyph(
    glyph: char,
    size_px: f64,
    color: Color,
) -> crate::view::SizedBox<crate::view::Label> {
    let icon_label = label(glyph.to_string())
        .text_size((size_px * 0.90) as f32)
        .font(LUCIDE_FONT_FAMILY.to_string())
        .color(color);
    sized_box(icon_label)
        .width(Dim::Fixed(Length::px(size_px)))
        .height(Dim::Fixed(Length::px(size_px)))
}