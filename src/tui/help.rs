use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use super::theme::{color_to_hex, AppTheme};

fn wrap_to_lines(text: &str, width: usize) -> Vec<String> {
    let w = width.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut out: Vec<String> = vec![];
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
            if current.chars().count() > w {
                // hard-break very long token
                let chars: Vec<char> = current.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    let end = (i + w).min(chars.len());
                    out.push(chars[i..end].iter().collect());
                    i = end;
                }
                current.clear();
            }
            continue;
        }
        let with_space = format!("{} {}", current, word);
        if with_space.chars().count() <= w {
            current = with_space;
        } else {
            if !current.is_empty() {
                out.push(current);
            }
            current = word.to_string();
            if current.chars().count() > w {
                let chars: Vec<char> = current.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    let end = (i + w).min(chars.len());
                    out.push(chars[i..end].iter().collect());
                    i = end;
                }
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

pub(crate) fn build_help_text(theme: &AppTheme, width: usize) -> Text<'static> {
    let h_style = Style::default()
        .fg(theme.modal_header)
        .add_modifier(Modifier::BOLD);
    let k_style = Style::default().fg(theme.text_active_focus);
    let div_style = Style::default().fg(theme.text_secondary);

    const KEY_COL: usize = 22;

    fn add_section(
        lines: &mut Vec<Line<'static>>,
        title: &'static str,
        items: &[(&'static str, &'static str)],
        icon: &str,
        h_style: Style,
        k_style: Style,
        div_style: Style,
        width: usize,
    ) {
        lines.push(Line::from(Span::styled(title.to_string(), h_style)));
        lines.push(Line::from("")); // padding top inside section
        for (k, a) in items {
            let prefix = format!("  {} {:<18}", icon, k);
            let avail = width.saturating_sub(KEY_COL);
            let chunks = wrap_to_lines(a, avail);
            if chunks.is_empty() || (chunks.len() == 1 && chunks[0].is_empty()) {
                lines.push(Line::from(Span::styled(prefix, k_style)));
            } else {
                for (i, chunk) in chunks.iter().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(prefix.clone(), k_style),
                            Span::raw(chunk.clone()),
                        ]));
                    } else {
                        let cont = format!("{}{}", " ".repeat(KEY_COL), chunk);
                        lines.push(Line::from(Span::raw(cont)));
                    }
                }
            }
        }
        lines.push(Line::from("")); // padding bottom inside section

        // visible divider between sections (in muted)
        let rule_len = width.saturating_sub(4).clamp(12, 50);
        let rule = "─".repeat(rule_len);
        lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
        lines.push(Line::from("")); // breathing room after divider
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Global
    add_section(
        &mut lines,
        "Global",
        &[
            ("F1", "Open/close this help"),
            ("F2", "Open/close theme source + color details (F2:Theme)"),
            ("F3", "Validate site (shows errors in a modal)"),
            ("Shift+E", "Export site to HTML (validates first; prompts for output dir on first use)"),
            ("p", "Preview current page: export, start local HTTP server, open browser"),
            ("Ctrl+Q", "Quit"),
            ("s", "Open save modal and enter file path (also writes a .backup checkpoint)"),
            ("Tab / Shift+Tab", "Next/previous page"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    // Autosave (descriptive paragraph-style note under its header)
    lines.push(Line::from(Span::styled("Autosave", h_style)));
    lines.push(Line::from("")); // padding top inside section
    let note = "2s after a change, the active site JSON is rewritten. The .backup is only refreshed by manual `s` saves; on next load the TUI surfaces a toast when site.json and site.json.backup differ.";
    for chunk in wrap_to_lines(note, width.saturating_sub(2)) {
        lines.push(Line::from(Span::raw(format!("  {}", chunk))));
    }
    lines.push(Line::from("")); // padding bottom inside section
    // divider after autosave section
    let rule_len = width.saturating_sub(4).clamp(12, 50);
    let rule = "─".repeat(rule_len);
    lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
    lines.push(Line::from(""));

    // Node navigation and edits
    add_section(
        &mut lines,
        "Node navigation and edits",
        &[
            ("Up/Down or wheel", "Select row in Layout tree"),
            ("PageUp/PageDown", "Scroll Details blueprint panel"),
            ("Enter", "Edit selected row"),
            ("Space", "Expand/collapse selected section or accordion/alternating/card/filmstrip/milestones/slider items"),
            ("/", "Open insert fuzzy finder (hero/section/cta/.../slider); inserts after the selected row"),
            ("A / X", "Add/remove dd-accordion, dd-alternating, dd-card, dd-filmstrip, dd-milestones, or dd-slider item"),
            ("d", "Delete selected row (node, component, or collection item)"),
            ("y", "Duplicate selected row after the current one"),
            ("u", "Undo last tree edit (session snapshots, cap 20)"),
            ("J / K", "Move selected row down / up (node, component, item, or column)"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    // Pages panel
    add_section(
        &mut lines,
        "Pages panel ([2] Pages)",
        &[
            ("Shift+A", "Add page (title prompt → template picker: Blank / Hero only / Hero + Section / Duplicate)"),
            ("Shift+X", "Delete current page (confirms; refuses if only 1 page)"),
            ("u", "Undo last page deletion (session trash)"),
            ("Shift+J / Shift+K", "Move current page down / up (also = sitemap order)"),
            ("r", "Rename page (auto-slug until first disk save; locked pages expose a Slug field in [HEAD])"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    // Section layout
    add_section(
        &mut lines,
        "Section layout",
        &[
            ("C / V", "Add/remove selected column"),
            ("c / v", "Select previous/next column"),
            ("J / K", "Move selected grain down/up (column when a column row is selected)"),
            ("r / f", "Edit selected column id / width class"),
            ("Details pane", "Shows ASCII blueprint (click selects, double-click edits)"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    // Edit modal
    add_section(
        &mut lines,
        "Edit modal (unified FormEdit)",
        &[
            ("Tab / Shift+Tab", "Next/previous editable field"),
            ("Ctrl+P (image)", "Open image picker (./source/images/)"),
            ("Ctrl+P (link)", "Open page picker (lists site pages)"),
            ("←/→ (options)", "Cycle choices for type/option fields"),
            ("Enter", "Newline in textarea / next field / drill into SubForm item"),
            ("Ctrl+S", "Save"),
            ("Esc", "Cancel edit"),
            ("Backspace", "Delete character"),
            ("multiline ↑/↓/Enter", "Move/copy lines; Enter newline; Ctrl+S saves"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    // Mouse (using safe bullet icon)
    add_section(
        &mut lines,
        "Mouse controls",
        &[
            ("Click panel/list", "Select the row/item (Regions/Pages/Layout); Details click selects the matching tree grain"),
            ("Double-click item", "Edit (unified modal; works on page-head, header/footer roots, sections, columns, components)"),
            ("Click modal field", "Focus that input (click-to-focus in all FormEdit + legacy)"),
            ("Wheel over pane", "Scroll the pane under the cursor (Regions/Pages/Layout/Details)"),
            ("Scrollbar track/thumb", "Jump/scroll via custom painted │/█ scrollbar"),
            ("Wheel / drag scroll", "Scroll lists, Details, help modal, long form content"),
        ],
        "•",
        h_style,
        k_style,
        div_style,
        width,
    );

    Text::from(lines)
}

pub(crate) fn build_theme_text(theme: &AppTheme, source: &str, status: &Option<String>, width: usize) -> Text<'static> {
    let h_style = Style::default()
        .fg(theme.modal_header)
        .add_modifier(Modifier::BOLD);
    let k_style = Style::default().fg(theme.text_active_focus);
    let div_style = Style::default().fg(theme.text_secondary);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Theme section
    lines.push(Line::from(Span::styled("Theme", h_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  App: ", k_style),
        Span::raw(format!("dd_siteforge v{}", env!("CARGO_PKG_VERSION"))),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Source: ", k_style),
        Span::raw(format!("{}   (./dd_siteforge_theme.yml or equivalent)", source)),
    ]));
    let status_str = status.as_deref().unwrap_or("OK (loaded cleanly)");
    lines.push(Line::from(vec![
        Span::styled("  Status: ", k_style),
        Span::raw(status_str.to_string()),
    ]));
    lines.push(Line::from(""));

    // divider
    let rule_len = width.saturating_sub(4).clamp(12, 50);
    let rule = "─".repeat(rule_len);
    lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
    lines.push(Line::from(""));

    // Color tokens section
    lines.push(Line::from(Span::styled("Loaded color tokens (sampled)", h_style)));
    lines.push(Line::from(""));

    let tokens: Vec<(&str, Color, &str)> = vec![
        ("base_background", theme.base_background, "app_shell base"),
        ("body_background", theme.body_background, "content panes"),
        ("modal_background", theme.modal_background, "modals & popups"),
        ("text_primary", theme.text_primary, "primary text"),
        ("modal_header", theme.modal_header, "section titles bold"),
        ("text_labels", theme.text_labels, "labels default"),
        ("text_active_focus", theme.text_active_focus, "focus + keys"),
        ("input_border_focus", theme.input_border_focus, "focused inputs"),
        ("cursor", theme.cursor, "caret overlay"),
        ("success", theme.success, "success toasts"),
        ("warning", theme.warning, "warning toasts"),
        ("error", theme.error, "error toasts"),
        ("info", theme.info, "info toasts"),
        ("folders", theme.folders, "image picker folders"),
        ("files", theme.files, "image picker files"),
        ("links", theme.links, "image picker links"),
        ("scrollbar", theme.scrollbar, "scrollbars"),
        ("scrollbar_hover", theme.scrollbar_hover, "scrollbar thumb"),
    ];

    for (name, color, role) in tokens {
        let hex = color_to_hex(color);
        let line = format!("  {:<18} {}   ({})", name, hex, role);
        lines.push(Line::from(Span::raw(line)));
    }

    lines.push(Line::from(""));

    // final divider
    lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::raw("  (All colors from self.theme.*. No hardcodes.)")));

    Text::from(lines)
}

pub(crate) fn count_lines(text: &Text, _width: usize) -> usize {
    text.lines.len()
}
