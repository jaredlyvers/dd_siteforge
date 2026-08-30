//! ASCII blueprints for hero, section, header, and footer.
use super::super::*;
use super::*;

pub(in crate::tui) fn section_ascii_map(
    section: &crate::model::DdSection,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
    const MAX_COMPONENT_ROWS: usize = 4;

    let inner_width = panel_width.saturating_sub(4).max(12);
    let columns = section_columns_ref(section);
    if columns.is_empty() {
        return ("(no columns)".to_string(), vec![]);
    }
    let active = selected_column.min(columns.len().saturating_sub(1));

    // column_data: per column (its box_lines, and per vertical line: which comp_idx it belongs to, or None for box headers/borders)
    let column_data: Vec<(Vec<String>, Vec<Option<usize>>)> = columns
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let marker = if idx == active { "*" } else { "-" };
            let item_inner_width = section_item_ascii_inner_width(&col.width_class, inner_width);
            let item_border = format!("+{}+", "-".repeat(item_inner_width + 2));
            let mut box_lines = vec![
                item_border.clone(),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("{marker} item: {}", col.id), item_inner_width)
                ),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("width: {}", col.width_class), item_inner_width)
                ),
            ];
            let mut box_comps: Vec<Option<usize>> = vec![None, None, None];
            if col.components.is_empty() {
                box_lines.push(format!(
                    "| {} |",
                    fit_ascii_cell("(empty)", item_inner_width)
                ));
                box_comps.push(None);
            } else {
                for (comp_i, component) in col.components.iter().take(MAX_COMPONENT_ROWS).enumerate() {
                    match component {
                        crate::model::SectionComponent::Card(card) => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell("- dd-card", item_inner_width)
                            ));
                            box_comps.push(Some(comp_i));
                            for line in card_items_ascii_lines(card, item_inner_width) {
                                box_lines.push(format!(
                                    "| {} |",
                                    fit_ascii_cell(&line, item_inner_width)
                                ));
                                box_comps.push(Some(comp_i));
                            }
                        }
                        _ => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell(
                                    &format!("- {}", component_blueprint_label(component)),
                                    item_inner_width
                                )
                            ));
                            box_comps.push(Some(comp_i));
                        }
                    }
                }
                let more = col.components.len().saturating_sub(MAX_COMPONENT_ROWS);
                if more > 0 {
                    box_lines.push(format!(
                        "| {} |",
                        fit_ascii_cell(&format!("+{more} more"), item_inner_width)
                    ));
                    box_comps.push(None);
                }
            }
            box_lines.push(item_border);
            box_comps.push(None);
            (box_lines, box_comps)
        })
        .collect::<Vec<_>>();

    // We will build annotations for the inner composed lines (before section outer | wrap)
    // Each inner line will have segments for components: (x0, x1, col, comp)
    let mut inner_composed_lines: Vec<String> = vec![];
    let mut inner_line_segments: Vec<Vec<(usize, usize, usize, usize)>> = vec![]; // x0,x1,col,comp per inner line

    // section header lines (inside the section ascii border)
    let section_header_lines = vec![
        fit_ascii_cell("SECTION", inner_width),
        fit_ascii_cell(&format!("id: {}", section.id), inner_width),
        fit_ascii_cell(
            &format!(
                "title: {}",
                section.section_title.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "class: {}",
                section_class_to_str(
                    section
                        .section_class
                        .unwrap_or(crate::model::SectionClass::FullContained)
                )
            ),
            inner_width,
        ),
        fit_ascii_cell("items:", inner_width),
    ];
    for hl in section_header_lines {
        inner_composed_lines.push(hl);
        inner_line_segments.push(vec![]);
    }

    let item_box_widths = column_data
        .iter()
        .map(|(bl, _)| bl.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 1usize;
    let mut row_groups: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_row_width = 0usize;
    for (idx, width) in item_box_widths.iter().copied().enumerate() {
        let next = if current_row.is_empty() {
            width
        } else {
            current_row_width + gap + width
        };
        if !current_row.is_empty() && next > inner_width {
            row_groups.push(current_row);
            current_row = vec![idx];
            current_row_width = width;
        } else {
            current_row.push(idx);
            current_row_width = next;
        }
    }
    if !current_row.is_empty() {
        row_groups.push(current_row);
    }

    for (row_idx, row) in row_groups.iter().enumerate() {
        if row_idx > 0 {
            inner_composed_lines.push("".to_string());
            inner_line_segments.push(vec![]);
        }
        let max_height = row
            .iter()
            .map(|idx| column_data[*idx].0.len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..max_height {
            let mut composed = String::new();
            let mut segs: Vec<(usize, usize, usize, usize)> = vec![];
            let mut cur_x = 0usize;
            for (pos, &col_idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str(" ");
                    cur_x += 1;
                }
                let (box_lines, box_comps) = &column_data[col_idx];
                let box_w = item_box_widths[col_idx];
                let part = box_lines
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_w));
                let part_start = cur_x;
                composed.push_str(&part);
                cur_x += part.chars().count();
                if let Some(cp) = box_comps.get(line_idx).copied().flatten() {
                    segs.push((part_start, cur_x, col_idx, cp));
                }
            }
            let fitted = fit_ascii_cell(&composed, inner_width);
            inner_composed_lines.push(fitted);
            inner_line_segments.push(segs);
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    let mut out_hits: Vec<Vec<(usize, usize, usize, usize)>> = vec![]; // final hits per out line
    out.push(border.clone());
    out_hits.push(vec![]);
    for (i, line) in inner_composed_lines.into_iter().enumerate() {
        let final_line = format!("| {} |", line);
        // adjust the inner segs x by +2 for the leading "| "
        let adjusted: Vec<(usize,usize,usize,usize)> = inner_line_segments[i]
            .iter()
            .map(|(x0,x1,c,cp)| (x0 + 2, x1 + 2, *c, *cp))
            .collect();
        out.push(final_line);
        out_hits.push(adjusted);
    }
    out.push(border.clone());
    out_hits.push(vec![]);
    (out.join("\n"), out_hits)
}

