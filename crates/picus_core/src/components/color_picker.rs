use bevy_ecs::{entity::Entity, prelude::*};

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// An inline color picker that opens an overlay panel for color selection.
///
/// Layout and interaction follow WinUI `ColorPicker` (vertical orientation):
/// SV spectrum, hue slider, optional alpha slider, preview, hex, and RGB/opacity
/// channel text fields. There is no preset swatch grid.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiColorPicker {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Alpha channel (0 = transparent, 255 = opaque).
    pub a: u8,
    /// Whether the color picker overlay panel is currently open.
    pub is_open: bool,
    /// When true, show alpha slider / opacity field and accept `#AARRGGBB` hex
    /// (WinUI `IsAlphaEnabled`).
    pub alpha_enabled: bool,
}

impl Default for UiColorPicker {
    fn default() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
            is_open: false,
            // Match WinUI default: alpha editing off until requested.
            alpha_enabled: false,
        }
    }
}

impl UiColorPicker {
    /// Creates an opaque RGB color picker (`alpha_enabled = false`).
    #[must_use]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: 255,
            is_open: false,
            alpha_enabled: false,
        }
    }

    /// Creates an RGBA color picker with alpha editing enabled.
    #[must_use]
    pub fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r,
            g,
            b,
            a,
            is_open: false,
            alpha_enabled: true,
        }
    }

    /// Enables or disables alpha editing in the overlay panel.
    #[must_use]
    pub fn with_alpha_enabled(mut self, enabled: bool) -> Self {
        self.alpha_enabled = enabled;
        self
    }

    /// Sets the alpha channel (does not by itself enable alpha editing).
    #[must_use]
    pub fn with_alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }

    /// Formats the current color as WinUI-style hex:
    /// `#RRGGBB` when alpha is disabled, `#AARRGGBB` when enabled.
    #[must_use]
    pub fn hex_string(&self) -> String {
        format_color_hex(self.r, self.g, self.b, self.a, self.alpha_enabled)
    }

    /// Opacity as a 0..=100 percentage (rounded).
    #[must_use]
    pub fn opacity_percent(&self) -> u8 {
        ((self.a as f32 / 255.0) * 100.0).round().clamp(0.0, 100.0) as u8
    }
}

// ---------------------------------------------------------------------------
// Hex formatting / parsing (WinUI ColorPicker conventions)
// ---------------------------------------------------------------------------

/// Formats color hex. With alpha: `#AARRGGBB`; without: `#RRGGBB`.
#[must_use]
pub fn format_color_hex(r: u8, g: u8, b: u8, a: u8, alpha_enabled: bool) -> String {
    if alpha_enabled {
        format!("#{a:02X}{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}")
    }
}

/// Parsed hex color. Alpha is `None` when the input did not include an alpha digit group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHexColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: Option<u8>,
}

/// Parses `#RGB`, `#RRGGBB`, `#AARRGGBB` (optional leading `#`).
///
/// When only 6 hex digits are present, alpha is left as `None` so callers can
/// preserve the previous alpha channel.
#[must_use]
pub fn parse_color_hex(input: &str) -> Option<ParsedHexColor> {
    let hex = input.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(ParsedHexColor {
                r,
                g,
                b,
                a: None,
            })
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(ParsedHexColor {
                r,
                g,
                b,
                a: None,
            })
        }
        8 => {
            // WinUI alpha-enabled form: #AARRGGBB
            let a = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let r = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let g = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let b = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(ParsedHexColor {
                r,
                g,
                b,
                a: Some(a),
            })
        }
        _ => None,
    }
}

