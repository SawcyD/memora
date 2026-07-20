//! Dynamic tray icon rendering.
//!
//! The icon is a ring meter that fills clockwise with physical memory usage.
//! Everything is rasterized by hand into a straight RGBA buffer: the shapes are
//! simple enough that a drawing crate and a font rasterizer would be far more
//! dependency than this needs, and hand-drawn digits stay crisp at tray sizes
//! where a scaled-down vector font turns to mush.

use crate::system::accent::Rgb;

/// Rendered at 32x32; Windows downscales to whatever the tray is using. Larger
/// source sizes look worse, not better, because the ring gets thinner relative
/// to the downscale filter.
pub const SIZE: u32 = 32;

const CENTER: f32 = SIZE as f32 / 2.0;
const RING_OUTER: f32 = 15.0;
const RING_WIDTH: f32 = 3.6;
/// 3x3 samples per pixel. Enough to hide stairstepping on the arc at 32px.
const SUPERSAMPLE: u32 = 3;

/// Usage bands. The tray changes color only at these boundaries, never per
/// percent, so the icon stays visually quiet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageState {
    Normal,
    Elevated,
    High,
    Critical,
}

impl UsageState {
    /// Each band gets its own user-configurable threshold. Deriving `high` from
    /// `warning` looks tempting but cannot reproduce the spec's bands, which are
    /// 15 apart at the bottom and 10 at the top.
    pub fn from_percent(pct: u8, warning: u8, high: u8, critical: u8) -> Self {
        if pct >= critical {
            UsageState::Critical
        } else if pct >= high {
            UsageState::High
        } else if pct >= warning {
            UsageState::Elevated
        } else {
            UsageState::Normal
        }
    }

    /// Normal and elevated ride the system accent. Only the genuinely alarming
    /// bands depart from it, using the Windows caution/critical colors.
    fn color(self, accent: Rgb) -> Rgb {
        match self {
            UsageState::Normal | UsageState::Elevated => accent,
            UsageState::High => (0xf7, 0x63, 0x0c),
            UsageState::Critical => (0xe8, 0x11, 0x23),
        }
    }
}

/// 3x5 bitmap digits, MSB-left in the low 3 bits of each row.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

struct Canvas {
    px: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            px: vec![0; (SIZE * SIZE * 4) as usize],
        }
    }

    /// Source-over blend of a straight-alpha color onto the canvas.
    fn blend(&mut self, x: u32, y: u32, (r, g, b): Rgb, a: f32) {
        if a <= 0.0 || x >= SIZE || y >= SIZE {
            return;
        }
        let a = a.min(1.0);
        let i = ((y * SIZE + x) * 4) as usize;
        let dst_a = self.px[i + 3] as f32 / 255.0;
        let out_a = a + dst_a * (1.0 - a);
        if out_a <= 0.0 {
            return;
        }

        let mix = |src: u8, dst: u8| -> u8 {
            let s = src as f32 / 255.0;
            let d = dst as f32 / 255.0;
            (((s * a + d * dst_a * (1.0 - a)) / out_a) * 255.0).round() as u8
        };

        self.px[i] = mix(r, self.px[i]);
        self.px[i + 1] = mix(g, self.px[i + 1]);
        self.px[i + 2] = mix(b, self.px[i + 2]);
        self.px[i + 3] = (out_a * 255.0).round() as u8;
    }
}

/// Coverage of one pixel by the ring, restricted to `sweep` radians measured
/// clockwise from 12 o'clock. `sweep` of `TAU` is a full ring.
fn ring_coverage(x: u32, y: u32, sweep: f32) -> f32 {
    let inner = RING_OUTER - RING_WIDTH;
    let n = SUPERSAMPLE;
    let mut hits = 0u32;

    for sy in 0..n {
        for sx in 0..n {
            let px = x as f32 + (sx as f32 + 0.5) / n as f32 - CENTER;
            let py = y as f32 + (sy as f32 + 0.5) / n as f32 - CENTER;
            let dist = (px * px + py * py).sqrt();
            if dist < inner || dist > RING_OUTER {
                continue;
            }
            if sweep >= std::f32::consts::TAU {
                hits += 1;
                continue;
            }
            // atan2 with the axes swapped and y negated gives clockwise-from-top.
            let mut ang = px.atan2(-py);
            if ang < 0.0 {
                ang += std::f32::consts::TAU;
            }
            if ang <= sweep {
                hits += 1;
            }
        }
    }

    hits as f32 / (n * n) as f32
}

/// Draws a bitmap glyph at 2x scale with its top-left at (`ox`, `oy`).
fn draw_glyph(c: &mut Canvas, glyph: &[u8; 5], ox: i32, oy: i32, color: Rgb) {
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..3 {
            if bits & (0b100 >> col) == 0 {
                continue;
            }
            for dy in 0..2 {
                for dx in 0..2 {
                    let x = ox + col as i32 * 2 + dx;
                    let y = oy + row as i32 * 2 + dy;
                    if x >= 0 && y >= 0 {
                        c.blend(x as u32, y as u32, color, 1.0);
                    }
                }
            }
        }
    }
}