pub(in crate::tui) fn header_ascii_map(
    header: &crate::model::DdHeader,
    selected_section: usize,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
    let inner_width = panel_width.saturating_sub(4).max(12);

    let mut lines = vec![
        fit_ascii_cell("HEADER", inner_width),
        fit_ascii_cell(&format!("id: {}", header.id), inner_width),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                header.custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "alert: {}",
                if header.alert.is_some() { "yes" } else { "(none)" }
            ),
            inner_width,
        ),
        fit_ascii_cell("sections:", inner_width),
    ];

    if header.sections.is_empty() {
        lines.push(fit_ascii_cell(
            "(no sections - press '/' to add)",
            inner_width,
        ));
    } else {
        let active_section = selected_section.min(header.sections.len().saturating_sub(1));
        for (s_idx, section) in header.sections.iter().enumerate() {
            let s_marker = if s_idx == active_section { "*" } else { "-" };
            lines.push(fit_ascii_cell(
                &format!("{s_marker} section: {}", section.id),
                inner_width,
            ));

            if section.columns.is_empty() {
                lines.push(fit_ascii_cell("  (no columns)", inner_width));
            } else {
                let active_col = if s_idx == active_section {
                    selected_column.min(section.columns.len().saturating_sub(1))
                } else {
                    0
                };
                for (c_idx, col) in section.columns.iter().enumerate() {
                    let c_marker = if s_idx == active_section && c_idx == active_col {
                        "*"
                    } else {
                        "-"
                    };
                    lines.push(fit_ascii_cell(
                        &format!("  {c_marker} column: {} [{}]", col.id, col.width_class),
                        inner_width,
                    ));
                    if col.components.is_empty() {
                        lines.push(fit_ascii_cell("    (empty)", inner_width));
                    } else {
                        for comp in col.components.iter() {
                            lines.push(fit_ascii_cell(
                                &format!("    - {}", component_label(comp)),
                                inner_width,
                            ));
                        }
                    }
                }
            }
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    let s = out.join("\n");
    let hits = vec![vec![]; out.len()];
    (s, hits)
}

pub(in crate::tui) fn footer_ascii_map(
    footer: &crate::model::DdFooter,
    selected_section: usize,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
    let inner_width = panel_width.saturating_sub(4).max(12);
    let mut lines = vec![
        fit_ascii_cell("FOOTER", inner_width),
        fit_ascii_cell(&format!("id: {}", footer.id), inner_width),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                footer.custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell("sections:", inner_width),
    ];
    if footer.sections.is_empty() {
        lines.push(fit_ascii_cell(
            "(no sections - press '/' to add)",
            inner_width,
        ));
    } else {
        let active_section = selected_section.min(footer.sections.len().saturating_sub(1));
        for (s_idx, section) in footer.sections.iter().enumerate() {
            let s_marker = if s_idx == active_section { "*" } else { "-" };
            lines.push(fit_ascii_cell(
                &format!("{s_marker} section: {}", section.id),
                inner_width,
            ));
            if section.columns.is_empty() {
                lines.push(fit_ascii_cell("  (no columns)", inner_width));
            } else {
                let active_col = if s_idx == active_section {
                    selected_column.min(section.columns.len().saturating_sub(1))
                } else {
                    0
                };
                for (c_idx, col) in section.columns.iter().enumerate() {
                    let c_marker = if s_idx == active_section && c_idx == active_col {
                        "*"
                    } else {
                        "-"
                    };
                    lines.push(fit_ascii_cell(
                        &format!("  {c_marker} column: {} [{}]", col.id, col.width_class),
                        inner_width,
                    ));
                    if col.components.is_empty() {
                        lines.push(fit_ascii_cell("    (empty)", inner_width));
                    } else {
                        for comp in col.components.iter() {
                            lines.push(fit_ascii_cell(
                                &format!("    - {}", component_label(comp)),
                                inner_width,
                            ));
                        }
                    }
                }
            }
        }
    }
    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    let s = out.join("\n");
    let hits = vec![vec![]; out.len()];
    (s, hits)
}

pub(in crate::tui) fn card_items_ascii_lines(
    card: &crate::model::DdCard,
    container_inner_width: usize,
) -> Vec<String> {
    if card.items.is_empty() {
        return vec![fit_ascii_cell("(empty)", container_inner_width)];
    }

    let child_inner_width = section_item_ascii_inner_width(&card.parent_width, container_inner_width)
        .min(container_inner_width.saturating_sub(4))
        .max(10);
    let child_border = format!("+{}+", "-".repeat(child_inner_width + 2));

    let child_boxes = card
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            vec![
                child_border.clone(),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("card {}:", idx + 1), child_inner_width)
                ),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("title: {}", item.child_title), child_inner_width)
                ),
                child_border.clone(),
            ]
        })
        .collect::<Vec<_>>();

    let box_widths = child_boxes
        .iter()
        .map(|b| b.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 1usize;
    let mut row_groups: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_row_width = 0usize;
    for (idx, width) in box_widths.iter().copied().enumerate() {
        let next = if current_row.is_empty() {
            width
        } else {
            current_row_width + gap + width
        };
        if !current_row.is_empty() && next > container_inner_width {
            row_groups.push(current_row);
            current_row = vec![idx];
            current_row_width = width;
        } else {
            current_row.push(idx);
            current_row_width = next;
        }
    }
    if !current_row.is_empty() {
        row_groups.push(current_row);
    }

    let mut lines = Vec::new();
    for (row_idx, row) in row_groups.iter().enumerate() {
        if row_idx > 0 {
            lines.push(String::new());
        }
        let row_height = row
            .iter()
            .map(|idx| child_boxes[*idx].len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..row_height {
            let mut composed = String::new();
            for (pos, idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str("  ");
                }
                let part = child_boxes[*idx]
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_widths[*idx]));
                composed.push_str(&part);
            }
            lines.push(composed);
        }
    }
    lines
}

