//! Pixel-level tray icon renderer, decoupled from any platform icon API.
//!
//! Returns raw RGBA bytes so callers (egui tray manager, Tauri shell, tests)
//! can adapt the result to their own icon type without pulling in extra deps.

use image::{ImageBuffer, Rgba, RgbaImage};

use super::icon::UsageLevel;

/// Side length of the generated tray icon in pixels.
pub const TRAY_ICON_SIZE: u32 = 32;

/// Render a usage-bar tray icon as raw RGBA bytes.
///
/// - `session_percent`: primary bar fill (0–100), colour-coded by [`UsageLevel`]
/// - `weekly_percent`: optional secondary bar fill (0–100). When `Some`, two thin
///   bars are drawn (session top, weekly bottom). When `None`, a single thick bar
///   is drawn instead.
/// - `has_error`: desaturate all bar colours to grey to signal an error/unknown state.
///
/// Returns `(rgba_bytes, width, height)` for a [`TRAY_ICON_SIZE`]×[`TRAY_ICON_SIZE`] icon.
pub fn render_bar_icon_rgba(
    session_percent: f64,
    weekly_percent: Option<f64>,
    has_error: bool,
) -> (Vec<u8>, u32, u32) {
    const SZ: u32 = TRAY_ICON_SIZE;
    let mut img: RgbaImage = ImageBuffer::new(SZ, SZ);

    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    let bg_alpha: u8 = if has_error { 180 } else { 255 };
    let bg_color = Rgba([60, 60, 70, bg_alpha]);
    for y in 2..SZ - 2 {
        for x in 2..SZ - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    let color_for = |percent: f64| -> (u8, u8, u8) {
        let (r, g, b) = UsageLevel::from_percent(percent).color();
        if has_error {
            let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            (gray, gray, gray)
        } else {
            (r, g, b)
        }
    };

    let bar_left = 4u32;
    let bar_right = SZ - 4;
    let bar_width = bar_right - bar_left;

    let fill_px = |pct: f64| ((pct.clamp(0.0, 100.0) / 100.0) * bar_width as f64) as u32;

    let mut draw_bar = |y_start: u32, y_end: u32, pct: f64| {
        let (r, g, b) = color_for(pct);
        let fill_end = (bar_left + fill_px(pct)).min(bar_right);
        for y in y_start..y_end {
            for x in bar_left..bar_right {
                img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
            }
        }
        for y in y_start..y_end {
            for x in bar_left..fill_end {
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
    };

    match weekly_percent {
        Some(weekly) => {
            draw_bar(8, 15, session_percent);  // session bar (top, thicker)
            draw_bar(18, 23, weekly);          // weekly bar (bottom, thinner)
        }
        None => {
            draw_bar(10, 22, session_percent); // single thick bar (centred)
        }
    }

    (img.into_raw(), SZ, SZ)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_correct_dimensions() {
        let (rgba, w, h) = render_bar_icon_rgba(50.0, None, false);
        assert_eq!(w, TRAY_ICON_SIZE);
        assert_eq!(h, TRAY_ICON_SIZE);
        assert_eq!(rgba.len() as u32, w * h * 4);
    }

    #[test]
    fn render_two_bar_has_correct_size() {
        let (rgba, w, h) = render_bar_icon_rgba(30.0, Some(60.0), false);
        assert_eq!(rgba.len() as u32, w * h * 4);
    }

    #[test]
    fn zero_fill_gives_gray_only_bar() {
        let (rgba, w, _h) = render_bar_icon_rgba(0.0, None, false);
        // Sample a pixel near the centre of the bar track area (y=16, x=8)
        let idx = ((16 * w + 8) * 4) as usize;
        // Should be the gray track colour, not a usage colour
        assert_eq!(rgba[idx], 80); // R
        assert_eq!(rgba[idx + 1], 80); // G
        assert_eq!(rgba[idx + 2], 90); // B
    }

    #[test]
    fn full_fill_gives_colored_bar() {
        let (rgba, w, _h) = render_bar_icon_rgba(100.0, None, false);
        // At 100% used the bar is at Critical level
        let idx = ((16 * w + 8) * 4) as usize;
        let (er, eg, eb) = UsageLevel::Critical.color();
        assert_eq!(rgba[idx], er);
        assert_eq!(rgba[idx + 1], eg);
        assert_eq!(rgba[idx + 2], eb);
    }

    #[test]
    fn error_state_desaturates_colors() {
        let (normal, _, _) = render_bar_icon_rgba(100.0, None, false);
        let (error, _, _) = render_bar_icon_rgba(100.0, None, true);
        // In error mode all three channels at the filled bar pixel should be equal (grey)
        let idx = ((16 * 32 + 8) * 4) as usize;
        assert_ne!(normal[idx], normal[idx + 1]); // colour has distinct channels
        assert_eq!(error[idx], error[idx + 1]);   // grey: R == G
        assert_eq!(error[idx + 1], error[idx + 2]); // grey: G == B
    }
}
