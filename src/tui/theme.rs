use std::path::{Path, PathBuf};

use ratatui::style::{Color, Style};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct AppTheme {
    // Core UI backgrounds
    pub(crate) background: Color,
    pub(crate) panel_background: Color,
    pub(crate) popup_background: Color,
    // Text colors
    pub(crate) foreground: Color,
    pub(crate) muted: Color,
    pub(crate) disabled: Color,
    pub(crate) text_inverse: Color,
    pub(crate) text_labels: Color,
    pub(crate) text_active_focus: Color,
    pub(crate) modal_labels: Color,
    pub(crate) modal_text: Color,
    pub(crate) modal_header: Color,
    // Accent colors
    pub(crate) title: Color,
    pub(crate) active: Color,
    // Border colors
    pub(crate) border: Color,
    pub(crate) border_active: Color,
    // Input field colors (split border vs text, default vs focus)
    pub(crate) input_border_default: Color,
    pub(crate) input_border_focus: Color,
    pub(crate) input_text_default: Color,
    pub(crate) input_text_focus: Color,
    pub(crate) cursor: Color,
    // Scrollbar colors
    pub(crate) scrollbar: Color,
    pub(crate) scrollbar_hover: Color,
    // Selection colors
    pub(crate) selected_background: Color,
    pub(crate) selected_foreground: Color,
    // Semantic colors
    pub(crate) success: Color,
    pub(crate) warning: Color,
    pub(crate) error: Color,
    pub(crate) info: Color,
    // File-role colors (LDNDDEV_TUI_VISUAL_STANDARD.md)
    pub(crate) folders: Color,
    pub(crate) files: Color,
    pub(crate) links: Color,
    // Backwards-compat aliases (used by older code paths that haven't been
    // migrated to the split border/text inputs yet).
    pub(crate) input_default: Color,
    pub(crate) input_focus: Color,
    pub(crate) app_shell: Style,
    pub(crate) active_border: Style,
    pub(crate) header_quotes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ThemeFile {
    #[serde(default)]
    version: Option<u32>,
    #[serde(default)]
    header_quotes: Vec<String>,
    colors: PaletteFile,
}

#[derive(Debug, Deserialize)]
struct PaletteFile {
    // Core backgrounds
    base_background: String,
    body_background: Option<String>,
    modal_background: Option<String>,
    // Text colors — canonical names from LDNDDEV_TUI_VISUAL_STANDARD.md; old
    // names kept as aliases for in-tree theme files.
    #[serde(alias = "text")]
    text_primary: String,
    #[serde(alias = "subtext0")]
    text_secondary: Option<String>,
    text_disabled: Option<String>,
    text_inverse: Option<String>,
    text_labels: Option<String>,
    text_active_focus: Option<String>,
    modal_labels: Option<String>,
    modal_text: Option<String>,
    modal_header: Option<String>,
    // Selection
    selected_background: String,
    // Borders
    border_default: String,
    border_active: Option<String>,
    // Scrollbar
    scrollbar: Option<String>,
    scrollbar_hover: Option<String>,
    // Input field colors — split for border vs text, default vs focus
    input_border_default: Option<String>,
    input_border_focus: Option<String>,
    input_text_default: Option<String>,
    input_text_focus: Option<String>,
    cursor: Option<String>,
    // Backwards-compat: plain input_default/input_focus still accepted
    input_default: Option<String>,
    input_focus: Option<String>,
    // Accent
    active: Option<String>,
    // Semantic
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    info: Option<String>,
    // File roles (currently unused in the TUI, kept for schema completeness)
    #[serde(default)]
    folders: Option<String>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    links: Option<String>,
}

impl AppTheme {
    pub(crate) fn load() -> (Self, String, Option<String>) {
        let candidates: Vec<(PathBuf, &'static str)> = {
            let mut c = vec![
                (PathBuf::from("dd_siteforge_theme.yml"), "local"),
            ];
            if let Some(home) = std::env::var_os("HOME") {
                let base = Path::new(&home).join(".config").join("ldnddev");
                c.push((base.join("dd_siteforge_theme.yml"), "global"));
            }
            c
        };

        let mut warning: Option<String> = None;

        for (path, src) in candidates {
            if !path.exists() {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(e) => {
                    warning = Some(format!("could not read '{}': {}", path.display(), e));
                    continue;
                }
            };
            let theme_file: ThemeFile = match serde_yaml::from_str(&raw) {
                Ok(f) => f,
                Err(e) => {
                    warning = Some(format!("invalid theme file '{}': {}", path.display(), e));
                    continue;
                }
            };

            // Strict version enforcement per LDNDDEV_TUI_VISUAL_STANDARD.md
            match theme_file.version {
                Some(1) => {}
                Some(v) => {
                    warning = Some(format!(
                        "theme '{}' declares version {} (expected 1); using built-in defaults",
                        path.display(),
                        v
                    ));
                    continue;
                }
                None => {
                    warning = Some(format!(
                        "theme '{}' is missing required 'version: 1'; using built-in defaults",
                        path.display()
                    ));
                    continue;
                }
            }

            let quotes = if !theme_file.header_quotes.is_empty() {
                theme_file.header_quotes
            } else {
                default_header_quotes()
            };

            match Self::from_palette(theme_file.colors, quotes) {
                Ok(t) => return (t, src.to_string(), warning),
                Err(e) => {
                    warning = Some(format!(
                        "theme '{}' color parse error: {}; using defaults",
                        path.display(),
                        e
                    ));
                    continue;
                }
            }
        }

        // Built-in fallback
        (Self::default(), "default".to_string(), warning)
    }

    fn from_palette(
        p: PaletteFile,
        header_quotes: Vec<String>,
    ) -> anyhow::Result<Self> {
        // Core backgrounds
        let background = parse_hex_color(p.base_background.as_str())?;
        let panel_background = parse_hex_color(
            p.body_background
                .as_deref()
                .unwrap_or(p.base_background.as_str()),
        )?;
        let popup_background = parse_hex_color(
            p.modal_background
                .as_deref()
                .unwrap_or(p.base_background.as_str()),
        )?;

        // Text colors
        let foreground = parse_hex_color(p.text_primary.as_str())?;
        let muted = parse_hex_color(p.text_secondary.as_deref().unwrap_or("#9ea3aa"))?;
        let disabled = parse_hex_color(p.text_disabled.as_deref().unwrap_or("#A0A4A8"))?;
        let text_inverse = parse_hex_color(p.text_inverse.as_deref().unwrap_or("#F9FAFB"))?;
        let text_labels = parse_hex_color(p.text_labels.as_deref().unwrap_or("#ffaf46"))?;
        let text_active_focus =
            parse_hex_color(p.text_active_focus.as_deref().unwrap_or("#64b4f5"))?;
        let modal_labels = parse_hex_color(p.modal_labels.as_deref().unwrap_or("#64b4f5"))?;
        let modal_text = parse_hex_color(p.modal_text.as_deref().unwrap_or(p.text_primary.as_str()))?;
        let modal_header = parse_hex_color(p.modal_header.as_deref().unwrap_or("#64b4f5"))?;

        // Selection
        let selected_background = parse_hex_color(p.selected_background.as_str())?;

        // Borders
        let border = parse_hex_color(p.border_default.as_str())?;
        let border_active = parse_hex_color(p.border_active.as_deref().unwrap_or("#64B4F5"))?;

        // Scrollbar
        let scrollbar = parse_hex_color(p.scrollbar.as_deref().unwrap_or("#ffa087"))?;
        let scrollbar_hover =
            parse_hex_color(p.scrollbar_hover.as_deref().unwrap_or("#64b4f5"))?;

        // Input field colors — prefer new split names; fall back to old input_default/input_focus.
        let input_border_default = parse_hex_color(
            p.input_border_default
                .as_deref()
                .or(p.input_default.as_deref())
                .unwrap_or(p.border_default.as_str()),
        )?;
        let input_border_focus = parse_hex_color(
            p.input_border_focus
                .as_deref()
                .or(p.input_focus.as_deref())
                .unwrap_or("#64b4f5"),
        )?;
        let input_text_default = parse_hex_color(
            p.input_text_default
                .as_deref()
                .or(p.input_default.as_deref())
                .unwrap_or(p.text_primary.as_str()),
        )?;
        let input_text_focus = parse_hex_color(
            p.input_text_focus
                .as_deref()
                .or(p.input_focus.as_deref())
                .unwrap_or("#64b4f5"),
        )?;
        let cursor = parse_hex_color(p.cursor.as_deref().unwrap_or("#64b4f5"))?;

        // Back-compat aliases (keep the old semantics for any untouched code paths).
        let input_default = input_border_default;
        let input_focus = input_border_focus;

        // Accents
        let title_seed = p
            .modal_labels
            .as_deref()
            .or(p.text_active_focus.as_deref())
            .or(p.input_border_focus.as_deref())
            .or(p.input_focus.as_deref())
            .unwrap_or(p.text_primary.as_str());
        let title = parse_hex_color(title_seed)?;
        let active = parse_hex_color(p.active.as_deref().unwrap_or("#64B4F5"))?;

        // Semantic
        let success = parse_hex_color(p.success.as_deref().unwrap_or("#82e0aa"))?;
        let warning = parse_hex_color(p.warning.as_deref().unwrap_or("#f5c469"))?;
        let error = parse_hex_color(p.error.as_deref().unwrap_or("#e57373"))?;
        let info = parse_hex_color(p.info.as_deref().unwrap_or("#5dade2"))?;

        // File roles (LDNDDEV_TUI_VISUAL_STANDARD.md)
        let folders = parse_hex_color(p.folders.as_deref().unwrap_or("#64b4f5"))?;
        let files = parse_hex_color(p.files.as_deref().unwrap_or("#ffaf46"))?;
        let links = parse_hex_color(p.links.as_deref().unwrap_or("#ffa087"))?;

        let app_shell = Style::default()
            .bg(background)
            .fg(foreground);
        let active_border = Style::default().fg(border_active);

        Ok(Self {
            background,
            panel_background,
            popup_background,
            foreground,
            muted,
            disabled,
            text_inverse,
            text_labels,
            text_active_focus,
            modal_labels,
            modal_text,
            modal_header,
            title,
            active,
            border,
            border_active,
            input_border_default,
            input_border_focus,
            input_text_default,
            input_text_focus,
            cursor,
            scrollbar,
            scrollbar_hover,
            selected_background,
            selected_foreground: foreground,
            success,
            warning,
            error,
            info,
            folders,
            files,
            links,
            input_default,
            input_focus,
            app_shell,
            active_border,
            header_quotes,
        })
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        let border_def = Color::Rgb(245, 246, 247);
        let border_focus = Color::Rgb(100, 180, 245);
        Self {
            background: Color::Rgb(15, 17, 20),
            panel_background: Color::Rgb(42, 45, 49),
            popup_background: Color::Rgb(28, 30, 33),
            foreground: Color::Rgb(245, 246, 247),
            muted: Color::Rgb(158, 163, 170),
            disabled: Color::Rgb(160, 164, 168),
            text_inverse: Color::Rgb(249, 250, 251),
            text_labels: Color::Rgb(255, 175, 70),
            text_active_focus: border_focus,
            modal_labels: border_focus,
            modal_text: Color::Rgb(245, 246, 247),
            modal_header: border_focus,
            title: border_focus,
            active: border_focus,
            border: border_def,
            border_active: border_focus,
            input_border_default: border_def,
            input_border_focus: border_focus,
            input_text_default: Color::Rgb(245, 246, 247),
            input_text_focus: border_focus,
            cursor: border_focus,
            scrollbar: Color::Rgb(255, 160, 135),
            scrollbar_hover: border_focus,
            selected_background: Color::Rgb(15, 17, 20),
            selected_foreground: Color::Rgb(245, 246, 247),
            success: Color::Rgb(130, 224, 170),
            warning: Color::Rgb(245, 196, 105),
            error: Color::Rgb(229, 115, 115),
            info: Color::Rgb(93, 173, 226),
            folders: Color::Rgb(100, 180, 245),
            files: Color::Rgb(255, 175, 70),
            links: Color::Rgb(255, 160, 135),
            input_default: border_def,
            input_focus: border_focus,
            app_shell: Style::default()
                .bg(Color::Rgb(15, 17, 20))
                .fg(Color::Rgb(245, 246, 247)),
            active_border: Style::default().fg(border_focus),
            header_quotes: default_header_quotes(),
        }
    }
}

pub(crate) fn default_header_quotes() -> Vec<String> {
    vec![
        "Drafts are just commits that lost their nerve.".to_string(),
        "Saved. Probably. Hopefully. Definitely. (It saved.).".to_string(),
        "This post is live, which means it's officially out of your hands.".to_string(),
        "Scheduled for later — future you can deal with the typos.".to_string(),
        "Deleted. We won't talk about it again. (We both saw it.)".to_string(),
    ]
}

pub(crate) fn choose_header_copy(quotes: &[String]) -> String {
    if quotes.is_empty() {
        return "Drafts are just commits that lost their nerve.".to_string();
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        ^ (std::process::id() as u64);
    quotes[(seed as usize) % quotes.len()].clone()
}

fn parse_hex_color(raw: &str) -> anyhow::Result<Color> {
    let hex = raw.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!(
            "expected hex color like '#RRGGBB', got '{}'",
            raw
        ));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

pub(crate) fn color_to_hex(c: Color) -> String {
    if let Color::Rgb(r, g, b) = c {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        "?".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn required_only_palette() -> PaletteFile {
        PaletteFile {
            base_background: "#0F1114".into(),
            body_background: None,
            modal_background: None,
            text_primary: "#F5F6F7".into(),
            text_secondary: None,
            text_disabled: None,
            text_inverse: None,
            text_labels: None,
            text_active_focus: None,
            modal_labels: None,
            modal_text: None,
            modal_header: None,
            selected_background: "#0F1114".into(),
            border_default: "#F5F6F7".into(),
            border_active: None,
            scrollbar: None,
            scrollbar_hover: None,
            input_border_default: None,
            input_border_focus: None,
            input_text_default: None,
            input_text_focus: None,
            cursor: None,
            input_default: None,
            input_focus: None,
            active: None,
            success: None,
            warning: None,
            error: None,
            info: None,
            folders: None,
            files: None,
            links: None,
        }
    }

    #[test]
    fn from_palette_omitted_keys_use_family_hex() {
        let theme = AppTheme::from_palette(required_only_palette(), vec![]).unwrap();
        assert_eq!(color_to_hex(theme.success), "#82e0aa");
        assert_eq!(color_to_hex(theme.warning), "#f5c469");
        assert_eq!(color_to_hex(theme.error), "#e57373");
        assert_eq!(color_to_hex(theme.info), "#5dade2");
        assert_eq!(color_to_hex(theme.border_active), "#64b4f5");
        assert_eq!(color_to_hex(theme.text_inverse), "#f9fafb");
        assert_eq!(color_to_hex(theme.disabled), "#a0a4a8");
        assert_eq!(color_to_hex(theme.active), "#64b4f5");
        assert_eq!(color_to_hex(AppTheme::default().active), "#64b4f5");
    }
}
