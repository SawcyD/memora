//! Windows system accent color, so Memora's primary action matches the rest of
//! the shell instead of inventing a brand color.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Accent {
    /// `#rrggbb`, ready to drop into a CSS custom property.
    pub accent: String,
    pub accent_light1: String,
    pub accent_light2: String,
    pub accent_dark1: String,
    /// True when the user has turned on high contrast; the UI drops its own
    /// accent styling and defers to system colors.
    pub high_contrast: bool,
}

impl Default for Accent {
    fn default() -> Self {
        // Windows' own default accent ("Blue"), used when UISettings is unavailable.
        Self {
            accent: "#0078d4".into(),
            accent_light1: "#0086f0".into(),
            accent_light2: "#4cc2ff".into(),
            accent_dark1: "#005a9e".into(),
            high_contrast: false,
        }
    }
}

#[cfg(windows)]
pub fn accent() -> Accent {
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    let Ok(settings) = UISettings::new() else {
        return Accent::default();
    };

    let hex = |ty: UIColorType, fallback: &str| -> String {
        settings
            .GetColorValue(ty)
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
            .unwrap_or_else(|_| fallback.to_string())
    };

    let fallback = Accent::default();
    Accent {
        accent: hex(UIColorType::Accent, &fallback.accent),
        accent_light1: hex(UIColorType::AccentLight1, &fallback.accent_light1),
        accent_light2: hex(UIColorType::AccentLight2, &fallback.accent_light2),
        accent_dark1: hex(UIColorType::AccentDark1, &fallback.accent_dark1),
        high_contrast: windows::UI::ViewManagement::AccessibilitySettings::new()
            .and_then(|a| a.HighContrast())
            .unwrap_or(false),
    }
}

#[cfg(not(windows))]
pub fn accent() -> Accent {
    Accent::default()
}