pub(in crate::tui) fn section_item_ascii_inner_width(width_class: &str, section_inner_width: usize) -> usize {
    let min_inner = 12usize;
    // Upper bound chosen so a full-width (ratio 1.0) box renders exactly the
    // same total row width as two half-width (ratio 0.5) boxes + 2-char gap:
    // both resolve to (section_inner_width - 2). Previously inner-10, which
    // left the 1-1 row 4 chars short and misaligned the right edge.
    let max_inner = section_inner_width.saturating_sub(6).max(min_inner);
    let ratio = resolve_dd_u_ratio_for_panel(width_class, section_inner_width)
        .map(|(num, den)| (num as f64 / den as f64).clamp(0.1, 1.0))
        .unwrap_or(1.0);

    // Compute using total box width first so row packing includes border/padding footprint.
    // Box width = inner + 4 (left/right borders + spaces).
    // Subtract a small safety margin to avoid rounding forcing 50/50 items onto separate rows.
    let box_target = ((section_inner_width as f64) * ratio).floor() as isize - 2;
    let inner_target = box_target - 4;
    (inner_target as usize).clamp(min_inner, max_inner)
}

pub(in crate::tui) fn resolve_dd_u_ratio_for_panel(width_class: &str, panel_chars: usize) -> Option<(usize, usize)> {
    let current_bp = breakpoint_for_panel_chars(panel_chars);
    let mut base: Option<(usize, usize)> = None;
    let mut sm: Option<(usize, usize)> = None;
    let mut md: Option<(usize, usize)> = None;
    let mut lg: Option<(usize, usize)> = None;
    let mut xl: Option<(usize, usize)> = None;
    let mut xxl: Option<(usize, usize)> = None;

    for token in width_class.split_whitespace() {
        match parse_dd_u_token_ratio(token) {
            Some((ResponsiveBp::Base, ratio)) => base = Some(ratio),
            Some((ResponsiveBp::Sm, ratio)) => sm = Some(ratio),
            Some((ResponsiveBp::Md, ratio)) => md = Some(ratio),
            Some((ResponsiveBp::Lg, ratio)) => lg = Some(ratio),
            Some((ResponsiveBp::Xl, ratio)) => xl = Some(ratio),
            Some((ResponsiveBp::Xxl, ratio)) => xxl = Some(ratio),
            None => {}
        }
    }

    let ordered = [base, sm, md, lg, xl, xxl];
    let idx = current_bp.index();
    for i in (0..=idx).rev() {
        if let Some(ratio) = ordered[i] {
            return Some(ratio);
        }
    }
    for ratio in ordered.iter().skip(idx + 1).flatten() {
        return Some(*ratio);
    }
    None
}

