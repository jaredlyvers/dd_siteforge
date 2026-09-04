//! Details panel text, footer hints, and click-to-select.
use super::super::*;
use super::*;

fn is_footer_chrome(part: &str) -> bool {
    matches!(part, "F1:Help" | "F2:Theme" | "Esc:Close" | "Ctrl+Q:Quit")
}

const MOUSE_HINT: &str = "(mouse: click/scroll)";

impl App {
    pub(in crate::tui) fn footer_hint(&self, width: u16) -> String {
        if width == 0 {
            return String::new();
        }
        let overlay = self.overlay.is_some() || self.modal.is_some();
        let mut parts: Vec<&str> = vec!["F1:Help", "F2:Theme"];
        if overlay {
            parts.push("Esc:Close");
            parts.push("Ctrl+Q:Quit");
        } else {
            match self.selected_sidebar_section {
                SidebarSection::Pages => {
                    if width < 80 {
                        parts.push("Ctrl+Q:Quit");
                        parts.push("r:Rename");
                    } else {
                        parts.extend_from_slice(&[
                            "Shift+A:Add",
                            "Shift+X:Del",
                            "u:Undo-page",
                            "r:Rename",
                            "Shift+J/K:Move",
                            "Ctrl+Q:Quit",
                        ]);
                    }
                }
                SidebarSection::Regions => {
                    if width < 80 {
                        parts.push("Ctrl+Q:Quit");
                        parts.push("Enter:Edit");
                    } else {
                        parts.extend_from_slice(&[
                            "j/k:Header/Footer",
                            "Enter:Edit",
                            "Ctrl+Q:Quit",
                        ]);
                    }
                }
                SidebarSection::Layouts => {
                    if width < 80 {
                        parts.push("Ctrl+Q:Quit");
                        parts.push("Enter:Edit");
                    } else if width < 110 {
                        parts.extend_from_slice(&[
                            "/:Insert",
                            "d:Del",
                            "y:Dup",
                            "u:Undo-tree",
                            "r:Col-id",
                            "J/K:Move",
                            "Ctrl+Q:Quit",
                        ]);
                    } else {
                        parts.extend_from_slice(&[
                            "/:Insert",
                            "Enter:Edit",
                            "d:Del",
                            "y:Dup",
                            "u:Undo-tree",
                            "r:Col-id",
                            "J/K:Move",
                            "p:Preview",
                            "Ctrl+Q:Quit",
                        ]);
                    }
                }
                SidebarSection::Details => {
                    if width < 80 {
                        parts.push("Ctrl+Q:Quit");
                        parts.push("j/k:Scroll");
                    } else {
                        parts.extend_from_slice(&[
                            "j/k:Scroll",
                            "Enter:Edit",
                            "Ctrl+Q:Quit",
                        ]);
                    }
                }
            }
        }
        let prefix = if self.dirty { "*  " } else { "" };
        let max = width as usize;
        // Drop trailing scoped tokens until chrome+actions fit. Never clip mid-key.
        // Mouse is appended only when that line already fits, so widening cannot hide a key.
        loop {
            let joined = format!("{prefix}{}", parts.join("  "));
            if joined.chars().count() <= max {
                break;
            }
            if let Some(idx) = parts.iter().rposition(|p| !is_footer_chrome(p)) {
                parts.remove(idx);
            } else if !parts.is_empty() {
                parts.pop();
            } else {
                return String::new();
            }
        }
        if !overlay && width >= 110 {
            let with_mouse = format!("{prefix}{}  {MOUSE_HINT}", parts.join("  "));
            if with_mouse.chars().count() <= max {
                parts.push(MOUSE_HINT);
            }
        }
        format!("{prefix}{}", parts.join("  "))
    }
    pub(in crate::tui) fn details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
        match self.selected_region {
            SelectedRegion::Header => (self.header_details_text(detail_width), vec![]),
            SelectedRegion::Footer => (self.footer_details_text(detail_width), vec![]),
            SelectedRegion::Page => self.page_details_text(detail_width),
        }
    }
    pub(in crate::tui) fn header_details_text(&self, detail_width: usize) -> String {
        let mut out = Vec::new();
        out.push("Site header".to_string());
        out.push(String::new());
        let marker = if matches!(self.selected_region, SelectedRegion::Header) {
            "*"
        } else {
            " "
        };
        out.push(format!("{}[01] dd-header {}", marker, self.site.header.id));
        let (hmap, _h_hits) = header_ascii_map(
            &self.site.header,
            self.selected_header_section,
            self.selected_header_column,
            detail_width,
        );
        out.push(hmap);
        out.push(String::new());
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.header_selection_summary(),
            self.component_kind.label()
        ));
        out.join("\n")
    }
    pub(in crate::tui) fn footer_details_text(&self, detail_width: usize) -> String {
        let mut out = Vec::new();
        out.push("Site footer".to_string());
        out.push(String::new());
        let marker = if matches!(self.selected_region, SelectedRegion::Footer) {
            "*"
        } else {
            " "
        };
        out.push(format!("{}[01] dd-footer {}", marker, self.site.footer.id));
        let (fmap, _f_hits) = footer_ascii_map(
            &self.site.footer,
            self.selected_header_section,
            self.selected_header_column,
            detail_width,
        );
        out.push(fmap);
        out.join("\n")
    }
    pub(in crate::tui) fn page_details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return ("No nodes on this page.".to_string(), vec![]);
        }
        let mut out = Vec::new();
        let mut out_hits: Vec<Vec<(usize, usize, usize, usize)>> = vec![];
        out.push(format!("Page blueprint: {}", page.head.title));
        out_hits.push(vec![]);
        out.push(String::new());
        out_hits.push(vec![]);
        for (idx, node) in page.nodes.iter().enumerate() {
            let marker = if idx == self.selected_node { "*" } else { " " };
            match node {
                PageNode::Hero(v) => {
                    out.push(format!("{marker}[{:02}] dd-hero", idx + 1,));
                    out_hits.push(vec![]);
                    let hmap = hero_ascii_map(v, detail_width);
                    for l in hmap.lines() {
                        out.push(l.to_string());
                        out_hits.push(vec![]);
                    }
                }
                PageNode::Section(v) => {
                    out.push(format!("{marker}[{:02}] dd-section {}", idx + 1, v.id));
                    out_hits.push(vec![]);
                    let (sec_str, sec_hits) = section_ascii_map(
                        v,
                        if idx == self.selected_node {
                            self.selected_column
                        } else {
                            0
                        },
                        detail_width,
                    );
                    for (i, l) in sec_str.lines().enumerate() {
                        out.push(l.to_string());
                        out_hits.push(sec_hits.get(i).cloned().unwrap_or_default());
                    }
                }
            }
            out.push(String::new());
            out_hits.push(vec![]);
        }
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.selection_summary(),
            self.component_kind.label()
        ));
        out_hits.push(vec![]);
        (out.join("\n"), out_hits)
    }
    pub(in crate::tui) fn details_max_scroll(&self) -> usize {
        let visible_rows = self.details_area.height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return 0;
        }
        let detail_width = self.details_area.width.saturating_sub(2) as usize;
        if detail_width == 0 {
            return 0;
        }
        let (dtxt, _dhits) = self.details_text(detail_width);
        let total_rows = dtxt.lines().count().max(1);
        total_rows.saturating_sub(visible_rows)
    }
    pub(in crate::tui) fn scroll_details_by(&mut self, delta: isize) {
        let max_scroll = self.details_max_scroll() as isize;
        let next = self.details_scroll_row as isize + delta;
        self.details_scroll_row = next.clamp(0, max_scroll) as usize;
    }

    pub(in crate::tui) fn select_item_from_details_click(&mut self, text_line: usize, char_x: usize) {
        let detail_w = self.details_area.width.saturating_sub(2) as usize;
        if detail_w == 0 {
            return;
        }
        let (content, _content_hits_for_render) = self.details_text(detail_w);
        let lines: Vec<&str> = content.lines().collect();
        if text_line >= lines.len() {
            return;
        }
        match self.selected_region {
            SelectedRegion::Header => self.select_header_from_details_lines(&lines, text_line),
            SelectedRegion::Page => self.select_page_from_details_lines(&lines, text_line, char_x, detail_w),
            _ => return,
        }
        // Set tree row to the most specific (deepest) matching row for the selection level.
        // This makes tree highlight follow the clicked item, and double-click edit the right thing.
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let clicked_line = lines[text_line];
        // For decl lines, use MAX for lower levels so only the decl row matches predicate (ancestors do but we pick specific).
        let mut tcol = self.selected_column;
        let mut tcomp = self.selected_component;
        if clicked_line.contains('[')
            && (clicked_line.contains("dd-hero")
                || clicked_line.contains("dd-section")
                || clicked_line.contains("dd-header"))
        {
            tcol = usize::MAX;
            tcomp = usize::MAX;
        } else if clicked_line.contains("column: ") || clicked_line.contains("item: ") {
            tcomp = usize::MAX;
        }
        let matches = |r: &TreeRow| -> bool {
            match r.kind {
                TreeRowKind::HeaderRoot { .. } => true,
                TreeRowKind::HeaderSection { section_idx } => section_idx == self.selected_header_section,
                TreeRowKind::HeaderColumn { section_idx, column_idx } => {
                    section_idx == self.selected_header_section && column_idx == self.selected_header_column
                }
                TreeRowKind::HeaderComponent { section_idx, column_idx, component_idx } => {
                    section_idx == self.selected_header_section
                        && column_idx == self.selected_header_column
                        && component_idx == self.selected_header_component
                }
                TreeRowKind::Hero { node_idx } | TreeRowKind::Section { node_idx } => node_idx == self.selected_node,
                TreeRowKind::Column { node_idx, column_idx } => {
                    node_idx == self.selected_node && column_idx == tcol
                }
                TreeRowKind::Component { node_idx, column_idx, component_idx } => {
                    node_idx == self.selected_node && column_idx == tcol && component_idx == tcomp
                }
                _ => false,
            }
        };
        if let Some((i, _)) = rows.iter().enumerate().rev().find(|(_, r)| matches(r)) {
            self.selected_tree_row = i;
        }
    }

    pub(in crate::tui) fn select_page_from_details_lines(&mut self, lines: &[&str], up_to: usize, char_x: usize, detail_w: usize) {
        let mut node_idx = None;
        let mut col_idx = 0usize;
        let mut comp_idx = 0usize;
        let mut cols_since = 0usize;
        let mut comps_since = 0usize;
        for (_i, &l) in lines.iter().enumerate().take(up_to + 1) {
            if let Some(br) = l.find('[') {
                if let Some(er) = l[br + 1..].find(']') {
                    let ns = &l[br + 1..br + 1 + er];
                    if let Ok(n) = ns.trim().parse::<usize>() {
                        if l.contains("dd-hero") || l.contains("dd-section") {
                            node_idx = Some(n.saturating_sub(1));
                            cols_since = 0;
                            comps_since = 0;
                            col_idx = 0;
                            comp_idx = 0;
                        }
                    }
                }
            }
            if node_idx.is_some() {
                let t = l.trim();
                if t.contains("item: ") || t.contains(" column: ") {
                    col_idx = cols_since;
                    cols_since += 1;
                    comps_since = 0;
                    comp_idx = 0;
                }
                if t.contains("dd-") && !t.contains("dd-section") && !t.contains("dd-hero") {
                    comp_idx = comps_since;
                    comps_since += 1;
                }
            }
        }
        if let Some(n) = node_idx {
            let page = self.current_page();
            if n < page.nodes.len() {
                self.selected_node = n;
                self.selected_column = col_idx;
                self.selected_component = comp_idx;
            }
        }
        // Use precise component hit segments from generation (handles side-by-side column boxes correctly)
        let (_ , hits) = self.details_text(detail_w);  // re-get with same w; hits only for page
        if let Some(line_segs) = hits.get(up_to) {
            for &(x0, x1, c, cp) in line_segs {
                if char_x >= x0 && char_x < x1 {
                    if let Some(n) = node_idx.or(Some(self.selected_node)) {
                        let page = self.current_page();
                        if n < page.nodes.len() {
                            self.selected_node = n;
                            self.selected_column = c;
                            self.selected_component = cp;
                        }
                    }
                    break;
                }
            }
        }
    }

    pub(in crate::tui) fn select_header_from_details_lines(&mut self, lines: &[&str], up_to: usize) {
        let mut sec_idx = 0usize;
        let mut col_idx = 0usize;
        let mut comp_idx = 0usize;
        let mut secs = 0usize;
        let mut cols = 0usize;
        let mut comps = 0usize;
        for (_i, &l) in lines.iter().enumerate().take(up_to + 1) {
            let t = l.trim();
            if t.contains("section: ") {
                sec_idx = secs;
                secs += 1;
                cols = 0;
                comps = 0;
                col_idx = 0;
                comp_idx = 0;
            } else if t.contains("column: ") {
                col_idx = cols;
                cols += 1;
                comps = 0;
                comp_idx = 0;
            } else if t.contains("dd-") && !t.contains("section:") {
                comp_idx = comps;
                comps += 1;
            }
        }
        self.selected_header_section = sec_idx;
        self.selected_header_column = col_idx;
        self.selected_header_component = comp_idx;
    }
}