/// Parses an opacity field: `100`, `100%`, or `0.0`–`1.0` fraction.
#[must_use]
pub fn parse_opacity_text(input: &str) -> Option<u8> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    let percent_form = text.strip_suffix('%');
    if let Some(body) = percent_form {
        let value: f32 = body.trim().parse().ok()?;
        return Some(
            ((value.clamp(0.0, 100.0) / 100.0) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8,
        );
    }
    let value: f32 = text.parse().ok()?;
    if (0.0..=1.0).contains(&value) && text.contains('.') {
        // Fractional 0..1
        Some((value * 255.0).round().clamp(0.0, 255.0) as u8)
    } else if (0.0..=100.0).contains(&value) {
        // Bare percentage without %
        Some(((value / 100.0) * 255.0).round().clamp(0.0, 255.0) as u8)
    } else if (0.0..=255.0).contains(&value) {
        Some(value.round().clamp(0.0, 255.0) as u8)
    } else {
        None
    }
}

/// Parses a 0–255 channel text field.
#[must_use]
pub fn parse_channel_u8(input: &str) -> Option<u8> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    let value: i32 = text.parse().ok()?;
    if (0..=255).contains(&value) {
        Some(value as u8)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// RGB ↔ HSV conversions (f32, hue in degrees 0..360).
// ---------------------------------------------------------------------------

/// Convert sRGB (0..255) to HSV with hue in degrees (0..360), S/V in 0..1.
#[must_use]
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;
    let s = if max <= 0.0 { 0.0 } else { delta / max };

    let h = if delta <= 0.0 {
        0.0
    } else if (max - r).abs() < f32::EPSILON {
        // max is red
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f32::EPSILON {
        // max is green
        60.0 * ((b - r) / delta + 2.0)
    } else {
        // max is blue
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Convert HSV (hue in degrees 0..360, S/V in 0..1) to sRGB (0..255).
#[must_use]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let to_u8 = |c: f32| ((c + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

/// Convert a hue (degrees 0..360) to sRGB at full saturation/value.
#[must_use]
pub fn hue_to_rgb(h: f32) -> (u8, u8, u8) {
    hsv_to_rgb(h, 1.0, 1.0)
}

/// Floating color picker panel (rendered in the overlay layer).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiColorPickerPanel {
    /// The [`UiColorPicker`] anchor entity this panel belongs to.
    pub anchor: Entity,
}

impl Default for UiColorPickerPanel {
    fn default() -> Self {
        Self {
            anchor: Entity::PLACEHOLDER,
        }
    }
}

/// Emitted when the selected color changes in a [`UiColorPicker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiColorPickerChanged {
    pub picker: Entity,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Color channel targeted by panel text input (WinUI RGB + Opacity fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerChannel {
    Red,
    Green,
    Blue,
    /// Opacity as percent / fraction / 0–255 (see [`parse_opacity_text`]).
    Alpha,
}

impl UiComponentTemplate for UiColorPicker {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_color_picker(component, ctx)
    }
}

impl UiComponentTemplate for UiColorPickerPanel {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_color_picker_panel(component, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_hex_rgb_and_argb() {
        assert_eq!(format_color_hex(0x60, 0xA5, 0xFA, 0xFF, false), "#60A5FA");
        assert_eq!(
            format_color_hex(0x60, 0xA5, 0xFA, 0x80, true),
            "#8060A5FA"
        );
    }

    #[test]
    fn parse_hex_variants() {
        assert_eq!(
            parse_color_hex("#60A5FA"),
            Some(ParsedHexColor {
                r: 0x60,
                g: 0xA5,
                b: 0xFA,
                a: None
            })
        );
        assert_eq!(
            parse_color_hex("8060A5FA"),
            Some(ParsedHexColor {
                r: 0x60,
                g: 0xA5,
                b: 0xFA,
                a: Some(0x80)
            })
        );
        assert_eq!(
            parse_color_hex("#FAB"),
            Some(ParsedHexColor {
                r: 0xFF,
                g: 0xAA,
                b: 0xBB,
                a: None
            })
        );
        assert!(parse_color_hex("xyz").is_none());
    }

    #[test]
    fn parse_opacity_forms() {
        assert_eq!(parse_opacity_text("100%"), Some(255));
        assert_eq!(parse_opacity_text("50%"), Some(128));
        assert_eq!(parse_opacity_text("0.5"), Some(128));
        assert_eq!(parse_opacity_text("50"), Some(128));
    }
}
