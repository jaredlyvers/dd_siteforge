//! Small TUI helpers.
use super::*;
pub(super) fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

pub(super) fn component_index(total: usize, selected_component: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_component.min(total - 1))
    }
}

pub(super) fn section_columns_ref(section: &crate::model::DdSection) -> Vec<SectionColumn> {
    section.columns.clone()
}

pub(super) fn normalize_section_columns(section: &mut crate::model::DdSection) {
    if section.columns.is_empty() {
        section.columns.push(SectionColumn {
            id: "column-1".to_string(),
            width_class: "dd-u-1-1".to_string(),
            components: Vec::new(),
        });
    }
}

pub(super) fn component_search_haystack(kind: ComponentKind) -> String {
    let label = kind.label();
    let underscore = label.replace('-', "_");
    let short = label
        .trim_start_matches("dd-")
        .replace('-', "_")
        .to_string();
    format!("{label} {underscore} {short}")
}

pub(super) fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q = query.to_ascii_lowercase();
    let t = text.to_ascii_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    if t.contains(&q) {
        return Some(1000 - (t.find(&q).unwrap_or(0) as i32));
    }
    let mut score = 0i32;
    let mut t_chars = t.chars().enumerate();
    let mut last_idx: Option<usize> = None;
    for qc in q.chars() {
        let mut found = None;
        for (idx, tc) in t_chars.by_ref() {
            if tc == qc {
                found = Some(idx);
                break;
            }
        }
        let Some(idx) = found else {
            return None;
        };
        score += 10;
        if let Some(prev) = last_idx {
            if idx == prev + 1 {
                score += 8;
            }
        }
        if idx == 0 {
            score += 6;
        }
        last_idx = Some(idx);
    }
    Some(score)
}

pub(super) fn next_section_id_for_page(page: &crate::model::Page) -> String {
    let mut used = HashSet::new();
    for node in &page.nodes {
        if let PageNode::Section(section) = node {
            if !section.id.trim().is_empty() {
                used.insert(section.id.clone());
            }
        }
    }
    let mut idx = 1usize;
    loop {
        let candidate = format!("section-{}", idx);
        if !used.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

pub(super) fn ensure_page_section_ids(page: &mut crate::model::Page) {
    let mut used = HashSet::new();
    let mut next_idx = 1usize;
    for node in &mut page.nodes {
        let PageNode::Section(section) = node else {
            continue;
        };
        let current = section.id.trim().to_string();
        if !current.is_empty() && !used.contains(&current) {
            used.insert(current);
            continue;
        }
        loop {
            let candidate = format!("section-{}", next_idx);
            next_idx += 1;
            if !used.contains(&candidate) {
                section.id = candidate.clone();
                used.insert(candidate);
                break;
            }
        }
    }
}

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}



pub(super) fn backup_path_for(path: &std::path::Path) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".backup");
    std::path::PathBuf::from(s)
}

pub(super) fn chrono_like_format(t: std::time::SystemTime) -> Option<String> {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}s since epoch", secs))
}

/// Spawn the OS-default opener on the given file path. Returns the spawn
/// error if the command can't be invoked. The browser may take time to
/// open after this returns; we don't wait.
///
/// All three stdio streams are redirected to /dev/null. Without this, any
/// output the opener (or its forked browser) writes to stdout/stderr lands
/// on the same TTY as the TUI in raw mode and scrambles the screen layout.
#[allow(dead_code)]
pub(super) fn open_in_browser(target: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd: Command;
    #[cfg(target_os = "linux")]
    {
        cmd = Command::new("xdg-open");
        cmd.arg(target);
    }
    #[cfg(target_os = "macos")]
    {
        cmd = Command::new("open");
        cmd.arg(target);
    }
    #[cfg(target_os = "windows")]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]).arg(target);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no known browser opener for this target",
        ));
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}


#[derive(Debug, Clone)]
pub(super) struct DirEntryRow {
    pub(super) name: String,
    pub(super) is_dir: bool,
}

/// List immediate children of `dir`, sorted: subdirs first (alpha), then
/// files (alpha). Hidden entries (leading dot) are skipped. Returns an
/// empty Vec when the directory is unreadable.
pub(super) fn list_dir_entries(dir: &std::path::Path) -> Vec<DirEntryRow> {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let row = DirEntryRow { name, is_dir };
        if is_dir {
            dirs.push(row);
        } else {
            files.push(row);
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.extend(files);
    dirs
}

/// Substring filter (case-insensitive). Empty filter passes all entries.
pub(super) fn filter_entries(entries: &[DirEntryRow], filter: &str) -> Vec<DirEntryRow> {
    if filter.is_empty() {
        return entries.to_vec();
    }
    let needle = filter.to_lowercase();
    entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

/// Substring filter for page (slug, title) pairs. Empty filter passes all.
pub(super) fn filter_pages(pages: &[(String, String)], filter: &str) -> Vec<(String, String)> {
    if filter.is_empty() {
        return pages.to_vec();
    }
    let needle = filter.to_lowercase();
    pages
        .iter()
        .filter(|(slug, title)| {
            title.to_lowercase().contains(&needle) || slug.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

/// Strip a leading `./` (and any extra `/`) from a user-supplied relative
/// path so joining against a base of `.` doesn't produce `././foo` paths.
/// Trailing slashes are also trimmed for consistent display.
pub(super) fn normalize_relative_path(raw: &str) -> String {
    let mut s = raw.trim();
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.trim_start_matches('/');
    }
    s.trim_end_matches('/').to_string()
}

/// Build a clean display path. Prefer `./<rel>` when the export sits inside
/// the site JSON's directory; otherwise fall back to the absolute-ish form.
pub(super) fn display_relative_path(
    _base: &std::path::Path,
    out: &std::path::Path,
    normalized: &str,
) -> String {
    if normalized.is_empty() {
        out.display().to_string()
    } else {
        format!("./{}/", normalized)
    }
}

