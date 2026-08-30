//! Details panel ASCII maps and text.
use super::*;
pub(super) fn section_ascii_map(
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

pub(super) fn header_ascii_map(
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

pub(super) fn footer_ascii_map(
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

pub(super) fn card_items_ascii_lines(
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

pub(super) fn section_item_ascii_inner_width(width_class: &str, section_inner_width: usize) -> usize {
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

pub(super) fn resolve_dd_u_ratio_for_panel(width_class: &str, panel_chars: usize) -> Option<(usize, usize)> {
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

pub(super) fn parse_dd_u_token_ratio(token: &str) -> Option<(ResponsiveBp, (usize, usize))> {
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

#[derive(Clone, Copy)]
pub(super) enum ResponsiveBp {
    Base,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl ResponsiveBp {
    fn index(self) -> usize {
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

pub(super) fn breakpoint_for_panel_chars(panel_chars: usize) -> ResponsiveBp {
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

pub(super) fn hero_ascii_map(hero: &crate::model::DdHero, panel_width: usize) -> String {
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
                "aos: {}",
                parent_data_aos_to_str(hero.parent_data_aos.unwrap_or(crate::model::HeroAos::FadeIn))
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

pub(super) fn section_class_to_str(v: crate::model::SectionClass) -> &'static str {
    match v {
        crate::model::SectionClass::Contained => "-contained",
        crate::model::SectionClass::ContainedMd => "-contained-md",
        crate::model::SectionClass::ContainedLg => "-contained-lg",
        crate::model::SectionClass::ContainedXl => "-contained-xl",
        crate::model::SectionClass::ContainedXxl => "-contained-xxl",
        crate::model::SectionClass::FullFull => "-full-full",
        crate::model::SectionClass::FullContained => "-full-contained",
        crate::model::SectionClass::FullContainedMd => "-full-contained-md",
        crate::model::SectionClass::FullContainedLg => "-full-contained-lg",
        crate::model::SectionClass::FullContainedXl => "-full-contained-xl",
        crate::model::SectionClass::FullContainedXxl => "-full-contained-xxl",
    }
}

#[allow(dead_code)]
pub(super) fn next_alert_type(current: crate::model::AlertType, forward: bool) -> crate::model::AlertType {
    use crate::model::AlertType;
    let all = [
        AlertType::Default,
        AlertType::Info,
        AlertType::Warning,
        AlertType::Error,
        AlertType::Success,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

#[allow(dead_code)]
pub(super) fn next_alert_class(current: crate::model::AlertClass, forward: bool) -> crate::model::AlertClass {
    use crate::model::AlertClass;
    let all = [AlertClass::Default, AlertClass::Compact];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

pub(super) fn fit_ascii_cell(value: &str, width: usize) -> String {
    let shortened = truncate_ascii(value, width);
    format!("{shortened:<width$}")
}

pub(super) fn input_lines_preserve(s: &str) -> Vec<String> {
    s.split('\n').map(|line| line.to_string()).collect()
}

pub(super) fn cursor_from_row_col(lines: &[String], target_row: usize, target_col: usize) -> usize {
    let row = target_row.min(lines.len().saturating_sub(1));
    let mut cursor = 0usize;
    for line in lines.iter().take(row) {
        cursor += line.chars().count() + 1;
    }
    let line_len = lines.get(row).map(|line| line.chars().count()).unwrap_or(0);
    cursor + target_col.min(line_len)
}


pub(super) fn truncate_ascii(value: &str, max_chars: usize) -> String {
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

pub(super) fn component_label(component: &crate::model::SectionComponent) -> &'static str {
    match component {
        crate::model::SectionComponent::Cta(_) => "dd-cta",
        crate::model::SectionComponent::Filmstrip(_) => "dd-filmstrip",
        crate::model::SectionComponent::Milestones(_) => "dd-milestones",
        crate::model::SectionComponent::Slider(_) => "dd-slider",
        crate::model::SectionComponent::Modal(_) => "dd-modal",
        crate::model::SectionComponent::Banner(_) => "dd-banner",
        crate::model::SectionComponent::Card(_) => "dd-card",
        crate::model::SectionComponent::Blockquote(_) => "dd-blockquote",
        crate::model::SectionComponent::Accordion(_) => "dd-accordion",
        crate::model::SectionComponent::Alternating(_) => "dd-alternating",
        crate::model::SectionComponent::Alert(_) => "dd-alert",
        crate::model::SectionComponent::Image(_) => "dd-image",
        crate::model::SectionComponent::RichText(_) => "dd-rich_text",
        crate::model::SectionComponent::Navigation(_) => "dd-navigation",
        crate::model::SectionComponent::HeaderSearch(_) => "dd-header-search",
        crate::model::SectionComponent::HeaderMenu(_) => "dd-header-menu",
    }
}

pub(super) fn component_blueprint_label(component: &crate::model::SectionComponent) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => {
            format!("dd-cta | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Filmstrip(v) => format!(
            "dd-filmstrip | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Milestones(v) => format!(
            "dd-milestones | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Slider(v) => format!(
            "dd-slider | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Modal(v) => {
            format!("dd-modal | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Accordion(v) => format!(
            "dd-accordion | accordion_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Alternating(v) => format!(
            "dd-alternating | alternating_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Card(v) => format!(
            "dd-card | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Blockquote(v) => format!(
            "dd-blockquote | parent_name: {} | parent_role: {}",
            v.parent_name, v.parent_role
        ),
        _ => component_label(component).to_string(),
    }
}

pub(super) fn hero_image_class_to_str(v: crate::model::HeroImageClass) -> &'static str {
    match v {
        crate::model::HeroImageClass::Contained => "-contained",
        crate::model::HeroImageClass::ContainedMd => "-contained-md",
        crate::model::HeroImageClass::ContainedLg => "-contained-lg",
        crate::model::HeroImageClass::ContainedXl => "-contained-xl",
        crate::model::HeroImageClass::ContainedXxl => "-contained-xxl",
        crate::model::HeroImageClass::FullFull => "-full-full",
        crate::model::HeroImageClass::FullContained => "-full-contained",
        crate::model::HeroImageClass::FullContainedMd => "-full-contained-md",
        crate::model::HeroImageClass::FullContainedLg => "-full-contained-lg",
        crate::model::HeroImageClass::FullContainedXl => "-full-contained-xl",
        crate::model::HeroImageClass::FullContainedXxl => "-full-contained-xxl",
    }
}

pub(super) fn parent_data_aos_to_str(v: crate::model::HeroAos) -> &'static str {
    match v {
        crate::model::HeroAos::FadeIn => "fade-in",
        crate::model::HeroAos::FadeUp => "fade-up",
        crate::model::HeroAos::FadeRight => "fade-right",
        crate::model::HeroAos::FadeDown => "fade-down",
        crate::model::HeroAos::FadeLeft => "fade-left",
        crate::model::HeroAos::ZoomIn => "zoom-in",
        crate::model::HeroAos::ZoomInUp => "zoom-in-up",
        crate::model::HeroAos::ZoomInDown => "zoom-in-down",
    }
}

#[allow(dead_code)]
pub(super) fn next_navigation_type(
    current: crate::model::NavigationType,
    forward: bool,
) -> crate::model::NavigationType {
    use crate::model::NavigationType;
    let all = [NavigationType::HeaderNav, NavigationType::FooterNav];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(super) fn next_navigation_class(
    current: crate::model::NavigationClass,
    forward: bool,
) -> crate::model::NavigationClass {
    use crate::model::NavigationClass;
    let all = [
        NavigationClass::MainMenu,
        NavigationClass::MenuSecondary,
        NavigationClass::MenuTertiary,
        NavigationClass::FooterMenu,
        NavigationClass::FooterMenuSecondary,
        NavigationClass::FooterMenuTertiary,
        NavigationClass::SocialMenu,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(super) fn navigation_kind_to_str(v: crate::model::NavigationKind) -> &'static str {
    match v {
        crate::model::NavigationKind::Link => "link",
        crate::model::NavigationKind::Button => "button",
    }
}

#[allow(dead_code)]
pub(super) fn parse_navigation_kind(raw: &str) -> Option<crate::model::NavigationKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" => Some(crate::model::NavigationKind::Link),
        "button" => Some(crate::model::NavigationKind::Button),
        _ => None,
    }
}

#[allow(dead_code)]
pub(super) fn next_navigation_kind(
    current: crate::model::NavigationKind,
    forward: bool,
) -> crate::model::NavigationKind {
    let _ = forward;
    match current {
        crate::model::NavigationKind::Link => crate::model::NavigationKind::Button,
        crate::model::NavigationKind::Button => crate::model::NavigationKind::Link,
    }
}

#[allow(dead_code)]
pub(super) fn robots_directive_to_str(v: crate::model::RobotsDirective) -> &'static str {
    match v {
        crate::model::RobotsDirective::IndexFollow => "index, follow",
        crate::model::RobotsDirective::NoindexFollow => "noindex, follow",
        crate::model::RobotsDirective::IndexNofollow => "index, nofollow",
        crate::model::RobotsDirective::NoindexNofollow => "noindex, nofollow",
    }
}

#[allow(dead_code)]
pub(super) fn next_robots_directive(
    current: crate::model::RobotsDirective,
    forward: bool,
) -> crate::model::RobotsDirective {
    use crate::model::RobotsDirective;
    let all = [
        RobotsDirective::IndexFollow,
        RobotsDirective::NoindexFollow,
        RobotsDirective::IndexNofollow,
        RobotsDirective::NoindexNofollow,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(super) fn schema_type_to_str(v: crate::model::SchemaType) -> &'static str {
    match v {
        crate::model::SchemaType::WebPage => "WebPage",
        crate::model::SchemaType::Article => "Article",
        crate::model::SchemaType::AboutPage => "AboutPage",
        crate::model::SchemaType::ContactPage => "ContactPage",
        crate::model::SchemaType::CollectionPage => "CollectionPage",
        crate::model::SchemaType::Organization => "Organization",
        crate::model::SchemaType::LocalBusiness => "LocalBusiness",
        crate::model::SchemaType::Product => "Product",
        crate::model::SchemaType::Service => "Service",
    }
}

#[allow(dead_code)]
pub(super) fn next_schema_type(
    current: crate::model::SchemaType,
    forward: bool,
) -> crate::model::SchemaType {
    use crate::model::SchemaType;
    let all = [
        SchemaType::WebPage,
        SchemaType::Article,
        SchemaType::AboutPage,
        SchemaType::ContactPage,
        SchemaType::CollectionPage,
        SchemaType::Organization,
        SchemaType::LocalBusiness,
        SchemaType::Product,
        SchemaType::Service,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}



impl App {
    pub(super) fn footer_hint(&self, width: u16) -> String {
        let parts: &[&str] = if self.modal.is_some() || self.show_help || self.show_theme {
            &["F1:Help", "Esc:Close", "Ctrl+Q:Quit"]
        } else {
            match self.selected_sidebar_section {
                SidebarSection::Pages => {
                    if width < 80 {
                        &["F1:Help", "Shift+A:Add", "r:Rename", "Ctrl+Q:Quit"]
                    } else {
                        &[
                            "F1:Help",
                            "Shift+A:Add",
                            "Shift+X:Del",
                            "u:Undo",
                            "r:Rename",
                            "Shift+J/K:Move",
                            "Ctrl+Q:Quit",
                        ]
                    }
                }
                SidebarSection::Regions => {
                    &["F1:Help", "j/k:Header/Footer", "Enter:Edit", "Ctrl+Q:Quit"]
                }
                SidebarSection::Layouts => {
                    if width < 80 {
                        &["F1:Help", "Enter:Edit", "d:Del", "y:Dup", "Ctrl+Q:Quit"]
                    } else if width < 110 {
                        &[
                            "F1:Help",
                            "/:Insert",
                            "d:Del",
                            "y:Dup",
                            "u:Undo",
                            "J/K:Move",
                            "Ctrl+Q:Quit",
                        ]
                    } else {
                        &[
                            "F1:Help",
                            "/:Insert",
                            "Enter:Edit",
                            "d:Del",
                            "y:Dup",
                            "u:Undo",
                            "J/K:Move",
                            "p:Preview",
                            "Ctrl+Q:Quit",
                        ]
                    }
                }
            }
        };
        let mut joined = parts.join("  ");
        if self.dirty {
            joined = format!("*  {joined}");
        }
        if width == 0 {
            return String::new();
        }
        joined.chars().take(width as usize).collect()
    }
    pub(super) fn details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
        match self.selected_region {
            SelectedRegion::Header => (self.header_details_text(detail_width), vec![]),
            SelectedRegion::Footer => (self.footer_details_text(detail_width), vec![]),
            SelectedRegion::Page => self.page_details_text(detail_width),
        }
    }
    pub(super) fn header_details_text(&self, detail_width: usize) -> String {
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
    pub(super) fn footer_details_text(&self, detail_width: usize) -> String {
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
    pub(super) fn page_details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
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
    pub(super) fn details_max_scroll(&self) -> usize {
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
    pub(super) fn scroll_details_by(&mut self, delta: isize) {
        let max_scroll = self.details_max_scroll() as isize;
        let next = self.details_scroll_row as isize + delta;
        self.details_scroll_row = next.clamp(0, max_scroll) as usize;
    }

    pub(super) fn select_item_from_details_click(&mut self, text_line: usize, char_x: usize) {
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

    pub(super) fn select_page_from_details_lines(&mut self, lines: &[&str], up_to: usize, char_x: usize, detail_w: usize) {
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

    pub(super) fn select_header_from_details_lines(&mut self, lines: &[&str], up_to: usize) {
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