pub(in crate::tui) fn parse_dd_u_token_ratio(token: &str) -> Option<(ResponsiveBp, (usize, usize))> {
    let value = token.strip_prefix("dd-u-")?;
    let parts = value.split('-').collect::<Vec<_>>();
    let (bp, num_raw, den_raw) = match parts.as_slice() {
        [num, den] => (ResponsiveBp::Base, *num, *den),
        [bp, num, den] => (
            match *bp {
                "sm" => ResponsiveBp::Sm,
                "md" => ResponsiveBp::Md,
                "lg" => ResponsiveBp::Lg,
                "xl" => ResponsiveBp::Xl,
                "xxl" => ResponsiveBp::Xxl,
                _ => return None,
            },
            *num,
            *den,
        ),
        _ => return None,
    };
    let num = num_raw.parse::<usize>().ok()?;
    let den = den_raw.parse::<usize>().ok()?;
    if den == 0 || num == 0 {
        return None;
    }
    Some((bp, (num.min(den), den)))
}

pub(in crate::tui) fn breakpoint_for_panel_chars(panel_chars: usize) -> ResponsiveBp {
    if panel_chars >= 180 {
        ResponsiveBp::Xxl
    } else if panel_chars >= 150 {
        ResponsiveBp::Xl
    } else if panel_chars >= 120 {
        ResponsiveBp::Lg
    } else if panel_chars >= 90 {
        ResponsiveBp::Md
    } else if panel_chars >= 60 {
        ResponsiveBp::Sm
    } else {
        ResponsiveBp::Base
    }
}

pub(in crate::tui) fn hero_ascii_map(hero: &crate::model::DdHero, panel_width: usize) -> String {
    let inner_width = panel_width.saturating_sub(4).max(8);
    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let lines = [
        fit_ascii_cell("HERO", inner_width),
        fit_ascii_cell(
            &format!(
                "class: {}",
                hero_image_class_to_str(
                    hero.parent_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "sal: {}",
                sal_to_str(hero.sal.unwrap_or(crate::model::SalAnimation::Fade))
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                hero.parent_custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("title: {}", hero.parent_title), inner_width),
        fit_ascii_cell(&format!("subtitle: {}", hero.parent_subtitle), inner_width),
        fit_ascii_cell(
            &format!(
                "cta: {} -> {}",
                hero.link_1_label.as_deref().unwrap_or("(none)"),
                hero.link_1_url.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "cta_2: {} -> {}",
                hero.link_2_label.as_deref().unwrap_or("(none)"),
                hero.link_2_url.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("image: {}", hero.parent_image_url), inner_width),
    ];
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

pub(in crate::tui) fn fit_ascii_cell(value: &str, width: usize) -> String {
    let shortened = truncate_ascii(value, width);
    format!("{shortened:<width$}")
}

pub(in crate::tui) fn truncate_ascii(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return chars.into_iter().take(max_chars).collect();
    }
    let mut out = chars.into_iter().take(max_chars - 3).collect::<String>();
    out.push_str("...");
    out
}

#[derive(Clone, Copy)]
pub(in crate::tui) enum ResponsiveBp {
    Base,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl ResponsiveBp {
    pub(in crate::tui) fn index(self) -> usize {
        match self {
            ResponsiveBp::Base => 0,
            ResponsiveBp::Sm => 1,
            ResponsiveBp::Md => 2,
            ResponsiveBp::Lg => 3,
            ResponsiveBp::Xl => 4,
            ResponsiveBp::Xxl => 5,
        }
    }
}
