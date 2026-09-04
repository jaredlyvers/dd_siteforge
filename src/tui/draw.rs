//! Frame layout: header, sidebar, details, footer, overlays.
use super::*;

impl App {
    pub(super) fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.prune_toasts();

        // Full-screen app shell base layer (base_background + text_primary).
        frame.render_widget(
            Block::default().style(self.theme.app_shell),
            frame.area(),
        );

        let page = self.current_page();
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // HEADER (fixed, per LDNDDEV standard)
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // FOOTER (fixed, decluttered keys only)
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(root[1]);

        // === NEW 3-line header (bordered, title=project, content=header_copy) ===
        let header_block = Block::default()
            .title("dd_siteforge")
            .borders(Borders::ALL)
            .border_style(self.theme.active_border)
            .style(self.theme.app_shell)
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(header_block.clone(), root[0]);

        if root[0].height >= 3 {
            let inner = header_block.inner(root[0]);
            let quote = Paragraph::new(self.header_copy.as_str()).style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.background),
            );
            frame.render_widget(quote, inner);
        }
        // (no mouse/keyboard capture for header — purely decorative)

        // Compute page context early so the & from current_page() does not live across later
        // self.* field assigns (list_area etc) in the panel rendering.
        let page_idx = self.selected_page + 1;
        let page_label = if page.head.title.trim().is_empty() {
            page.slug.as_str()
        } else {
            page.head.title.as_str()
        };
        let details_title = format!("Details — {:02}: {}", page_idx, page_label);

        // Split sidebar into three sections: Regions, Pages, Layouts
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Regions (Header, Footer)
                Constraint::Length(8), // Pages (numbered list, scrollable)
                Constraint::Min(1),    // Layouts (component tree)
            ])
            .split(main[0]);

        // Determine border colors based on active section
        let regions_border = if self.selected_sidebar_section == SidebarSection::Regions {
            self.theme.active
        } else {
            self.theme.border
        };
        let pages_border = if self.selected_sidebar_section == SidebarSection::Pages {
            self.theme.active
        } else {
            self.theme.border
        };
        let layouts_border = if self.selected_sidebar_section == SidebarSection::Layouts {
            self.theme.active
        } else {
            self.theme.border
        };

        // Regions section (Header, Footer)
        let regions_items: Vec<ListItem> = vec!["  Header", "  Footer"]
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                let is_selected = match self.selected_region {
                    SelectedRegion::Header => idx == 0,
                    SelectedRegion::Footer => idx == 1,
                    SelectedRegion::Page => false,
                };
                let style = if is_selected {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(*label).style(style)
            })
            .collect();
        let regions_list = List::new(regions_items)
            .block(
                Block::default()
                    .title("[1] Regions")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(regions_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut regions_state = ListState::default();
        let regions_selected = match self.selected_region {
            SelectedRegion::Header => Some(0),
            SelectedRegion::Footer => Some(1),
            SelectedRegion::Page => None,
        };
        regions_state.select(regions_selected);
        frame.render_stateful_widget(regions_list, sidebar[0], &mut regions_state);
        self.regions_area = sidebar[0];

        // Pages section (numbered list)
        let page_items: Vec<ListItem> = self
            .site
            .pages
            .iter()
            .enumerate()
            .map(|(idx, page)| {
                let num = format!("{:02}", idx + 1);
                let title = page.head.title.trim();
                let label_body = if title.is_empty() {
                    page.slug.as_str()
                } else {
                    title
                };
                let label = format!("{} {}", num, label_body);
                let style = if idx == self.selected_page {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(label).style(style)
            })
            .collect();
        let pages_title = if self.site.pages.is_empty() {
            "[2] Pages".to_string()
        } else {
            format!(
                "[2] Pages {}/{}",
                self.selected_page + 1,
                self.site.pages.len()
            )
        };
        let pages_list = List::new(page_items)
            .block(
                Block::default()
                    .title(pages_title)
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(pages_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        if !self.site.pages.is_empty() {
            self.pages_list_state.select(Some(self.selected_page));
        } else {
            self.pages_list_state.select(None);
        }
        frame.render_stateful_widget(pages_list, sidebar[1], &mut self.pages_list_state);
        self.pages_area = sidebar[1];

        // Layouts section (component tree)
        let tree_rows = self.build_tree_rows();
        let layout_items: Vec<ListItem> = tree_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let label = self.tree_row_label(row);
                let style = if idx == self.selected_tree_row {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(label).style(style)
            })
            .collect();
        let layouts_list = List::new(layout_items)
            .block(
                Block::default()
                    .title("[3] Layout")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(layouts_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        if !tree_rows.is_empty() {
            self.layout_list_state
                .select(Some(self.selected_tree_row.min(tree_rows.len() - 1)));
        } else {
            self.layout_list_state.select(None);
        }
        frame.render_stateful_widget(layouts_list, sidebar[2], &mut self.layout_list_state);
        self.list_area = sidebar[2];

        self.details_area = main[1];
        let details_width = main[1].width.saturating_sub(2) as usize;
        let (details_content, _details_hits) = self.details_text(details_width);
        let details_total_rows = details_content.lines().count().max(1);
        let details_visible_rows = main[1].height.saturating_sub(2) as usize;
        let details_max_scroll = details_total_rows.saturating_sub(details_visible_rows);
        self.details_scroll_row = self.details_scroll_row.min(details_max_scroll);

        let details = Paragraph::new(details_content)
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .block(
                Block::default()
                    .title(details_title)
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .scroll((self.details_scroll_row.min(u16::MAX as usize) as u16, 0))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, main[1]);

        // Scrollbar on the right edge of the Details panel — only painted
        // when the content exceeds the visible window. Track lives on the
        // last column inside the border; thumb height is proportional to
        // visible/total rows.
        self.details_scrollbar_track = ScrollbarTrack::default();
        if details_total_rows > details_visible_rows
            && main[1].width >= 3
            && main[1].height >= 4
        {
            let track = Rect {
                x: main[1].x + main[1].width.saturating_sub(2),
                y: main[1].y + 1,
                width: 1,
                height: main[1].height.saturating_sub(2),
            };
            self.details_scrollbar_track = ScrollbarTrack {
                rect: track,
                total: details_total_rows,
                visible: details_visible_rows,
            };
            paint_scrollbar(
                frame,
                track,
                self.details_scroll_row,
                details_total_rows,
                details_visible_rows,
                self.theme.scrollbar,
                self.theme.scrollbar_hover,
                self.theme.panel_background,
            );
        }

        let footer_text = self.footer_hint(root[2].width);
        let footer = Paragraph::new(footer_text).style(self.theme.app_shell);
        frame.render_widget(footer, root[2]);

        if self.show_help {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Key & Mouse bindings (F1 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.modal_header)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Reserve a 1-col gutter on the right for the scrollbar so the
            // body wraps before reaching the modal border.
            let scrollbar_width: u16 = 1;
            let body_w = inner.width.saturating_sub(scrollbar_width + 1);
            let body_area = Rect {
                x: inner.x,
                y: inner.y,
                width: body_w,
                height: inner.height,
            };

            // Build rich help (section headers in modal_header, 2-col key/action with
            // wrapping that preserves columns, icons, internal padding + dividers).
            let help = build_help_text(&self.theme, body_w as usize);
            let wrapped_total = count_wrapped_lines(&help, body_w as usize);
            let visible = inner.height as usize;
            let max_scroll = wrapped_total.saturating_sub(visible) as u16;
            // Publish the max so event handlers can clamp on key/wheel events.
            self.help_scroll_max = max_scroll;
            // Clamp on render too so an inflated stored value (e.g. from
            // pressing G/End before wrapping was known) snaps back into range.
            if self.help_scroll > max_scroll {
                self.help_scroll = max_scroll;
            }
            let scroll = self.help_scroll;

            let body = Paragraph::new(help)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(body, body_area);

            self.help_scrollbar_track = ScrollbarTrack::default();
            if wrapped_total > visible && inner.height > 0 {
                let track = Rect {
                    x: inner.x + inner.width.saturating_sub(1),
                    y: inner.y,
                    width: 1,
                    height: inner.height,
                };
                self.help_scrollbar_track = ScrollbarTrack {
                    rect: track,
                    total: wrapped_total,
                    visible,
                };
                paint_scrollbar(
                    frame,
                    track,
                    scroll as usize,
                    wrapped_total,
                    visible,
                    self.theme.scrollbar,
                    self.theme.scrollbar_hover,
                    self.theme.popup_background,
                );
            }
        } else {
            self.help_scrollbar_track = ScrollbarTrack::default();
        }

        if self.show_theme {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Theme (F2 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.modal_header)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let scrollbar_width: u16 = 1;
            let body_w = inner.width.saturating_sub(scrollbar_width + 1);
            let body_area = Rect {
                x: inner.x,
                y: inner.y,
                width: body_w,
                height: inner.height,
            };

            let help = build_theme_text(&self.theme, &self.theme_source, &self.theme_status, body_w as usize);
            let wrapped_total = count_wrapped_lines(&help, body_w as usize);
            let visible = inner.height as usize;
            let max_scroll = wrapped_total.saturating_sub(visible) as u16;
            self.theme_scroll_max = max_scroll;
            if self.theme_scroll > max_scroll {
                self.theme_scroll = max_scroll;
            }
            let scroll = self.theme_scroll;

            let body = Paragraph::new(help)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(body, body_area);

            self.theme_scrollbar_track = ScrollbarTrack::default();
            if wrapped_total > visible && inner.height > 0 {
                let track = Rect {
                    x: inner.x + inner.width.saturating_sub(1),
                    y: inner.y,
                    width: 1,
                    height: inner.height,
                };
                self.theme_scrollbar_track = ScrollbarTrack {
                    rect: track,
                    total: wrapped_total,
                    visible,
                };
                paint_scrollbar(
                    frame,
                    track,
                    scroll as usize,
                    wrapped_total,
                    visible,
                    self.theme.scrollbar,
                    self.theme.scrollbar_hover,
                    self.theme.popup_background,
                );
            }
        } else {
            self.theme_scrollbar_track = ScrollbarTrack::default();
        }

        // Render unified modal if open (handles all modal types)
        self.render_modal(frame);

        // Toasts paint last so they float above everything except the
        // active-input cursor overlay.
        self.render_toasts(frame, frame.area());

        let cursor_overlay = self.set_cursor_for_active_input(frame);
        if let Some((x, y, ch)) = cursor_overlay {
            let cursor_cell = Paragraph::new(ch.to_string()).style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(
                cursor_cell,
                Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }

    pub(super) fn set_cursor_for_active_input(&self, frame: &mut ratatui::Frame) -> Option<(u16, u16, char)> {
        let _ = frame;
        None
    }
}