/// Renders the meter for `percent`, returning a 32x32 RGBA buffer.
///
/// `percent` is clamped to 0..=100. At 100 the digits render as "99+" because
/// three full-width digits do not fit inside the ring.
pub fn render(percent: u8, state: UsageState, accent: Rgb, show_digits: bool) -> Vec<u8> {
    let percent = percent.min(100);
    let color = state.color(accent);
    let mut c = Canvas::new();

    // Track: the unfilled remainder, faint so the filled arc reads as the value.
    for y in 0..SIZE {
        for x in 0..SIZE {
            let cov = ring_coverage(x, y, std::f32::consts::TAU);
            c.blend(x, y, color, cov * 0.22);
        }
    }

    // Fill: clockwise from 12 o'clock.
    let sweep = (percent as f32 / 100.0) * std::f32::consts::TAU;
    if sweep > 0.0 {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let cov = ring_coverage(x, y, sweep);
                c.blend(x, y, color, cov);
            }
        }
    }

    // At 100 the ring is already solid and there is no room for three glyphs.
    // "99+" would need a narrower font and "9+" reads as nine percent, so the
    // spec's other option applies: a completely filled icon, no digits.
    if show_digits && percent < 100 {
        let glyphs: [&[u8; 5]; 2] = if percent >= 10 {
            [&DIGITS[(percent / 10) as usize], &DIGITS[(percent % 10) as usize]]
        } else {
            // Single digit: centered, no leading zero.
            let ox = CENTER as i32 - 3;
            draw_glyph(&mut c, &DIGITS[percent as usize], ox, CENTER as i32 - 5, color);
            return c.px;
        };

        // Two 6px glyphs with a 1px gap = 13px, centered.
        let ox = CENTER as i32 - 6;
        draw_glyph(&mut c, glyphs[0], ox, CENTER as i32 - 5, color);
        draw_glyph(&mut c, glyphs[1], ox + 7, CENTER as i32 - 5, color);
    }

    c.px
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENT: Rgb = (0x4c, 0xc2, 0xff);

    fn alpha_sum(buf: &[u8]) -> u64 {
        buf.chunks_exact(4).map(|p| p[3] as u64).sum()
    }

    #[test]
    fn buffer_is_correctly_sized() {
        let buf = render(50, UsageState::Normal, ACCENT, true);
        assert_eq!(buf.len(), (SIZE * SIZE * 4) as usize);
    }

    #[test]
    fn fill_increases_with_percent() {
        // More usage must mean more ink, monotonically, or the meter lies.
        let mut prev = 0;
        for pct in [0u8, 25, 50, 75, 100] {
            let ink = alpha_sum(&render(pct, UsageState::Normal, ACCENT, false));
            assert!(ink > prev, "{pct}% produced no more ink than the previous step");
            prev = ink;
        }
    }

    #[test]
    fn center_is_transparent_without_digits() {
        let buf = render(100, UsageState::Normal, ACCENT, false);
        let i = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
        assert_eq!(buf[i + 3], 0, "ring center must be hollow");
    }

    #[test]
    fn digits_mark_the_center() {
        let with = alpha_sum(&render(63, UsageState::Normal, ACCENT, true));
        let without = alpha_sum(&render(63, UsageState::Normal, ACCENT, false));
        assert!(with > without, "digits should add ink");
    }

    /// 100 must not render two glyphs: "9+" reads as nine percent.
    #[test]
    fn full_usage_drops_the_digits() {
        let full = render(100, UsageState::Critical, ACCENT, true);
        let bare = render(100, UsageState::Critical, ACCENT, false);
        assert_eq!(full, bare, "100% must render as a filled ring with no digits");
    }

    #[test]
    fn percent_is_clamped() {
        let a = render(100, UsageState::Critical, ACCENT, true);
        let b = render(255, UsageState::Critical, ACCENT, true);
        assert_eq!(a, b, "over-100 input must clamp rather than wrap");
    }

    /// The spec's bands: 0–69 Normal, 70–84 Elevated, 85–94 High, 95–100 Critical.
    #[test]
    fn states_follow_thresholds() {
        let at = |p| UsageState::from_percent(p, 70, 85, 95);
        assert_eq!(at(0), UsageState::Normal);
        assert_eq!(at(69), UsageState::Normal);
        assert_eq!(at(70), UsageState::Elevated);
        assert_eq!(at(84), UsageState::Elevated);
        assert_eq!(at(85), UsageState::High);
        assert_eq!(at(94), UsageState::High);
        assert_eq!(at(95), UsageState::Critical);
        assert_eq!(at(100), UsageState::Critical);
    }
}

/// Dumps sample icons as raw RGBA for visual inspection. Not part of the suite:
/// run with `cargo test -- --ignored dump_samples`.
#[cfg(test)]
#[test]
#[ignore]
fn dump_samples() {
    let dir = std::env::var("MEMORA_DUMP_DIR").expect("set MEMORA_DUMP_DIR");
    for pct in [0u8, 7, 25, 42, 63, 85, 97, 100] {
        let state = UsageState::from_percent(pct, 70, 85, 95);
        let buf = render(pct, state, (0x4c, 0xc2, 0xff), true);
        std::fs::write(format!("{dir}/icon-{pct:03}.rgba"), &buf).unwrap();
    }
}
