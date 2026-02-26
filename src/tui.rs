use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde::Deserialize;

use crate::model::{PageNode, SectionColumn, Site};
use crate::storage::save_site;

pub fn run_tui(site: Site, path: Option<PathBuf>) -> anyhow::Result<()> {
    let theme = match AppTheme::load() {
        Ok(theme) => theme,
        Err(err) => {
            eprintln!("failed to load theme config, using defaults: {err}");
            AppTheme::default()
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(site, path, theme);
    let run_res = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    run_res
}

struct App {
    site: Site,
    theme: AppTheme,
    selected_page: usize,
    selected_node: usize,
    selected_tree_row: usize,
    selected_column: usize,
    selected_component: usize,
    selected_nested_item: usize,
    list_area: Rect,
    details_area: Rect,
    details_scroll_row: usize,
    status: String,
    path: Option<PathBuf>,
    should_quit: bool,
    save_prompt_open: bool,
    save_input: String,
    input_mode: Option<InputMode>,
    input_buffer: String,
    input_cursor: usize,
    multiline_value_area: Option<Rect>,
    multiline_scroll_row: usize,
    component_picker: Option<ComponentPickerState>,
    component_kind: ComponentKind,
    show_help: bool,
    expanded_sections: HashSet<(usize, usize)>,
    expanded_accordion_items: HashSet<(usize, usize, usize, usize)>,
    expanded_alternating_items: HashSet<(usize, usize, usize, usize)>,
    expanded_card_items: HashSet<(usize, usize, usize, usize)>,
    expanded_filmstrip_items: HashSet<(usize, usize, usize, usize)>,
    expanded_milestones_items: HashSet<(usize, usize, usize, usize)>,
}

struct ComponentPickerState {
    query: String,
    selected: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    EditHeroImage,
    EditHeroClass,
    EditHeroAos,
    EditHeroCustomCss,
    EditHeroTitle,
    EditHeroSubtitle,
    EditHeroCopy,
    EditHeroCtaText,
    EditHeroCtaLink,
    EditHeroCtaTarget,
    EditHeroCtaText2,
    EditHeroCtaLink2,
    EditHeroCtaTarget2,
    EditSectionId,
    EditSectionTitle,
    EditSectionClass,
    EditColumnId,
    EditColumnWidthClass,
    EditBannerClass,
    EditBannerDataAos,
    EditBannerImageUrl,
    EditBannerImageAlt,
    EditCtaClass,
    EditCtaImageUrl,
    EditCtaImageAlt,
    EditCtaDataAos,
    EditCtaTitle,
    EditCtaSubtitle,
    EditCtaCopy,
    EditCtaLinkUrl,
    EditCtaLinkTarget,
    EditCtaLinkLabel,
    EditBlockquoteDataAos,
    EditBlockquoteImageUrl,
    EditBlockquoteImageAlt,
    EditBlockquotePersonsName,
    EditBlockquotePersonsTitle,
    EditBlockquoteCopy,
    EditAccordionType,
    EditAccordionClass,
    EditAccordionAos,
    EditAccordionGroupName,
    EditAccordionFirstTitle,
    EditAccordionFirstContent,
    EditAlternatingType,
    EditAlternatingClass,
    EditAlternatingDataAos,
    EditAlternatingItemImage,
    EditAlternatingItemImageAlt,
    EditAlternatingItemTitle,
    EditAlternatingItemCopy,
    EditCardType,
    EditCardDataAos,
    EditCardWidth,
    EditCardItemImageUrl,
    EditCardItemImageAlt,
    EditCardItemTitle,
    EditCardItemSubtitle,
    EditCardItemCopy,
    EditCardItemLinkUrl,
    EditCardItemLinkTarget,
    EditCardItemLinkLabel,
    EditFilmstripType,
    EditFilmstripDataAos,
    EditFilmstripItemImageUrl,
    EditFilmstripItemImageAlt,
    EditFilmstripItemTitle,
    EditMilestonesDataAos,
    EditMilestonesWidth,
    EditMilestonesItemPercentage,
    EditMilestonesItemTitle,
    EditMilestonesItemSubtitle,
    EditMilestonesItemCopy,
    EditMilestonesItemLinkUrl,
    EditMilestonesItemLinkTarget,
    EditMilestonesItemLinkLabel,
}

#[derive(Clone, Copy)]
enum ComponentKind {
    Hero,
    Section,
    Banner,
    Cta,
    Blockquote,
    Accordion,
    Alternating,
    Card,
    Filmstrip,
    Milestones,
}

#[derive(Clone, Copy)]
struct AppTheme {
    background: Color,
    panel_background: Color,
    popup_background: Color,
    foreground: Color,
    muted: Color,
    border: Color,
    title: Color,
    selected_background: Color,
    selected_foreground: Color,
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    colors: PaletteFile,
}

#[derive(Debug, Deserialize)]
struct PaletteFile {
    base: String,
    mantle: Option<String>,
    crust: Option<String>,
    text: String,
    subtext0: Option<String>,
    surface0: String,
    overlay0: String,
    lavender: Option<String>,
    blue: Option<String>,
}

#[derive(Clone, Copy)]
struct NodeTreeRow {
    kind: NodeTreeKind,
}

#[derive(Clone, Copy)]
enum NodeTreeKind {
    Hero {
        node_idx: usize,
    },
    Section {
        node_idx: usize,
    },
    Column {
        node_idx: usize,
        column_idx: usize,
    },
    Component {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    AccordionItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    AlternatingItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    CardItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    FilmstripItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    MilestonesItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
}

impl App {
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme) -> Self {
        for page in &mut site.pages {
            ensure_page_section_ids(page);
        }
        Self {
            site,
            theme,
            selected_page: 0,
            selected_node: 0,
            selected_tree_row: 0,
            selected_column: 0,
            selected_component: 0,
            selected_nested_item: 0,
            list_area: Rect::default(),
            details_area: Rect::default(),
            details_scroll_row: 0,
            status: "Ready.".to_string(),
            path,
            should_quit: false,
            save_prompt_open: false,
            save_input: String::new(),
            input_mode: None,
            input_buffer: String::new(),
            input_cursor: 0,
            multiline_value_area: None,
            multiline_scroll_row: 0,
            component_picker: None,
            component_kind: ComponentKind::Banner,
            show_help: false,
            expanded_sections: HashSet::new(),
            expanded_accordion_items: HashSet::new(),
            expanded_alternating_items: HashSet::new(),
            expanded_card_items: HashSet::new(),
            expanded_filmstrip_items: HashSet::new(),
            expanded_milestones_items: HashSet::new(),
        }
    }

    fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> anyhow::Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(100))? {
                let evt = event::read()?;
                self.handle_event(evt)?;
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.multiline_value_area = None;
        let page = self.current_page();
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(root[1]);

        let header = Paragraph::new(format!(
            "Page: {} ({}/{})",
            page.title,
            self.selected_page + 1,
            self.site.pages.len()
        ))
        .style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.background),
        );
        frame.render_widget(header, root[0]);

        let tree_rows = self.build_node_tree_rows();
        let node_lines = tree_rows
            .iter()
            .enumerate()
            .map(|(_, row)| ListItem::new(self.tree_row_label(row)))
            .collect::<Vec<_>>();
        let list = List::new(node_lines)
            .block(
                Block::default()
                    .title("Nodes")
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
        let mut state = ListState::default();
        if !tree_rows.is_empty() {
            state.select(Some(self.selected_tree_row.min(tree_rows.len() - 1)));
        }
        frame.render_stateful_widget(list, main[0], &mut state);
        self.list_area = main[0];

        self.details_area = main[1];
        let details_width = main[1].width.saturating_sub(2) as usize;
        let details_content = self.details_text(details_width);
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
                    .title("Details")
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

        let footer_text = format!(
            "F1 help | q quit | s save | / insert | Enter edit | Space expand/collapse | A add collection item | X remove collection item | {}",
            self.status
        );
        let footer = Paragraph::new(footer_text)
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.background),
            )
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            );
        frame.render_widget(footer, root[2]);

        if self.show_help {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let help = Paragraph::new(help_text())
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .block(
                    Block::default()
                        .title("Keybindings (F1 to close)")
                        .borders(Borders::ALL)
                        .style(
                            Style::default()
                                .fg(self.theme.foreground)
                                .bg(self.theme.popup_background),
                        )
                        .border_style(Style::default().fg(self.theme.border))
                        .title_style(
                            Style::default()
                                .fg(self.theme.title)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(help, area);
        }

        if self.input_mode.is_some() {
            let area = centered_rect(72, 60, frame.area());
            frame.render_widget(Clear, area);
            let edit_help = self.current_modal_fields();
            let value_block = if self.is_multiline_input_mode() {
                self.ensure_multiline_cursor_visible();
                let inner_width = area.width.saturating_sub(2) as usize;
                let box_inner = inner_width.saturating_sub(2).max(10);
                let visible_rows = self.multiline_rows();
                let all_lines = input_lines_preserve(&self.input_buffer);
                let total_rows = all_lines.len().max(1);
                let has_scroll = total_rows > visible_rows;
                let content_width = if has_scroll {
                    box_inner.saturating_sub(1).max(1)
                } else {
                    box_inner
                };
                let start = self.multiline_scroll_row.min(total_rows.saturating_sub(1));
                let end = (start + visible_rows).min(total_rows);
                let mut lines = all_lines[start..end].to_vec();
                while lines.len() < visible_rows {
                    lines.push(String::new());
                }
                self.multiline_value_area = Some(Rect {
                    x: area.x.saturating_add(2),
                    y: area.y.saturating_add(5),
                    width: content_width.min(u16::MAX as usize) as u16,
                    height: visible_rows.min(u16::MAX as usize) as u16,
                });
                let thumb_row = if has_scroll {
                    Some((self.multiline_scroll_row * visible_rows) / total_rows)
                } else {
                    None
                };
                let mut row_lines = Vec::with_capacity(visible_rows);
                for (idx, line) in lines.iter().enumerate() {
                    let mut rendered = fit_ascii_cell(line, content_width);
                    if has_scroll {
                        let ch = if Some(idx) == thumb_row { '#' } else { '|' };
                        rendered.push(ch);
                    }
                    row_lines.push(format!("|{}|", rendered));
                }
                let field_label = match self.input_mode {
                    Some(InputMode::EditHeroCopy) => {
                        "Value (textarea, 3 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditAlternatingItemCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditBlockquoteCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditCardItemCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditAccordionFirstContent) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    _ => "Value:",
                };
                format!(
                    "{}\n+{}+\n{}\n+{}+",
                    field_label,
                    "-".repeat(box_inner),
                    row_lines.join("\n"),
                    "-".repeat(box_inner)
                )
            } else {
                format!("Value:\n{}", self.input_buffer)
            };
            let modal = Paragraph::new(format!(
                "Editing: {}\n\n{}\n\nEditable fields:\n{}\n\nEnter: save | Esc: cancel",
                self.current_input_mode_label(),
                value_block,
                edit_help
            ))
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .title("Edit Item")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.popup_background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .wrap(Wrap { trim: true });
            frame.render_widget(modal, area);
        }

        if let Some(picker) = &self.component_picker {
            let area = centered_rect(70, 70, frame.area());
            frame.render_widget(Clear, area);
            let filtered = self.filtered_component_kinds(&picker.query);
            let mut lines = Vec::new();
            lines.push(format!("Search: {}", picker.query));
            lines.push(String::new());
            if filtered.is_empty() {
                lines.push("No component matches query.".to_string());
            } else {
                for (idx, kind) in filtered.iter().enumerate() {
                    let marker = if idx == picker.selected { ">" } else { " " };
                    lines.push(format!("{marker} {}", kind.label()));
                }
            }
            lines.push(String::new());
            lines.push("Type to fuzzy search (e.g. hero, dd-cta, dd-milestones).".to_string());
            lines.push("Up/Down to choose, Enter to add, Esc to cancel.".to_string());
            let picker_widget = Paragraph::new(lines.join("\n"))
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .block(
                    Block::default()
                        .title("Add Component")
                        .borders(Borders::ALL)
                        .style(
                            Style::default()
                                .fg(self.theme.foreground)
                                .bg(self.theme.popup_background),
                        )
                        .border_style(Style::default().fg(self.theme.border))
                        .title_style(
                            Style::default()
                                .fg(self.theme.title)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(picker_widget, area);
        }

        if self.save_prompt_open {
            let area = centered_rect(70, 35, frame.area());
            frame.render_widget(Clear, area);
            let prompt = Paragraph::new(format!(
                "Save file path:\n{}\n\nEnter: save | Esc: cancel",
                self.save_input
            ))
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .title("Save Page")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.popup_background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .wrap(Wrap { trim: true });
            frame.render_widget(prompt, area);
        }

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

    fn set_cursor_for_active_input(&self, frame: &mut ratatui::Frame) -> Option<(u16, u16, char)> {
        if self.input_mode.is_some() {
            let area = centered_rect(72, 60, frame.area());
            let inner_width = area.width.saturating_sub(2) as usize;
            let (x, y) = if self.is_multiline_input_mode() {
                let (row_idx, col_count) = cursor_row_col(&self.input_buffer, self.input_cursor);
                let value_area = self.multiline_value_area.unwrap_or(Rect {
                    x: area.x.saturating_add(2),
                    y: area.y.saturating_add(5),
                    width: inner_width.saturating_sub(2).min(u16::MAX as usize) as u16,
                    height: self.multiline_rows().min(u16::MAX as usize) as u16,
                });
                let max_col = value_area.width.saturating_sub(1) as usize;
                let visible_row = row_idx.saturating_sub(self.multiline_scroll_row);
                let max_row = self.multiline_rows().saturating_sub(1);
                (
                    value_area.x.saturating_add(col_count.min(max_col) as u16),
                    value_area.y.saturating_add(visible_row.min(max_row) as u16),
                )
            } else {
                (
                    area.x.saturating_add(1).saturating_add(
                        self.input_cursor
                            .min(inner_width.saturating_sub(1)) as u16,
                    ),
                    area.y.saturating_add(4),
                )
            };
            frame.set_cursor_position((x, y));
            return Some((x, y, self.current_input_cursor_glyph()));
        }

        if self.save_prompt_open {
            let area = centered_rect(70, 35, frame.area());
            let inner_width = area.width.saturating_sub(2) as usize;
            let x = area.x.saturating_add(1).saturating_add(
                self.save_input
                    .chars()
                    .count()
                    .min(inner_width.saturating_sub(1)) as u16,
            );
            let y = area.y.saturating_add(2);
            frame.set_cursor_position((x, y));
            let ch = self
                .save_input
                .chars()
                .nth(self.save_input.chars().count())
                .unwrap_or(' ');
            return Some((x, y, ch));
        }

        if let Some(picker) = &self.component_picker {
            let area = centered_rect(70, 70, frame.area());
            let prefix = "Search: ";
            let inner_width = area.width.saturating_sub(2) as usize;
            let max_query_width = inner_width.saturating_sub(prefix.chars().count() + 1);
            let x = area
                .x
                .saturating_add(1)
                .saturating_add(prefix.chars().count() as u16)
                .saturating_add(picker.query.chars().count().min(max_query_width) as u16);
            let y = area.y.saturating_add(1);
            frame.set_cursor_position((x, y));
            return Some((x, y, ' '));
        }
        None
    }

    fn current_input_cursor_glyph(&self) -> char {
        if self.input_cursor >= self.input_buffer.chars().count() {
            return ' ';
        }
        let ch = self
            .input_buffer
            .chars()
            .nth(self.input_cursor)
            .unwrap_or(' ');
        if ch == '\n' { ' ' } else { ch }
    }

    fn handle_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if self.show_help {
            if let Event::Key(k) = evt {
                match k.code {
                    KeyCode::F(1) | KeyCode::Esc => self.show_help = false,
                    _ => {}
                }
            }
            return Ok(());
        }

        if self.save_prompt_open {
            return self.handle_save_prompt_event(evt);
        }

        if self.component_picker.is_some() {
            return self.handle_component_picker_event(evt);
        }

        if self.input_mode.is_some() {
            return self.handle_input_mode(evt);
        }
        match evt {
            Event::Key(k) => match k.code {
                KeyCode::F(1) => self.show_help = true,
                KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true
                }
                KeyCode::Up => self.select_prev(),
                KeyCode::Down => self.select_next(),
                KeyCode::PageUp => self.scroll_details_by(-5),
                KeyCode::PageDown => self.scroll_details_by(5),
                KeyCode::Char(' ') => self.toggle_selected_tree_expanded(),
                KeyCode::Enter => self.handle_enter_on_selected_row(),
                KeyCode::Tab => self.select_next_page(),
                KeyCode::BackTab => self.select_prev_page(),
                KeyCode::Char('s') => self.begin_save_prompt(),
                KeyCode::Char('/') => self.open_component_picker(),
                KeyCode::Char('d') => self.delete_selected_node(),
                KeyCode::Char('J') => self.move_selected_down(),
                KeyCode::Char('K') => self.move_selected_up(),
                KeyCode::Char('C') => self.add_column(),
                KeyCode::Char('V') => self.remove_selected_column(),
                KeyCode::Char('c') => self.select_prev_column(),
                KeyCode::Char('v') => self.select_next_column(),
                KeyCode::Char('(') => self.move_selected_column_up(),
                KeyCode::Char(')') => self.move_selected_column_down(),
                KeyCode::Char('r') => self.begin_edit_selected_column_id(),
                KeyCode::Char('f') => self.begin_edit_selected_column_width_class(),
                KeyCode::Char('A') => self.add_selected_collection_item(),
                KeyCode::Char('X') => self.remove_selected_collection_item(),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(-3);
                    } else {
                        self.select_prev();
                    }
                }
                MouseEventKind::ScrollDown => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(3);
                    } else {
                        self.select_next();
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    self.handle_click(m.column, m.row);
                }
                _ => {}
            },
            _ => {}
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    fn handle_input_mode(&mut self, evt: Event) -> anyhow::Result<()> {
        match evt {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.input_mode = None;
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                    self.multiline_scroll_row = 0;
                    self.status = "Edit cancelled.".to_string();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Enter => {
                    if self.is_multiline_input_mode()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        let rows = input_lines_preserve(&self.input_buffer).len().max(1);
                        let max_rows = self.multiline_max_rows();
                        if max_rows.map(|limit| rows < limit).unwrap_or(true) {
                            self.insert_char_at_cursor('\n');
                        } else {
                            self.status = match self.input_mode {
                                Some(InputMode::EditHeroCopy) => {
                                    "hero.copy supports up to 3 lines. Press Ctrl+S to save."
                                        .to_string()
                                }
                                _ => "Line limit reached. Press Ctrl+S to save.".to_string(),
                            };
                        }
                        return Ok(());
                    }
                    let _ = self.commit_input_edit();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Char('s')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.is_multiline_input_mode() =>
                {
                    let _ = self.commit_input_edit();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Tab => {
                    self.tab_next_component_field();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::BackTab => {
                    self.tab_prev_component_field();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Up => {
                    if self.is_multiline_input_mode() {
                        self.move_cursor_up_line();
                    }
                }
                KeyCode::Down => {
                    if self.is_multiline_input_mode() {
                        self.move_cursor_down_line();
                    }
                }
                KeyCode::Left => match self.input_mode {
                    Some(InputMode::EditHeroClass) => {
                        self.cycle_hero_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroAos) => {
                        self.cycle_hero_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget) => {
                        self.cycle_hero_cta_target(false, false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroCtaTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget2) => {
                        self.cycle_hero_cta_target(true, false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditHeroCtaTarget2)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerClass) => {
                        self.cycle_banner_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerDataAos) => {
                        self.cycle_banner_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaClass) => {
                        self.cycle_cta_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaDataAos) => {
                        self.cycle_cta_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaLinkTarget) => {
                        self.cycle_cta_link_target(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBlockquoteDataAos) => {
                        self.cycle_blockquote_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditBlockquoteDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardType) => {
                        self.cycle_card_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardType) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardDataAos) => {
                        self.cycle_card_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardItemLinkTarget) => {
                        self.cycle_card_link_target(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditCardItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripType) => {
                        self.cycle_filmstrip_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditFilmstripType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripDataAos) => {
                        self.cycle_filmstrip_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditFilmstripDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesDataAos) => {
                        self.cycle_milestones_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesItemLinkTarget) => {
                        self.cycle_milestones_link_target(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingType) => {
                        self.cycle_alternating_type(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingDataAos) => {
                        self.cycle_alternating_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionType) => {
                        self.cycle_accordion_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionClass) => {
                        self.cycle_accordion_class(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAccordionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionAos) => {
                        self.cycle_accordion_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    _ => self.move_cursor_left(),
                },
                KeyCode::Right => match self.input_mode {
                    Some(InputMode::EditHeroClass) => {
                        self.cycle_hero_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroAos) => {
                        self.cycle_hero_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget) => {
                        self.cycle_hero_cta_target(false, true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroCtaTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget2) => {
                        self.cycle_hero_cta_target(true, true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditHeroCtaTarget2)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerClass) => {
                        self.cycle_banner_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerDataAos) => {
                        self.cycle_banner_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaClass) => {
                        self.cycle_cta_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaDataAos) => {
                        self.cycle_cta_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaLinkTarget) => {
                        self.cycle_cta_link_target(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBlockquoteDataAos) => {
                        self.cycle_blockquote_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditBlockquoteDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardType) => {
                        self.cycle_card_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardType) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardDataAos) => {
                        self.cycle_card_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardItemLinkTarget) => {
                        self.cycle_card_link_target(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditCardItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripType) => {
                        self.cycle_filmstrip_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditFilmstripType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripDataAos) => {
                        self.cycle_filmstrip_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditFilmstripDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesDataAos) => {
                        self.cycle_milestones_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesItemLinkTarget) => {
                        self.cycle_milestones_link_target(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingType) => {
                        self.cycle_alternating_type(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingDataAos) => {
                        self.cycle_alternating_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionType) => {
                        self.cycle_accordion_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionClass) => {
                        self.cycle_accordion_class(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAccordionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionAos) => {
                        self.cycle_accordion_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    _ => self.move_cursor_right(),
                },
                KeyCode::Backspace => {
                    self.delete_char_before_cursor();
                }
                KeyCode::Char(c) => {
                    self.insert_char_at_cursor(c);
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) if self.is_multiline_input_mode() => {
                    self.set_multiline_cursor_from_point(m.column, m.row);
                }
                MouseEventKind::ScrollUp if self.is_multiline_input_mode() => {
                    self.move_cursor_up_line();
                }
                MouseEventKind::ScrollDown if self.is_multiline_input_mode() => {
                    self.move_cursor_down_line();
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn is_multiline_mode(mode: InputMode) -> bool {
        matches!(
            mode,
            InputMode::EditHeroCopy
                | InputMode::EditAlternatingItemCopy
                | InputMode::EditCtaCopy
                | InputMode::EditBlockquoteCopy
                | InputMode::EditCardItemCopy
                | InputMode::EditMilestonesItemCopy
                | InputMode::EditAccordionFirstContent
        )
    }

    fn is_multiline_input_mode(&self) -> bool {
        self.input_mode.is_some_and(Self::is_multiline_mode)
    }

    fn multiline_rows_for_mode(mode: InputMode) -> usize {
        match mode {
            InputMode::EditHeroCopy => 3,
            InputMode::EditAlternatingItemCopy
            | InputMode::EditCtaCopy
            | InputMode::EditBlockquoteCopy
            | InputMode::EditCardItemCopy
            | InputMode::EditMilestonesItemCopy
            | InputMode::EditAccordionFirstContent => 5,
            _ => 1,
        }
    }

    fn multiline_rows(&self) -> usize {
        self.input_mode
            .map(Self::multiline_rows_for_mode)
            .unwrap_or(1)
    }

    fn multiline_max_rows_for_mode(mode: InputMode) -> Option<usize> {
        match mode {
            InputMode::EditHeroCopy => Some(3),
            InputMode::EditAlternatingItemCopy
            | InputMode::EditCtaCopy
            | InputMode::EditBlockquoteCopy
            | InputMode::EditCardItemCopy
            | InputMode::EditMilestonesItemCopy
            | InputMode::EditAccordionFirstContent => None,
            _ => None,
        }
    }

    fn multiline_max_rows(&self) -> Option<usize> {
        self.input_mode.and_then(Self::multiline_max_rows_for_mode)
    }

    fn ensure_multiline_cursor_visible(&mut self) {
        if !self.is_multiline_input_mode() {
            self.multiline_scroll_row = 0;
            return;
        }
        let visible_rows = self.multiline_rows();
        let (row_idx, _) = cursor_row_col(&self.input_buffer, self.input_cursor);
        if row_idx < self.multiline_scroll_row {
            self.multiline_scroll_row = row_idx;
        } else if row_idx >= self.multiline_scroll_row + visible_rows {
            self.multiline_scroll_row = row_idx.saturating_sub(visible_rows.saturating_sub(1));
        }
        let total_rows = input_lines_preserve(&self.input_buffer).len().max(1);
        let max_scroll = total_rows.saturating_sub(visible_rows);
        self.multiline_scroll_row = self.multiline_scroll_row.min(max_scroll);
    }

    fn clamp_multiline_input_if_needed(&mut self) {
        let Some(max_rows) = self.multiline_max_rows() else {
            return;
        };
        let mut lines = self
            .input_buffer
            .split('\n')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if lines.len() > max_rows {
            lines.truncate(max_rows);
            self.input_buffer = lines.join("\n");
            self.input_cursor = self.input_cursor.min(self.input_buffer.chars().count());
        }
    }

    fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.ensure_multiline_cursor_visible();
        }
    }

    fn move_cursor_right(&mut self) {
        let len = self.input_buffer.chars().count();
        if self.input_cursor < len {
            self.input_cursor += 1;
            self.ensure_multiline_cursor_visible();
        }
    }

    fn move_cursor_up_line(&mut self) {
        let (row, col) = cursor_row_col(&self.input_buffer, self.input_cursor);
        if row == 0 {
            return;
        }
        let lines = input_lines_preserve(&self.input_buffer);
        self.input_cursor = cursor_from_row_col(&lines, row - 1, col);
        self.ensure_multiline_cursor_visible();
    }

    fn move_cursor_down_line(&mut self) {
        let (row, col) = cursor_row_col(&self.input_buffer, self.input_cursor);
        let lines = input_lines_preserve(&self.input_buffer);
        if row + 1 >= lines.len() {
            return;
        }
        self.input_cursor = cursor_from_row_col(&lines, row + 1, col);
        self.ensure_multiline_cursor_visible();
    }

    fn delete_char_before_cursor(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let remove_at = self.input_cursor - 1;
        let byte_start = byte_index_for_char(&self.input_buffer, remove_at);
        let byte_end = byte_index_for_char(&self.input_buffer, self.input_cursor);
        self.input_buffer.replace_range(byte_start..byte_end, "");
        self.input_cursor = remove_at;
        self.ensure_multiline_cursor_visible();
    }

    fn insert_char_at_cursor(&mut self, c: char) {
        if self.is_multiline_input_mode() && c == '\n' {
            let mut candidate = self.input_buffer.clone();
            let at = byte_index_for_char(&candidate, self.input_cursor);
            candidate.insert(at, '\n');
            if let Some(max_rows) = self.multiline_max_rows() {
                if input_lines_preserve(&candidate).len().max(1) > max_rows {
                    self.status = match self.input_mode {
                        Some(InputMode::EditHeroCopy) => {
                            "hero.copy supports up to 3 lines.".to_string()
                        }
                        _ => "Line limit reached.".to_string(),
                    };
                    return;
                }
            }
            self.input_buffer = candidate;
            self.input_cursor += 1;
            self.ensure_multiline_cursor_visible();
            return;
        }
        let at = byte_index_for_char(&self.input_buffer, self.input_cursor);
        self.input_buffer.insert(at, c);
        self.input_cursor += 1;
        if self.is_multiline_input_mode() {
            self.ensure_multiline_cursor_visible();
        }
    }

    fn set_multiline_cursor_from_point(&mut self, x: u16, y: u16) {
        let Some(area) = self.multiline_value_area else {
            return;
        };
        if !contains(area, x, y) {
            return;
        }
        let row = y.saturating_sub(area.y) as usize + self.multiline_scroll_row;
        let col = x.saturating_sub(area.x) as usize;
        let lines = input_lines_preserve(&self.input_buffer);
        let mut cursor = 0usize;
        for i in 0..row {
            if let Some(line) = lines.get(i) {
                cursor += line.chars().count() + 1;
            } else {
                return;
            }
        }
        if let Some(line) = lines.get(row) {
            cursor += col.min(line.chars().count());
        } else if row == lines.len() {
            cursor += 0;
        } else {
            return;
        }
        self.input_cursor = cursor.min(self.input_buffer.chars().count());
        self.ensure_multiline_cursor_visible();
    }

    fn tab_next_component_field(&mut self) {
        self.tab_component_field(true);
    }

    fn tab_prev_component_field(&mut self) {
        self.tab_component_field(false);
    }

    fn tab_component_field(&mut self, forward: bool) {
        let Some(current) = self.input_mode else {
            return;
        };
        let Some(group) = self.component_edit_group_for_active_mode(current) else {
            self.status = "Tab field navigation is available while editing hero/component fields."
                .to_string();
            return;
        };
        let Some(idx) = group.iter().position(|m| *m == current) else {
            return;
        };
        let ok = self.commit_input_edit();
        if !ok {
            return;
        }
        let next_idx = if forward {
            (idx + 1) % group.len()
        } else if idx == 0 {
            group.len() - 1
        } else {
            idx - 1
        };
        let next_mode = group[next_idx];
        if !self.set_component_input_mode(next_mode) {
            self.status = "Could not switch to next component field.".to_string();
        }
    }

    fn component_edit_group_for_active_mode(&self, mode: InputMode) -> Option<Vec<InputMode>> {
        let accordion_mode = matches!(
            mode,
            InputMode::EditAccordionType
                | InputMode::EditAccordionClass
                | InputMode::EditAccordionAos
                | InputMode::EditAccordionGroupName
                | InputMode::EditAccordionFirstTitle
                | InputMode::EditAccordionFirstContent
        );
        if accordion_mode {
            let rows = self.build_node_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(NodeTreeKind::AccordionItem { .. })) {
                return Some(vec![
                    InputMode::EditAccordionFirstTitle,
                    InputMode::EditAccordionFirstContent,
                ]);
            }
            return Some(vec![
                InputMode::EditAccordionType,
                InputMode::EditAccordionClass,
                InputMode::EditAccordionAos,
                InputMode::EditAccordionGroupName,
            ]);
        }
        let alternating_mode = matches!(
            mode,
            InputMode::EditAlternatingType
                | InputMode::EditAlternatingClass
                | InputMode::EditAlternatingDataAos
                | InputMode::EditAlternatingItemImage
                | InputMode::EditAlternatingItemImageAlt
                | InputMode::EditAlternatingItemTitle
                | InputMode::EditAlternatingItemCopy
        );
        if alternating_mode {
            let rows = self.build_node_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(NodeTreeKind::AlternatingItem { .. })) {
                return Some(vec![
                    InputMode::EditAlternatingItemImage,
                    InputMode::EditAlternatingItemImageAlt,
                    InputMode::EditAlternatingItemTitle,
                    InputMode::EditAlternatingItemCopy,
                ]);
            }
            return Some(vec![
                InputMode::EditAlternatingType,
                InputMode::EditAlternatingClass,
                InputMode::EditAlternatingDataAos,
            ]);
        }
        let card_mode = matches!(
            mode,
            InputMode::EditCardType
                | InputMode::EditCardDataAos
                | InputMode::EditCardWidth
                | InputMode::EditCardItemImageUrl
                | InputMode::EditCardItemImageAlt
                | InputMode::EditCardItemTitle
                | InputMode::EditCardItemSubtitle
                | InputMode::EditCardItemCopy
                | InputMode::EditCardItemLinkUrl
                | InputMode::EditCardItemLinkTarget
                | InputMode::EditCardItemLinkLabel
        );
        if card_mode {
            let rows = self.build_node_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(NodeTreeKind::CardItem { .. })) {
                return Some(vec![
                    InputMode::EditCardItemImageUrl,
                    InputMode::EditCardItemImageAlt,
                    InputMode::EditCardItemTitle,
                    InputMode::EditCardItemSubtitle,
                    InputMode::EditCardItemCopy,
                    InputMode::EditCardItemLinkUrl,
                    InputMode::EditCardItemLinkTarget,
                    InputMode::EditCardItemLinkLabel,
                ]);
            }
            return Some(vec![
                InputMode::EditCardType,
                InputMode::EditCardDataAos,
                InputMode::EditCardWidth,
            ]);
        }
        let filmstrip_mode = matches!(
            mode,
            InputMode::EditFilmstripType
                | InputMode::EditFilmstripDataAos
                | InputMode::EditFilmstripItemImageUrl
                | InputMode::EditFilmstripItemImageAlt
                | InputMode::EditFilmstripItemTitle
        );
        if filmstrip_mode {
            let rows = self.build_node_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(NodeTreeKind::FilmstripItem { .. })) {
                return Some(vec![
                    InputMode::EditFilmstripItemImageUrl,
                    InputMode::EditFilmstripItemImageAlt,
                    InputMode::EditFilmstripItemTitle,
                ]);
            }
            return Some(vec![
                InputMode::EditFilmstripType,
                InputMode::EditFilmstripDataAos,
            ]);
        }
        let milestones_mode = matches!(
            mode,
            InputMode::EditMilestonesDataAos
                | InputMode::EditMilestonesWidth
                | InputMode::EditMilestonesItemPercentage
                | InputMode::EditMilestonesItemTitle
                | InputMode::EditMilestonesItemSubtitle
                | InputMode::EditMilestonesItemCopy
                | InputMode::EditMilestonesItemLinkUrl
                | InputMode::EditMilestonesItemLinkTarget
                | InputMode::EditMilestonesItemLinkLabel
        );
        if milestones_mode {
            let rows = self.build_node_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(NodeTreeKind::MilestonesItem { .. })) {
                return Some(vec![
                    InputMode::EditMilestonesItemPercentage,
                    InputMode::EditMilestonesItemTitle,
                    InputMode::EditMilestonesItemSubtitle,
                    InputMode::EditMilestonesItemCopy,
                    InputMode::EditMilestonesItemLinkUrl,
                    InputMode::EditMilestonesItemLinkTarget,
                    InputMode::EditMilestonesItemLinkLabel,
                ]);
            }
            return Some(vec![
                InputMode::EditMilestonesDataAos,
                InputMode::EditMilestonesWidth,
            ]);
        }
        component_edit_group_for_mode(mode).map(|modes| modes.to_vec())
    }

    fn handle_component_picker_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.component_picker = None;
                    self.status = "Component picker cancelled.".to_string();
                }
                KeyCode::Up => {
                    let selected = self
                        .component_picker
                        .as_ref()
                        .map(|p| p.selected)
                        .unwrap_or(0)
                        .saturating_sub(1);
                    if let Some(picker) = &mut self.component_picker {
                        picker.selected = selected;
                    }
                }
                KeyCode::Down => {
                    let (query, selected) = if let Some(picker) = &self.component_picker {
                        (picker.query.clone(), picker.selected)
                    } else {
                        (String::new(), 0)
                    };
                    let total = self.filtered_component_kinds(&query).len();
                    if let Some(picker) = &mut self.component_picker {
                        if total == 0 {
                            picker.selected = 0;
                        } else {
                            picker.selected = (selected + 1).min(total - 1);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(picker) = &mut self.component_picker {
                        picker.query.pop();
                    }
                    self.normalize_component_picker_selection();
                }
                KeyCode::Enter => {
                    let (query, selected) = if let Some(picker) = &self.component_picker {
                        (picker.query.clone(), picker.selected)
                    } else {
                        (String::new(), 0)
                    };
                    let filtered = self.filtered_component_kinds(&query);
                    let Some(kind) = filtered
                        .get(selected.min(filtered.len().saturating_sub(1)))
                        .copied()
                    else {
                        self.status = "No component selected.".to_string();
                        return Ok(());
                    };
                    self.component_kind = kind;
                    self.insert_selected_component_kind();
                    self.component_picker = None;
                }
                KeyCode::Char(c) => {
                    if let Some(picker) = &mut self.component_picker {
                        picker.query.push(c);
                    }
                    self.normalize_component_picker_selection();
                }
                _ => {}
            }
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    fn begin_save_prompt(&mut self) {
        self.component_picker = None;
        self.input_mode = None;
        self.save_prompt_open = true;
        self.save_input = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "site.json".to_string());
        self.status = "Save prompt opened.".to_string();
    }

    fn handle_save_prompt_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.save_prompt_open = false;
                    self.save_input.clear();
                    self.status = "Save cancelled.".to_string();
                }
                KeyCode::Enter => self.commit_save_prompt()?,
                KeyCode::Backspace => {
                    self.save_input.pop();
                }
                KeyCode::Char(c) => {
                    self.save_input.push(c);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn commit_save_prompt(&mut self) -> anyhow::Result<()> {
        let raw = self.save_input.trim();
        if raw.is_empty() {
            self.status = "Save path cannot be empty.".to_string();
            return Ok(());
        }
        let path = PathBuf::from(raw);
        save_site(&path, &self.site)?;
        self.path = Some(path.clone());
        self.save_prompt_open = false;
        self.save_input.clear();
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    fn begin_edit_selected(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                Some(match &page.nodes[idx] {
                    PageNode::Hero(v) => (InputMode::EditHeroImage, v.image.clone()),
                    PageNode::Section(v) => (InputMode::EditSectionId, v.id.clone()),
                })
            }
        };

        let Some((mode, value)) = selected else {
            self.status = "No node selected.".to_string();
            return;
        };

        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.clamp_multiline_input_if_needed();
        self.input_cursor = self.input_buffer.chars().count();
        self.ensure_multiline_cursor_visible();
        self.status = match mode {
            InputMode::EditHeroImage => {
                "Editing hero image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroClass => {
                "Editing hero default class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroAos => {
                "Editing hero data-aos option. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCustomCss => {
                "Editing hero custom CSS classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget => {
                "Editing hero primary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget2 => {
                "Editing hero secondary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionId => {
                "Editing section id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionTitle => {
                "Editing section title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionClass => {
                "Editing section class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditColumnId => {
                "Editing column id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditColumnWidthClass => {
                "Editing column width class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingType => {
                "Editing dd-alternating type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingClass => {
                "Editing dd-alternating class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingDataAos => {
                "Editing dd-alternating data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImage => {
                "Editing dd-alternating item image. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImageAlt => {
                "Editing dd-alternating item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemTitle => {
                "Editing dd-alternating item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemCopy => {
                "Editing dd-alternating item copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditBlockquoteDataAos => {
                "Editing dd-blockquote data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageUrl => {
                "Editing dd-blockquote image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageAlt => {
                "Editing dd-blockquote image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsName => {
                "Editing dd-blockquote person name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsTitle => {
                "Editing dd-blockquote person title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteCopy => {
                "Editing dd-blockquote copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditBannerClass => {
                "Editing dd-banner class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerDataAos => {
                "Editing dd-banner data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageUrl => {
                "Editing dd-banner image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageAlt => {
                "Editing dd-banner image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaClass => {
                "Editing dd-cta class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageUrl => {
                "Editing dd-cta image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageAlt => {
                "Editing dd-cta image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaDataAos => {
                "Editing dd-cta data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaSubtitle => {
                "Editing dd-cta subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaCopy => {
                "Editing dd-cta copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCtaLinkUrl => {
                "Editing dd-cta link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkTarget => {
                "Editing dd-cta link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkLabel => {
                "Editing dd-cta link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionType => {
                "Editing dd-accordion type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionClass => {
                "Editing dd-accordion class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionAos => {
                "Editing dd-accordion data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionGroupName => {
                "Editing dd-accordion group name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstTitle => {
                "Editing dd-accordion first title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstContent => {
                "Editing dd-accordion item content. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditCardType => {
                "Editing dd-card type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardDataAos => {
                "Editing dd-card data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardWidth => {
                "Editing dd-card width class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageUrl => {
                "Editing dd-card item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageAlt => {
                "Editing dd-card item image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemTitle => {
                "Editing dd-card item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemSubtitle => {
                "Editing dd-card item subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemCopy => {
                "Editing dd-card item copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardItemLinkUrl => {
                "Editing dd-card item link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkTarget => {
                "Editing dd-card item link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkLabel => {
                "Editing dd-card item link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripType => {
                "Editing dd-filmstrip type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripDataAos => {
                "Editing dd-filmstrip data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageUrl => {
                "Editing dd-filmstrip item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageAlt => {
                "Editing dd-filmstrip item image alt text. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditFilmstripItemTitle => {
                "Editing dd-filmstrip item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesDataAos => {
                "Editing dd-milestones parent_data_aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesWidth => {
                "Editing dd-milestones parent_width. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemPercentage => {
                "Editing dd-milestones child_percentage. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemTitle => {
                "Editing dd-milestones child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemSubtitle => {
                "Editing dd-milestones child_subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemCopy => {
                "Editing dd-milestones child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditMilestonesItemLinkUrl => {
                "Editing dd-milestones child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkTarget => {
                "Editing dd-milestones child_link_target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkLabel => {
                "Editing dd-milestones child_link_label. Enter to save, esc to cancel.".to_string()
            }
        };
    }

    fn begin_edit_selected_column_id(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[idx] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        Some((InputMode::EditColumnId, columns[col_i].id.clone()))
                    }
                }
            }
        };
        let Some((mode, value)) = selected else {
            self.status = "Selected node is not a section.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status = "Editing selected column id. Enter to save, esc to cancel.".to_string();
    }

    fn begin_edit_selected_column_width_class(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[idx] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        Some((
                            InputMode::EditColumnWidthClass,
                            columns[col_i].width_class.clone(),
                        ))
                    }
                }
            }
        };
        let Some((mode, value)) = selected else {
            self.status = "Selected node is not a section.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status =
            "Editing selected column width class. Enter to save, esc to cancel.".to_string();
    }

    fn commit_input_edit(&mut self) -> bool {
        let Some(mode) = self.input_mode else {
            return false;
        };
        let value = if Self::is_multiline_mode(mode) {
            self.input_buffer.clone()
        } else {
            self.input_buffer.trim().to_string()
        };
        let allow_empty = matches!(
            mode,
            InputMode::EditHeroImage
                | InputMode::EditHeroClass
                | InputMode::EditHeroCustomCss
                | InputMode::EditHeroSubtitle
                | InputMode::EditHeroCopy
                | InputMode::EditHeroCtaText
                | InputMode::EditHeroCtaLink
                | InputMode::EditHeroCtaTarget
                | InputMode::EditHeroCtaText2
                | InputMode::EditHeroCtaLink2
                | InputMode::EditHeroCtaTarget2
                | InputMode::EditSectionTitle
                | InputMode::EditAlternatingItemCopy
                | InputMode::EditCtaLinkUrl
                | InputMode::EditCtaLinkTarget
                | InputMode::EditCtaLinkLabel
                | InputMode::EditBlockquoteCopy
                | InputMode::EditCardItemLinkUrl
                | InputMode::EditCardItemLinkTarget
                | InputMode::EditCardItemLinkLabel
        );
        if value.is_empty() && !allow_empty {
            self.status = "Value cannot be empty.".to_string();
            return false;
        }
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let mut status = "No page available.".to_string();
        let mut clear_input = true;
        let mut applied = false;
        let Some(page) = self.current_page_mut() else {
            self.status = status;
            return false;
        };
        if page.nodes.is_empty() {
            self.status = "No node selected.".to_string();
            return false;
        }
        let idx = selected.min(page.nodes.len() - 1);
        if let PageNode::Section(section) = &mut page.nodes[idx] {
            pull_selected_column_into_legacy_components(section, selected_column);
        }
        status = match (&mut page.nodes[idx], mode) {
            (PageNode::Hero(v), InputMode::EditHeroImage) => {
                v.image = value;
                applied = true;
                "Updated hero image.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroClass) => {
                let parsed = parse_hero_image_class(value.as_str());
                if let Some(hero_class) = parsed {
                    v.hero_class = Some(hero_class);
                    applied = true;
                    "Updated hero default class.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero class option.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroAos) => {
                let parsed = parse_hero_aos(value.as_str());
                if let Some(aos) = parsed {
                    v.hero_aos = Some(aos);
                    applied = true;
                    "Updated hero data-aos option.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero data-aos option.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroCustomCss) => {
                v.custom_css = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero custom CSS classes.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroTitle) => {
                v.title = value;
                applied = true;
                "Updated hero title.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroSubtitle) => {
                v.subtitle = value;
                applied = true;
                "Updated hero subtitle.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCopy) => {
                v.copy = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero copy.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaText) => {
                v.cta_text = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero primary link text.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaLink) => {
                v.cta_link = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero primary link URL.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaTarget) => {
                if value.is_empty() {
                    v.cta_target = None;
                    applied = true;
                    "Updated hero primary link target.".to_string()
                } else if let Some(target) = parse_cta_target(value.as_str()) {
                    v.cta_target = Some(target);
                    applied = true;
                    "Updated hero primary link target.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero primary link target.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaText2) => {
                v.cta_text_2 = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero secondary link text.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaLink2) => {
                v.cta_link_2 = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero secondary link URL.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaTarget2) => {
                if value.is_empty() {
                    v.cta_target_2 = None;
                    applied = true;
                    "Updated hero secondary link target.".to_string()
                } else if let Some(target) = parse_cta_target(value.as_str()) {
                    v.cta_target_2 = Some(target);
                    applied = true;
                    "Updated hero secondary link target.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero secondary link target.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSectionId) => {
                v.id = value;
                applied = true;
                "Updated section id.".to_string()
            }
            (PageNode::Section(v), InputMode::EditSectionTitle) => {
                v.section_title = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated section title.".to_string()
            }
            (PageNode::Section(v), InputMode::EditSectionClass) => {
                if let Some(section_class) = parse_section_class(value.as_str()) {
                    v.section_class = Some(section_class);
                    applied = true;
                    "Updated section class.".to_string()
                } else {
                    clear_input = false;
                    "Invalid section class option.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditColumnId) => {
                let col_i = selected_column.min(v.columns.len().saturating_sub(1));
                v.columns[col_i].id = value;
                applied = true;
                "Updated column id.".to_string()
            }
            (PageNode::Section(v), InputMode::EditColumnWidthClass) => {
                let col_i = selected_column.min(v.columns.len().saturating_sub(1));
                v.columns[col_i].width_class = value;
                applied = true;
                "Updated column width class.".to_string()
            }
            (PageNode::Section(v), InputMode::EditAlternatingType) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(vt) = parse_alternating_type(value.as_str()) {
                            alt.alternating_type = vt;
                            applied = true;
                            "Updated dd-alternating type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-alternating type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingClass) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        alt.alternating_class = value;
                        applied = true;
                        "Updated dd-alternating class.".to_string()
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            alt.alternating_data_aos = va;
                            applied = true;
                            "Updated dd-alternating data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-alternating data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemImage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].image = value;
                            applied = true;
                            format!("Updated dd-alternating item {} image.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].image_alt = value;
                            applied = true;
                            format!("Updated dd-alternating item {} image alt.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].title = value;
                            applied = true;
                            format!("Updated dd-alternating item {} title.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].copy = value;
                            applied = true;
                            format!("Updated dd-alternating item {} copy.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerClass) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        if let Some(vc) = parse_banner_class(value.as_str()) {
                            banner.banner_class = vc;
                            applied = true;
                            "Updated dd-banner class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-banner class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            banner.banner_data_aos = va;
                            applied = true;
                            "Updated dd-banner data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-banner data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerImageUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.banner_image_url = value;
                        applied = true;
                        "Updated dd-banner image URL.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.banner_image_alt = value;
                        applied = true;
                        "Updated dd-banner image alt text.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaClass) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        if let Some(vc) = parse_cta_class(value.as_str()) {
                            cta.cta_class = vc;
                            applied = true;
                            "Updated dd-cta class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaImageUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_image_url = value;
                        applied = true;
                        "Updated dd-cta image URL.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_image_alt = value;
                        applied = true;
                        "Updated dd-cta image alt text.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            cta.cta_data_aos = va;
                            applied = true;
                            "Updated dd-cta data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_title = value;
                        applied = true;
                        "Updated dd-cta title.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaSubtitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_subtitle = value;
                        applied = true;
                        "Updated dd-cta subtitle.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_copy = value;
                        applied = true;
                        "Updated dd-cta copy.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_link_url = if value.is_empty() { None } else { Some(value) };
                        applied = true;
                        "Updated dd-cta link URL.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkTarget) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        if value.is_empty() {
                            cta.cta_link_target = None;
                            applied = true;
                            "Updated dd-cta link target.".to_string()
                        } else if let Some(vt) = parse_card_link_target(value.as_str()) {
                            cta.cta_link_target = Some(vt);
                            applied = true;
                            "Updated dd-cta link target.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta link target option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkLabel) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_link_label = if value.is_empty() { None } else { Some(value) };
                        applied = true;
                        "Updated dd-cta link label.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripType) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.components[ci]
                    {
                        if let Some(vt) = parse_filmstrip_type(value.as_str()) {
                            filmstrip.filmstrip_type = vt;
                            applied = true;
                            "Updated dd-filmstrip type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-filmstrip type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.components[ci]
                    {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            filmstrip.filmstrip_data_aos = va;
                            applied = true;
                            "Updated dd-filmstrip data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-filmstrip data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemImageUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].image_url = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} image URL.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].image_alt = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} image alt.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].title = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} title.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            milestones.parent_data_aos = va;
                            applied = true;
                            "Updated dd-milestones parent_data_aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-milestones parent_data_aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesWidth) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        milestones.parent_width = value;
                        applied = true;
                        "Updated dd-milestones parent_width.".to_string()
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemPercentage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_percentage = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_percentage.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_title.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemSubtitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_subtitle = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_subtitle.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_copy.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_link_url =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-milestones item {} child_link_url.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkTarget) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            if value.is_empty() {
                                milestones.items[ni].child_link_target = None;
                                applied = true;
                                format!("Updated dd-milestones item {} child_link_target.", ni + 1)
                            } else if let Some(vt) = parse_card_link_target(value.as_str()) {
                                milestones.items[ni].child_link_target = Some(vt);
                                applied = true;
                                format!("Updated dd-milestones item {} child_link_target.", ni + 1)
                            } else {
                                clear_input = false;
                                "Invalid dd-milestones child_link_target option.".to_string()
                            }
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkLabel) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_link_label =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-milestones item {} child_link_label.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            blockquote.blockquote_data_aos = va;
                            applied = true;
                            "Updated dd-blockquote data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-blockquote data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteImageUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        blockquote.blockquote_image_url = value;
                        applied = true;
                        "Updated dd-blockquote image URL.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        blockquote.blockquote_image_alt = value;
                        applied = true;
                        "Updated dd-blockquote image alt text.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquotePersonsName) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        blockquote.blockquote_persons_name = value;
                        applied = true;
                        "Updated dd-blockquote person name.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquotePersonsTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        blockquote.blockquote_persons_title = value;
                        applied = true;
                        "Updated dd-blockquote person title.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.components[ci]
                    {
                        blockquote.blockquote_copy = value;
                        applied = true;
                        "Updated dd-blockquote copy.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardType) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(vt) = parse_card_type(value.as_str()) {
                            card.card_type = vt;
                            applied = true;
                            "Updated dd-card type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-card type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardDataAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            card.card_data_aos = va;
                            applied = true;
                            "Updated dd-card data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-card data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardWidth) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        card.card_width = value;
                        applied = true;
                        "Updated dd-card width classes.".to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemImageUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_image_url = value;
                            applied = true;
                            format!("Updated dd-card item {} image URL.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemImageAlt) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_image_alt = value;
                            applied = true;
                            format!("Updated dd-card item {} image alt.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_title = value;
                            applied = true;
                            format!("Updated dd-card item {} title.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemSubtitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_subtitle = value;
                            applied = true;
                            format!("Updated dd-card item {} subtitle.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_copy = value;
                            applied = true;
                            format!("Updated dd-card item {} copy.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_link_url =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-card item {} link URL.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkTarget) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            if value.is_empty() {
                                card.items[ni].card_link_target = None;
                                applied = true;
                                format!("Updated dd-card item {} link target.", ni + 1)
                            } else if let Some(vt) = parse_card_link_target(value.as_str()) {
                                card.items[ni].card_link_target = Some(vt);
                                applied = true;
                                format!("Updated dd-card item {} link target.", ni + 1)
                            } else {
                                clear_input = false;
                                "Invalid dd-card link target option.".to_string()
                            }
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkLabel) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].card_link_label =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-card item {} link label.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionType) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(vt) = parse_accordion_type(value.as_str()) {
                            acc.accordion_type = vt;
                            applied = true;
                            "Updated dd-accordion type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionClass) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(vc) = parse_accordion_class(value.as_str()) {
                            acc.accordion_class = vc;
                            applied = true;
                            "Updated dd-accordion class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionAos) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(va) = parse_hero_aos(value.as_str()) {
                            acc.accordion_aos = va;
                            applied = true;
                            "Updated dd-accordion data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionGroupName) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        acc.group_name = value;
                        applied = true;
                        "Updated dd-accordion group name.".to_string()
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].title = value;
                            applied = true;
                            format!("Updated dd-accordion item {} title.", ni + 1)
                        } else {
                            "dd-accordion has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].content = value;
                            applied = true;
                            format!("Updated dd-accordion item {} content.", ni + 1)
                        } else {
                            "dd-accordion has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Edit type no longer matches selected node.".to_string(),
        };
        if let PageNode::Section(section) = &mut page.nodes[idx] {
            push_legacy_components_into_selected_column(section, selected_column);
        }
        self.status = status;
        if clear_input {
            self.input_mode = None;
            self.input_buffer.clear();
            self.input_cursor = 0;
            self.multiline_scroll_row = 0;
        }
        applied
    }

    fn handle_click(&mut self, x: u16, y: u16) {
        if !contains(self.list_area, x, y) {
            return;
        }
        let tree_rows = self.build_node_tree_rows();
        if tree_rows.is_empty() {
            return;
        }
        let body_top = self.list_area.y.saturating_add(1);
        let body_bottom = self
            .list_area
            .y
            .saturating_add(self.list_area.height.saturating_sub(1));
        if y < body_top || y >= body_bottom {
            return;
        }
        let idx = (y - body_top) as usize;
        if idx < tree_rows.len() {
            self.selected_tree_row = idx;
            self.apply_tree_row_selection(tree_rows[idx]);
            self.status = format!("Selected {}", self.tree_row_label(&tree_rows[idx]));
        }
    }

    fn current_page(&self) -> &crate::model::Page {
        &self.site.pages[self.selected_page]
    }

    fn current_page_mut(&mut self) -> Option<&mut crate::model::Page> {
        self.site.pages.get_mut(self.selected_page)
    }

    fn selected_index_for_page(page: &crate::model::Page, selected_node: usize) -> Option<usize> {
        if page.nodes.is_empty() {
            None
        } else {
            Some(selected_node.min(page.nodes.len() - 1))
        }
    }

    fn build_node_tree_rows(&self) -> Vec<NodeTreeRow> {
        let page = self.current_page();
        let mut rows = Vec::new();
        for (node_idx, node) in page.nodes.iter().enumerate() {
            match node {
                PageNode::Hero(_) => rows.push(NodeTreeRow {
                    kind: NodeTreeKind::Hero { node_idx },
                }),
                PageNode::Section(section) => {
                    rows.push(NodeTreeRow {
                        kind: NodeTreeKind::Section { node_idx },
                    });
                    if self.is_section_expanded(node_idx) {
                        let columns = section_columns_ref(section);
                        for (column_idx, col) in columns.iter().enumerate() {
                            rows.push(NodeTreeRow {
                                kind: NodeTreeKind::Column {
                                    node_idx,
                                    column_idx,
                                },
                            });
                            for (component_idx, _) in col.components.iter().enumerate() {
                                rows.push(NodeTreeRow {
                                    kind: NodeTreeKind::Component {
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    },
                                });
                                if let Some(crate::model::SectionComponent::Accordion(acc)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_accordion_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in acc.items.iter().enumerate() {
                                            rows.push(NodeTreeRow {
                                                kind: NodeTreeKind::AccordionItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Alternating(alt)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_alternating_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in alt.items.iter().enumerate() {
                                            rows.push(NodeTreeRow {
                                                kind: NodeTreeKind::AlternatingItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Card(card)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_card_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in card.items.iter().enumerate() {
                                            rows.push(NodeTreeRow {
                                                kind: NodeTreeKind::CardItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Filmstrip(filmstrip)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_filmstrip_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in filmstrip.items.iter().enumerate() {
                                            rows.push(NodeTreeRow {
                                                kind: NodeTreeKind::FilmstripItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Milestones(
                                    milestones,
                                )) = col.components.get(component_idx)
                                {
                                    if self.is_milestones_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in milestones.items.iter().enumerate() {
                                            rows.push(NodeTreeRow {
                                                kind: NodeTreeKind::MilestonesItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        rows
    }

    fn tree_row_label(&self, row: &NodeTreeRow) -> String {
        let page = self.current_page();
        match row.kind {
            NodeTreeKind::Hero { node_idx } => format!("{}. dd-hero", node_idx + 1),
            NodeTreeKind::Section { node_idx } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("{}. dd-section", node_idx + 1);
                };
                let marker = if self.is_section_expanded(node_idx) {
                    "[-]"
                } else {
                    "[+]"
                };
                format!("{}. {} dd-section ({})", node_idx + 1, marker, section.id)
            }
            NodeTreeKind::Column {
                node_idx,
                column_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("    |- column {}", column_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let col = &columns[col_i];
                format!(
                    "    |- column {} ({}) [{}]",
                    col_i + 1,
                    col.id,
                    col.width_class
                )
            }
            NodeTreeKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("       - component {}", component_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let component = &columns[col_i].components[comp_i];
                let label = component_label(component);
                if matches!(component, crate::model::SectionComponent::Accordion(_)) {
                    let marker = if self.is_accordion_items_expanded(node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Alternating(_)) {
                    let marker = if self.is_alternating_items_expanded(node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Card(_)) {
                    let marker = if self.is_card_items_expanded(node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Filmstrip(_)) {
                    let marker = if self.is_filmstrip_items_expanded(node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Milestones(_)) {
                    let marker = if self.is_milestones_items_expanded(node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else {
                    format!("       - {} {}", comp_i + 1, label)
                }
            }
            NodeTreeKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let title = if let Some(crate::model::SectionComponent::Accordion(acc)) =
                    columns[col_i].components.get(comp_i)
                {
                    acc.items
                        .get(item_idx)
                        .map(|i| i.title.as_str())
                        .unwrap_or("(none)")
                } else {
                    "(none)"
                };
                let marker = if node_idx == self.selected_node
                    && col_i == self.selected_column
                    && comp_i == self.selected_component
                    && item_idx == self.selected_nested_item
                {
                    "*"
                } else {
                    "-"
                };
                format!(
                    "          {} item {}: {}",
                    marker,
                    item_idx + 1,
                    truncate_ascii(title, 48)
                )
            }
            NodeTreeKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let title = if let Some(crate::model::SectionComponent::Alternating(alt)) =
                    columns[col_i].components.get(comp_i)
                {
                    alt.items
                        .get(item_idx)
                        .map(|i| i.title.as_str())
                        .unwrap_or("(none)")
                } else {
                    "(none)"
                };
                let marker = if node_idx == self.selected_node
                    && col_i == self.selected_column
                    && comp_i == self.selected_component
                    && item_idx == self.selected_nested_item
                {
                    "*"
                } else {
                    "-"
                };
                format!(
                    "          {} item {}: {}",
                    marker,
                    item_idx + 1,
                    truncate_ascii(title, 48)
                )
            }
            NodeTreeKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let title = if let Some(crate::model::SectionComponent::Card(card)) =
                    columns[col_i].components.get(comp_i)
                {
                    card.items
                        .get(item_idx)
                        .map(|i| i.card_title.as_str())
                        .unwrap_or("(none)")
                } else {
                    "(none)"
                };
                let marker = if node_idx == self.selected_node
                    && col_i == self.selected_column
                    && comp_i == self.selected_component
                    && item_idx == self.selected_nested_item
                {
                    "*"
                } else {
                    "-"
                };
                format!(
                    "          {} item {}: {}",
                    marker,
                    item_idx + 1,
                    truncate_ascii(title, 48)
                )
            }
            NodeTreeKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let title = if let Some(crate::model::SectionComponent::Filmstrip(filmstrip)) =
                    columns[col_i].components.get(comp_i)
                {
                    filmstrip
                        .items
                        .get(item_idx)
                        .map(|i| i.title.as_str())
                        .unwrap_or("(none)")
                } else {
                    "(none)"
                };
                let marker = if node_idx == self.selected_node
                    && col_i == self.selected_column
                    && comp_i == self.selected_component
                    && item_idx == self.selected_nested_item
                {
                    "*"
                } else {
                    "-"
                };
                format!(
                    "          {} item {}: {}",
                    marker,
                    item_idx + 1,
                    truncate_ascii(title, 48)
                )
            }
            NodeTreeKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let PageNode::Section(section) = &page.nodes[node_idx] else {
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = column_idx.min(columns.len().saturating_sub(1));
                let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
                let title = if let Some(crate::model::SectionComponent::Milestones(milestones)) =
                    columns[col_i].components.get(comp_i)
                {
                    milestones
                        .items
                        .get(item_idx)
                        .map(|i| i.child_title.as_str())
                        .unwrap_or("(none)")
                } else {
                    "(none)"
                };
                let marker = if node_idx == self.selected_node
                    && col_i == self.selected_column
                    && comp_i == self.selected_component
                    && item_idx == self.selected_nested_item
                {
                    "*"
                } else {
                    "-"
                };
                format!(
                    "          {} item {}: {}",
                    marker,
                    item_idx + 1,
                    truncate_ascii(title, 48)
                )
            }
        }
    }

    fn apply_tree_row_selection(&mut self, row: NodeTreeRow) {
        match row.kind {
            NodeTreeKind::Hero { node_idx } => {
                self.selected_node = node_idx;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            NodeTreeKind::Section { node_idx } => {
                self.selected_node = node_idx;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            NodeTreeKind::Column {
                node_idx,
                column_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            NodeTreeKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = 0;
            }
            NodeTreeKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            NodeTreeKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            NodeTreeKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            NodeTreeKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            NodeTreeKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
        }
    }

    fn sync_tree_row_with_selection(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.selected_tree_row = 0;
            return;
        }
        let row_matches_selection = |row: &NodeTreeRow| match row.kind {
            NodeTreeKind::Hero { node_idx } => node_idx == self.selected_node,
            NodeTreeKind::Section { node_idx } => node_idx == self.selected_node,
            NodeTreeKind::Column {
                node_idx,
                column_idx,
            } => node_idx == self.selected_node && column_idx == self.selected_column,
            NodeTreeKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && self.selected_nested_item == 0
            }
            NodeTreeKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            NodeTreeKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            NodeTreeKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            NodeTreeKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            NodeTreeKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
        };

        if let Some(current) = rows.get(self.selected_tree_row) {
            if row_matches_selection(current) {
                return;
            }
        }

        let wanted = rows
            .iter()
            .position(row_matches_selection)
            .unwrap_or_else(|| self.selected_tree_row.min(rows.len().saturating_sub(1)));
        self.selected_tree_row = wanted;
    }

    fn is_section_expanded(&self, node_idx: usize) -> bool {
        !self
            .expanded_sections
            .contains(&(self.selected_page, node_idx))
    }

    fn set_section_expanded(&mut self, node_idx: usize, expanded: bool) {
        if expanded {
            self.expanded_sections
                .remove(&(self.selected_page, node_idx));
        } else {
            self.expanded_sections
                .insert((self.selected_page, node_idx));
        }
    }

    fn is_accordion_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_accordion_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    fn set_accordion_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_accordion_items.remove(&key);
        } else {
            self.expanded_accordion_items.insert(key);
        }
    }

    fn is_alternating_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_alternating_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    fn set_alternating_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_alternating_items.remove(&key);
        } else {
            self.expanded_alternating_items.insert(key);
        }
    }

    fn is_card_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_card_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    fn set_card_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_card_items.remove(&key);
        } else {
            self.expanded_card_items.insert(key);
        }
    }

    fn is_filmstrip_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_filmstrip_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    fn set_filmstrip_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_filmstrip_items.remove(&key);
        } else {
            self.expanded_filmstrip_items.insert(key);
        }
    }

    fn is_milestones_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_milestones_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    fn set_milestones_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_milestones_items.remove(&key);
        } else {
            self.expanded_milestones_items.insert(key);
        }
    }

    fn toggle_selected_tree_expanded(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if let NodeTreeKind::Component {
            node_idx,
            column_idx,
            component_idx,
        }
        | NodeTreeKind::AccordionItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | NodeTreeKind::AlternatingItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | NodeTreeKind::CardItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | NodeTreeKind::FilmstripItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | NodeTreeKind::MilestonesItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        } = row.kind
        {
            let page = self.current_page();
            let Some(PageNode::Section(section)) = page.nodes.get(node_idx) else {
                self.status = "Selected row is not a section.".to_string();
                return;
            };
            let columns = section_columns_ref(section);
            let col_i = column_idx.min(columns.len().saturating_sub(1));
            let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Accordion(_))
            ) {
                let expanded = self.is_accordion_items_expanded(node_idx, col_i, comp_i);
                self.set_accordion_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                self.status = if expanded {
                    "Collapsed accordion items.".to_string()
                } else {
                    "Expanded accordion items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Alternating(_))
            ) {
                let expanded = self.is_alternating_items_expanded(node_idx, col_i, comp_i);
                self.set_alternating_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                self.status = if expanded {
                    "Collapsed alternating items.".to_string()
                } else {
                    "Expanded alternating items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Card(_))
            ) {
                let expanded = self.is_card_items_expanded(node_idx, col_i, comp_i);
                self.set_card_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                self.status = if expanded {
                    "Collapsed card items.".to_string()
                } else {
                    "Expanded card items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Filmstrip(_))
            ) {
                let expanded = self.is_filmstrip_items_expanded(node_idx, col_i, comp_i);
                self.set_filmstrip_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                self.status = if expanded {
                    "Collapsed filmstrip items.".to_string()
                } else {
                    "Expanded filmstrip items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Milestones(_))
            ) {
                let expanded = self.is_milestones_items_expanded(node_idx, col_i, comp_i);
                self.set_milestones_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                self.status = if expanded {
                    "Collapsed milestones items.".to_string()
                } else {
                    "Expanded milestones items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
        }
        let node_idx = match row.kind {
            NodeTreeKind::Section { node_idx } => node_idx,
            NodeTreeKind::Column { node_idx, .. } => node_idx,
            NodeTreeKind::Component { node_idx, .. } => node_idx,
            NodeTreeKind::AccordionItem { node_idx, .. } => node_idx,
            NodeTreeKind::AlternatingItem { node_idx, .. } => node_idx,
            NodeTreeKind::CardItem { node_idx, .. } => node_idx,
            NodeTreeKind::FilmstripItem { node_idx, .. } => node_idx,
            NodeTreeKind::MilestonesItem { node_idx, .. } => node_idx,
            NodeTreeKind::Hero { .. } => {
                self.status = "Selected row is not a section.".to_string();
                return;
            }
        };
        let page = self.current_page();
        let Some(PageNode::Section(_)) = page.nodes.get(node_idx) else {
            self.status = "Selected row is not a section.".to_string();
            return;
        };
        let expanded = self.is_section_expanded(node_idx);
        self.set_section_expanded(node_idx, !expanded);
        self.selected_node = node_idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = if expanded {
            "Collapsed section.".to_string()
        } else {
            "Expanded section.".to_string()
        };
        self.sync_tree_row_with_selection();
    }

    fn handle_enter_on_selected_row(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        match row.kind {
            NodeTreeKind::Section { .. } => self.begin_edit_selected(),
            NodeTreeKind::Hero { .. } => self.begin_edit_selected(),
            NodeTreeKind::Column { .. } => self.begin_edit_selected_column_width_class(),
            NodeTreeKind::Component { .. } => self.begin_edit_selected_component_primary(),
            NodeTreeKind::AccordionItem { .. } => {
                if self.set_component_input_mode(InputMode::EditAccordionFirstTitle) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            NodeTreeKind::AlternatingItem { .. } => {
                if self.set_component_input_mode(InputMode::EditAlternatingItemTitle) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            NodeTreeKind::CardItem { .. } => {
                if self.set_component_input_mode(InputMode::EditCardItemImageUrl) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            NodeTreeKind::FilmstripItem { .. } => {
                if self.set_component_input_mode(InputMode::EditFilmstripItemImageUrl) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            NodeTreeKind::MilestonesItem { .. } => {
                if self.set_component_input_mode(InputMode::EditMilestonesItemPercentage) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
        }
    }

    fn open_component_picker(&mut self) {
        self.input_mode = None;
        self.component_picker = Some(ComponentPickerState {
            query: String::new(),
            selected: 0,
        });
        self.status = "Insert picker opened.".to_string();
    }

    fn insert_selected_component_kind(&mut self) {
        match self.component_kind {
            ComponentKind::Hero => self.add_hero(),
            ComponentKind::Section => self.add_section(),
            _ => self.add_selected_component_to_section(),
        }
    }

    fn normalize_component_picker_selection(&mut self) {
        let (query, selected) = if let Some(picker) = &self.component_picker {
            (picker.query.clone(), picker.selected)
        } else {
            return;
        };
        let total = self.filtered_component_kinds(&query).len();
        if let Some(picker) = &mut self.component_picker {
            picker.selected = if total == 0 {
                0
            } else {
                selected.min(total - 1)
            };
        }
    }

    fn filtered_component_kinds(&self, query: &str) -> Vec<ComponentKind> {
        let all = ComponentKind::all();
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return all.to_vec();
        }
        let mut scored = Vec::new();
        for kind in all.iter().copied() {
            let hay = component_search_haystack(kind);
            if let Some(score) = fuzzy_score(&q, hay.as_str()) {
                scored.push((kind, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.label().cmp(b.0.label())));
        scored.into_iter().map(|(kind, _)| kind).collect()
    }

    fn selection_summary(&self) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "(none)".to_string();
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        match &page.nodes[ni] {
            PageNode::Hero(_) => format!("node {} (dd-hero)", ni + 1),
            PageNode::Section(section) => format!(
                "node {} (dd-section:{}), column {}, component {}",
                ni + 1,
                section.id,
                self.selected_column + 1,
                self.selected_component + 1
            ),
        }
    }

    fn current_input_mode_label(&self) -> &'static str {
        match self.input_mode {
            Some(InputMode::EditHeroImage) => "hero.image",
            Some(InputMode::EditHeroClass) => "hero.class",
            Some(InputMode::EditHeroAos) => "hero.data_aos",
            Some(InputMode::EditHeroCustomCss) => "hero.custom_css",
            Some(InputMode::EditHeroTitle) => "hero.title",
            Some(InputMode::EditHeroSubtitle) => "hero.subtitle",
            Some(InputMode::EditHeroCopy) => "hero.copy",
            Some(InputMode::EditHeroCtaText) => "hero.link_1.text",
            Some(InputMode::EditHeroCtaLink) => "hero.link_1.url",
            Some(InputMode::EditHeroCtaTarget) => "hero.link_1.target",
            Some(InputMode::EditHeroCtaText2) => "hero.link_2.text",
            Some(InputMode::EditHeroCtaLink2) => "hero.link_2.url",
            Some(InputMode::EditHeroCtaTarget2) => "hero.link_2.target",
            Some(InputMode::EditSectionId) => "section.id",
            Some(InputMode::EditSectionTitle) => "section.title",
            Some(InputMode::EditSectionClass) => "section.class",
            Some(InputMode::EditColumnId) => "section.column.id",
            Some(InputMode::EditColumnWidthClass) => "section.column.width_class",
            Some(InputMode::EditAlternatingType) => "dd-alternating.type",
            Some(InputMode::EditAlternatingClass) => "dd-alternating.class",
            Some(InputMode::EditAlternatingDataAos) => "dd-alternating.data_aos",
            Some(InputMode::EditAlternatingItemImage) => "dd-alternating.active.image",
            Some(InputMode::EditAlternatingItemImageAlt) => "dd-alternating.active.image_alt",
            Some(InputMode::EditAlternatingItemTitle) => "dd-alternating.active.title",
            Some(InputMode::EditAlternatingItemCopy) => "dd-alternating.active.copy",
            Some(InputMode::EditBannerClass) => "dd-banner.class",
            Some(InputMode::EditBannerDataAos) => "dd-banner.data_aos",
            Some(InputMode::EditBannerImageUrl) => "dd-banner_image_url",
            Some(InputMode::EditBannerImageAlt) => "dd-banner_image_alt",
            Some(InputMode::EditCtaClass) => "dd-cta.class",
            Some(InputMode::EditCtaImageUrl) => "dd-cta_image_url",
            Some(InputMode::EditCtaImageAlt) => "dd-cta_image_alt",
            Some(InputMode::EditCtaDataAos) => "dd-cta.data_aos",
            Some(InputMode::EditCtaTitle) => "dd-cta_title",
            Some(InputMode::EditCtaSubtitle) => "dd-cta_subtitle",
            Some(InputMode::EditCtaCopy) => "dd-cta_copy",
            Some(InputMode::EditCtaLinkUrl) => "dd-cta_link_url",
            Some(InputMode::EditCtaLinkTarget) => "dd-cta_link_target",
            Some(InputMode::EditCtaLinkLabel) => "dd-cta_link_label",
            Some(InputMode::EditFilmstripType) => "dd-filmstrip.type",
            Some(InputMode::EditFilmstripDataAos) => "dd-filmstrip.data_aos",
            Some(InputMode::EditFilmstripItemImageUrl) => "dd-filmstrip.active.image_url",
            Some(InputMode::EditFilmstripItemImageAlt) => "dd-filmstrip.active.image_alt",
            Some(InputMode::EditFilmstripItemTitle) => "dd-filmstrip.active.title",
            Some(InputMode::EditMilestonesDataAos) => "dd-milestones.parent_data_aos",
            Some(InputMode::EditMilestonesWidth) => "dd-milestones.parent_width",
            Some(InputMode::EditMilestonesItemPercentage) => {
                "dd-milestones.active.child_percentage"
            }
            Some(InputMode::EditMilestonesItemTitle) => "dd-milestones.active.child_title",
            Some(InputMode::EditMilestonesItemSubtitle) => "dd-milestones.active.child_subtitle",
            Some(InputMode::EditMilestonesItemCopy) => "dd-milestones.active.child_copy",
            Some(InputMode::EditMilestonesItemLinkUrl) => "dd-milestones.active.child_link_url",
            Some(InputMode::EditMilestonesItemLinkTarget) => {
                "dd-milestones.active.child_link_target"
            }
            Some(InputMode::EditMilestonesItemLinkLabel) => "dd-milestones.active.child_link_label",
            Some(InputMode::EditBlockquoteDataAos) => "dd-blockquote.data_aos",
            Some(InputMode::EditBlockquoteImageUrl) => "blockquote_image_url",
            Some(InputMode::EditBlockquoteImageAlt) => "blockquote_image_alt",
            Some(InputMode::EditBlockquotePersonsName) => "blockquote_persons_name",
            Some(InputMode::EditBlockquotePersonsTitle) => "blockquote_persons_title",
            Some(InputMode::EditBlockquoteCopy) => "blockquote_copy",
            Some(InputMode::EditCardType) => "card_type",
            Some(InputMode::EditCardDataAos) => "card_data_aos",
            Some(InputMode::EditCardWidth) => "card_width",
            Some(InputMode::EditCardItemImageUrl) => "dd-card.active.card_image_url",
            Some(InputMode::EditCardItemImageAlt) => "dd-card.active.card_image_alt",
            Some(InputMode::EditCardItemTitle) => "dd-card.active.card_title",
            Some(InputMode::EditCardItemSubtitle) => "dd-card.active.card_subtitle",
            Some(InputMode::EditCardItemCopy) => "dd-card.active.card_copy",
            Some(InputMode::EditCardItemLinkUrl) => "dd-card.active.card_link_url",
            Some(InputMode::EditCardItemLinkTarget) => "dd-card.active.card_link_target",
            Some(InputMode::EditCardItemLinkLabel) => "dd-card.active.card_link_label",
            Some(InputMode::EditAccordionType) => "dd-accordion.type",
            Some(InputMode::EditAccordionClass) => "dd-accordion.class",
            Some(InputMode::EditAccordionAos) => "dd-accordion.data_aos",
            Some(InputMode::EditAccordionGroupName) => "dd-accordion.group_name",
            Some(InputMode::EditAccordionFirstTitle) => "dd-accordion.active.title",
            Some(InputMode::EditAccordionFirstContent) => "dd-accordion.active.content",
            None => "field",
        }
    }

    fn current_modal_fields(&self) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "(none)".to_string();
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        match &page.nodes[ni] {
            PageNode::Hero(v) => format!(
                "- hero.image: {}\n- hero.class: {}\n- hero.data_aos: {}\n- hero.custom_css: {}\n- hero.title: {}\n- hero.subtitle: {}\n- hero.copy: {}\n- hero.link_1.text: {}\n- hero.link_1.url: {}\n- hero.link_1.target: {}\n- hero.link_2.text: {}\n- hero.link_2.url: {}\n- hero.link_2.target: {}",
                v.image,
                hero_image_class_to_str(
                    v.hero_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
                hero_aos_to_str(v.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn)),
                v.custom_css.as_deref().unwrap_or("(none)"),
                v.title,
                v.subtitle,
                v.copy.as_deref().unwrap_or("(none)"),
                v.cta_text.as_deref().unwrap_or("(none)"),
                v.cta_link.as_deref().unwrap_or("(none)"),
                cta_target_to_str(v.cta_target.unwrap_or(crate::model::CtaTarget::SelfTarget)),
                v.cta_text_2.as_deref().unwrap_or("(none)"),
                v.cta_link_2.as_deref().unwrap_or("(none)"),
                cta_target_to_str(
                    v.cta_target_2
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                )
            ),
            PageNode::Section(section) => {
                let rows = self.build_node_tree_rows();
                let row_kind = rows
                    .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                    .map(|row| row.kind);
                match row_kind {
                    Some(NodeTreeKind::Column { .. }) => {
                        let columns = section_columns_ref(section);
                        if let Some(col) =
                            columns.get(self.selected_column.min(columns.len().saturating_sub(1)))
                        {
                            vec![
                                format!("- column.id: {}", col.id),
                                format!("- column.width_class: {}", col.width_class),
                            ]
                            .join("\n")
                        } else {
                            "(none)".to_string()
                        }
                    }
                    Some(NodeTreeKind::Component { .. })
                    | Some(NodeTreeKind::AccordionItem { .. })
                    | Some(NodeTreeKind::AlternatingItem { .. })
                    | Some(NodeTreeKind::CardItem { .. })
                    | Some(NodeTreeKind::FilmstripItem { .. })
                    | Some(NodeTreeKind::MilestonesItem { .. }) => {
                        let columns = section_columns_ref(section);
                        if let Some(col) =
                            columns.get(self.selected_column.min(columns.len().saturating_sub(1)))
                        {
                            if let Some(component) = col.components.get(
                                self.selected_component
                                    .min(col.components.len().saturating_sub(1)),
                            ) {
                                if let crate::model::SectionComponent::Card(card) = component {
                                    match self.input_mode {
                                        Some(InputMode::EditCardType)
                                        | Some(InputMode::EditCardDataAos)
                                        | Some(InputMode::EditCardWidth) => vec![
                                            format!(
                                                "- card_type: {}",
                                                card_type_to_str(card.card_type)
                                            ),
                                            format!(
                                                "- card_data_aos: {}",
                                                hero_aos_to_str(card.card_data_aos)
                                            ),
                                            format!("- card_width: {}", card.card_width),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditCardItemImageUrl)
                                        | Some(InputMode::EditCardItemImageAlt)
                                        | Some(InputMode::EditCardItemTitle)
                                        | Some(InputMode::EditCardItemSubtitle)
                                        | Some(InputMode::EditCardItemCopy)
                                        | Some(InputMode::EditCardItemLinkUrl)
                                        | Some(InputMode::EditCardItemLinkTarget)
                                        | Some(InputMode::EditCardItemLinkLabel) => {
                                            let item = nested_index(
                                                card.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| card.items.get(i));
                                            vec![
                                                format!(
                                                    "- card_image_url: {}",
                                                    item.map(|i| i.card_image_url.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_image_alt: {}",
                                                    item.map(|i| i.card_image_alt.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_title: {}",
                                                    item.map(|i| i.card_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_subtitle: {}",
                                                    item.map(|i| i.card_subtitle.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_copy: {}",
                                                    item.map(|i| i.card_copy.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_link_url: {}",
                                                    item.and_then(|i| i.card_link_url.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- card_link_target: {}",
                                                    item.and_then(|i| i.card_link_target)
                                                        .map(card_link_target_to_str)
                                                        .unwrap_or("_self")
                                                ),
                                                format!(
                                                    "- card_link_label: {}",
                                                    item.and_then(|i| i.card_link_label.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditFilmstripType)
                                        | Some(InputMode::EditFilmstripDataAos) => vec![
                                            format!(
                                                "- parent_type: {}",
                                                filmstrip_type_to_str(filmstrip.filmstrip_type)
                                            ),
                                            format!(
                                                "- parent_data_aos: {}",
                                                hero_aos_to_str(filmstrip.filmstrip_data_aos)
                                            ),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditFilmstripItemImageUrl)
                                        | Some(InputMode::EditFilmstripItemImageAlt)
                                        | Some(InputMode::EditFilmstripItemTitle) => {
                                            let item = nested_index(
                                                filmstrip.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| filmstrip.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_image_url: {}",
                                                    item.map(|i| i.image_url.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_image_alt: {}",
                                                    item.map(|i| i.image_alt.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Milestones(
                                    milestones,
                                ) = component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditMilestonesDataAos)
                                        | Some(InputMode::EditMilestonesWidth) => vec![
                                            format!(
                                                "- parent_data_aos: {}",
                                                hero_aos_to_str(milestones.parent_data_aos)
                                            ),
                                            format!("- parent_width: {}", milestones.parent_width),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditMilestonesItemPercentage)
                                        | Some(InputMode::EditMilestonesItemTitle)
                                        | Some(InputMode::EditMilestonesItemSubtitle)
                                        | Some(InputMode::EditMilestonesItemCopy)
                                        | Some(InputMode::EditMilestonesItemLinkUrl)
                                        | Some(InputMode::EditMilestonesItemLinkTarget)
                                        | Some(InputMode::EditMilestonesItemLinkLabel) => {
                                            let item = nested_index(
                                                milestones.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| milestones.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_percentage: {}",
                                                    item.map(|i| i.child_percentage.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.child_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_subtitle: {}",
                                                    item.map(|i| i.child_subtitle.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_copy: {}",
                                                    item.map(|i| i.child_copy.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_url: {}",
                                                    item.and_then(|i| i.child_link_url.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_target: {}",
                                                    item.and_then(|i| i.child_link_target)
                                                        .map(card_link_target_to_str)
                                                        .unwrap_or("_self")
                                                ),
                                                format!(
                                                    "- child_link_label: {}",
                                                    item.and_then(|i| i
                                                        .child_link_label
                                                        .as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Alternating(alt) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditAlternatingType)
                                        | Some(InputMode::EditAlternatingClass)
                                        | Some(InputMode::EditAlternatingDataAos) => vec![
                                            format!(
                                                "- alternating_type: {}",
                                                alternating_type_to_str(alt.alternating_type)
                                            ),
                                            format!(
                                                "- alternating.class: {}",
                                                alt.alternating_class
                                            ),
                                            format!(
                                                "- alternating.data_aos: {}",
                                                hero_aos_to_str(alt.alternating_data_aos)
                                            ),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditAlternatingItemImage)
                                        | Some(InputMode::EditAlternatingItemImageAlt)
                                        | Some(InputMode::EditAlternatingItemTitle)
                                        | Some(InputMode::EditAlternatingItemCopy) => {
                                            let image = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.image.as_str())
                                            .unwrap_or("(none)");
                                            let image_alt = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.image_alt.as_str())
                                            .unwrap_or("(none)");
                                            let title = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.title.as_str())
                                            .unwrap_or("(none)");
                                            let copy = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.copy.as_str())
                                            .unwrap_or("(none)");
                                            vec![
                                                format!("- alternating_image: {}", image),
                                                format!("- alternating_image_alt: {}", image_alt),
                                                format!("- alternating_title: {}", title),
                                                format!("- alternating_copy: {}", copy),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Accordion(acc) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditAccordionType)
                                        | Some(InputMode::EditAccordionClass)
                                        | Some(InputMode::EditAccordionAos)
                                        | Some(InputMode::EditAccordionGroupName) => vec![
                                            format!(
                                                "- accordion_type: {}",
                                                accordion_type_to_str(acc.accordion_type)
                                            ),
                                            format!(
                                                "- accordion.class: {}",
                                                accordion_class_to_str(acc.accordion_class)
                                            ),
                                            format!(
                                                "- accordion.data_aos: {}",
                                                hero_aos_to_str(acc.accordion_aos)
                                            ),
                                            format!("- accordion.group_name: {}", acc.group_name),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditAccordionFirstTitle)
                                        | Some(InputMode::EditAccordionFirstContent) => {
                                            let title = nested_index(
                                                acc.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| acc.items.get(i))
                                            .map(|i| i.title.as_str())
                                            .unwrap_or("(none)");
                                            let content = nested_index(
                                                acc.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| acc.items.get(i))
                                            .map(|i| i.content.as_str())
                                            .unwrap_or("(none)");
                                            vec![
                                                format!("- accordion_title: {}", title),
                                                format!("- accordion_copy: {}", content),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else {
                                    component_form(component, self.selected_nested_item)
                                }
                            } else {
                                "(none)".to_string()
                            }
                        } else {
                            "(none)".to_string()
                        }
                    }
                    _ => vec![
                        format!("- section.id: {}", section.id),
                        format!(
                            "- section.title: {}",
                            section.section_title.as_deref().unwrap_or("(none)")
                        ),
                        format!(
                            "- section.class: {}",
                            section_class_to_str(
                                section
                                    .section_class
                                    .unwrap_or(crate::model::SectionClass::FullContained)
                            )
                        ),
                    ]
                    .join("\n"),
                }
            }
        }
    }

    fn set_component_input_mode(&mut self, mode: InputMode) -> bool {
        let Some(value) = self.value_for_component_mode(mode) else {
            return false;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.clamp_multiline_input_if_needed();
        self.input_cursor = self.input_buffer.chars().count();
        self.ensure_multiline_cursor_visible();
        self.status = match mode {
            InputMode::EditHeroImage => {
                "Editing hero image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroClass => {
                "Editing hero default class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroAos => {
                "Editing hero data-aos option. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCustomCss => {
                "Editing hero custom CSS classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget => {
                "Editing hero primary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget2 => {
                "Editing hero secondary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionId => {
                "Editing section id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionTitle => {
                "Editing section title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionClass => {
                "Editing section class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingType => {
                "Editing dd-alternating type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingClass => {
                "Editing dd-alternating class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingDataAos => {
                "Editing dd-alternating data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImage => {
                "Editing dd-alternating item image. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImageAlt => {
                "Editing dd-alternating item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemTitle => {
                "Editing dd-alternating item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemCopy => {
                "Editing dd-alternating item copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditBannerClass => {
                "Editing dd-banner class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerDataAos => {
                "Editing dd-banner data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageUrl => {
                "Editing dd-banner image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageAlt => {
                "Editing dd-banner image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaClass => {
                "Editing dd-cta class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageUrl => {
                "Editing dd-cta image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageAlt => {
                "Editing dd-cta image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaDataAos => {
                "Editing dd-cta data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaSubtitle => {
                "Editing dd-cta subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaCopy => {
                "Editing dd-cta copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCtaLinkUrl => {
                "Editing dd-cta link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkTarget => {
                "Editing dd-cta link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkLabel => {
                "Editing dd-cta link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripType => {
                "Editing dd-filmstrip type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripDataAos => {
                "Editing dd-filmstrip data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageUrl => {
                "Editing dd-filmstrip item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageAlt => {
                "Editing dd-filmstrip item image alt text. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditFilmstripItemTitle => {
                "Editing dd-filmstrip item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesDataAos => {
                "Editing dd-milestones parent_data_aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesWidth => {
                "Editing dd-milestones parent_width. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemPercentage => {
                "Editing dd-milestones child_percentage. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemTitle => {
                "Editing dd-milestones child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemSubtitle => {
                "Editing dd-milestones child_subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemCopy => {
                "Editing dd-milestones child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditMilestonesItemLinkUrl => {
                "Editing dd-milestones child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkTarget => {
                "Editing dd-milestones child_link_target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkLabel => {
                "Editing dd-milestones child_link_label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteDataAos => {
                "Editing dd-blockquote data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageUrl => {
                "Editing dd-blockquote image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageAlt => {
                "Editing dd-blockquote image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsName => {
                "Editing dd-blockquote person name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsTitle => {
                "Editing dd-blockquote person title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteCopy => {
                "Editing dd-blockquote copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardType => {
                "Editing dd-card type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardDataAos => {
                "Editing dd-card data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardWidth => {
                "Editing dd-card width classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageUrl => {
                "Editing dd-card item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageAlt => {
                "Editing dd-card item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemTitle => {
                "Editing dd-card item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemSubtitle => {
                "Editing dd-card item subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemCopy => {
                "Editing dd-card item copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardItemLinkUrl => {
                "Editing dd-card item link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkTarget => {
                "Editing dd-card item link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkLabel => {
                "Editing dd-card item link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionType => {
                "Editing dd-accordion type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionClass => {
                "Editing dd-accordion class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionAos => {
                "Editing dd-accordion data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionGroupName => {
                "Editing dd-accordion group name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstTitle => {
                "Editing dd-accordion item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstContent => {
                "Editing dd-accordion item content. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            _ => "Editing field. Enter to save, esc to cancel.".to_string(),
        };
        true
    }

    fn value_for_component_mode(&self, mode: InputMode) -> Option<String> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let node_idx = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[node_idx] {
            PageNode::Hero(hero) => match mode {
                InputMode::EditHeroImage => Some(hero.image.clone()),
                InputMode::EditHeroClass => Some(
                    hero_image_class_to_str(
                        hero.hero_class
                            .unwrap_or(crate::model::HeroImageClass::FullFull),
                    )
                    .to_string(),
                ),
                InputMode::EditHeroAos => Some(
                    hero_aos_to_str(hero.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn))
                        .to_string(),
                ),
                InputMode::EditHeroCustomCss => Some(hero.custom_css.clone().unwrap_or_default()),
                InputMode::EditHeroTitle => Some(hero.title.clone()),
                InputMode::EditHeroSubtitle => Some(hero.subtitle.clone()),
                InputMode::EditHeroCopy => Some(hero.copy.clone().unwrap_or_default()),
                InputMode::EditHeroCtaText => Some(hero.cta_text.clone().unwrap_or_default()),
                InputMode::EditHeroCtaLink => Some(hero.cta_link.clone().unwrap_or_default()),
                InputMode::EditHeroCtaTarget => Some(
                    cta_target_to_str(
                        hero.cta_target
                            .unwrap_or(crate::model::CtaTarget::SelfTarget),
                    )
                    .to_string(),
                ),
                InputMode::EditHeroCtaText2 => Some(hero.cta_text_2.clone().unwrap_or_default()),
                InputMode::EditHeroCtaLink2 => Some(hero.cta_link_2.clone().unwrap_or_default()),
                InputMode::EditHeroCtaTarget2 => Some(
                    cta_target_to_str(
                        hero.cta_target_2
                            .unwrap_or(crate::model::CtaTarget::SelfTarget),
                    )
                    .to_string(),
                ),
                _ => None,
            },
            PageNode::Section(section) => match mode {
                InputMode::EditSectionId => Some(section.id.clone()),
                InputMode::EditSectionTitle => {
                    Some(section.section_title.clone().unwrap_or_default())
                }
                InputMode::EditSectionClass => Some(
                    section_class_to_str(
                        section
                            .section_class
                            .unwrap_or(crate::model::SectionClass::FullContained),
                    )
                    .to_string(),
                ),
                InputMode::EditColumnId | InputMode::EditColumnWidthClass => {
                    let columns = section_columns_ref(section);
                    let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                    match mode {
                        InputMode::EditColumnId => Some(columns[col_i].id.clone()),
                        InputMode::EditColumnWidthClass => Some(columns[col_i].width_class.clone()),
                        _ => None,
                    }
                }
                _ => {
                    let columns = section_columns_ref(section);
                    let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                    let ci =
                        component_index(columns[col_i].components.len(), self.selected_component)?;
                    let component = columns[col_i].components.get(ci)?;
                    match (mode, component) {
                        (
                            InputMode::EditAlternatingType,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(alternating_type_to_str(v.alternating_type).to_string()),
                        (
                            InputMode::EditAlternatingClass,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(v.alternating_class.clone()),
                        (
                            InputMode::EditAlternatingDataAos,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(hero_aos_to_str(v.alternating_data_aos).to_string()),
                        (
                            InputMode::EditAlternatingItemImage,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].image.clone())
                        }
                        (
                            InputMode::EditAlternatingItemImageAlt,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].image_alt.clone())
                        }
                        (
                            InputMode::EditAlternatingItemTitle,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].title.clone())
                        }
                        (
                            InputMode::EditAlternatingItemCopy,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].copy.clone())
                        }
                        (InputMode::EditBannerClass, crate::model::SectionComponent::Banner(v)) => {
                            Some(banner_class_to_str(v.banner_class).to_string())
                        }
                        (
                            InputMode::EditBannerDataAos,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(hero_aos_to_str(v.banner_data_aos).to_string()),
                        (
                            InputMode::EditBannerImageUrl,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(v.banner_image_url.clone()),
                        (
                            InputMode::EditBannerImageAlt,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(v.banner_image_alt.clone()),
                        (InputMode::EditCtaClass, crate::model::SectionComponent::Cta(v)) => {
                            Some(cta_class_to_str(v.cta_class).to_string())
                        }
                        (InputMode::EditCtaImageUrl, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_image_url.clone())
                        }
                        (InputMode::EditCtaImageAlt, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_image_alt.clone())
                        }
                        (InputMode::EditCtaDataAos, crate::model::SectionComponent::Cta(v)) => {
                            Some(hero_aos_to_str(v.cta_data_aos).to_string())
                        }
                        (InputMode::EditCtaTitle, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_title.clone())
                        }
                        (InputMode::EditCtaSubtitle, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_subtitle.clone())
                        }
                        (InputMode::EditCtaCopy, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_copy.clone())
                        }
                        (InputMode::EditCtaLinkUrl, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_link_url.clone().unwrap_or_default())
                        }
                        (InputMode::EditCtaLinkTarget, crate::model::SectionComponent::Cta(v)) => {
                            Some(
                                v.cta_link_target
                                    .map(card_link_target_to_str)
                                    .unwrap_or("_self")
                                    .to_string(),
                            )
                        }
                        (InputMode::EditCtaLinkLabel, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.cta_link_label.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditFilmstripType,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => Some(filmstrip_type_to_str(v.filmstrip_type).to_string()),
                        (
                            InputMode::EditFilmstripDataAos,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => Some(hero_aos_to_str(v.filmstrip_data_aos).to_string()),
                        (
                            InputMode::EditFilmstripItemImageUrl,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].image_url.clone())
                        }
                        (
                            InputMode::EditFilmstripItemImageAlt,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].image_alt.clone())
                        }
                        (
                            InputMode::EditFilmstripItemTitle,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].title.clone())
                        }
                        (
                            InputMode::EditMilestonesDataAos,
                            crate::model::SectionComponent::Milestones(v),
                        ) => Some(hero_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditMilestonesWidth,
                            crate::model::SectionComponent::Milestones(v),
                        ) => Some(v.parent_width.clone()),
                        (
                            InputMode::EditMilestonesItemPercentage,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_percentage.clone())
                        }
                        (
                            InputMode::EditMilestonesItemTitle,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditMilestonesItemSubtitle,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_subtitle.clone())
                        }
                        (
                            InputMode::EditMilestonesItemCopy,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        (
                            InputMode::EditMilestonesItemLinkUrl,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_url.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditMilestonesItemLinkTarget,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(
                                card_link_target_to_str(
                                    v.items[ni]
                                        .child_link_target
                                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget),
                                )
                                .to_string(),
                            )
                        }
                        (
                            InputMode::EditMilestonesItemLinkLabel,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_label.clone().unwrap_or_default())
                        }
                        (InputMode::EditCardType, crate::model::SectionComponent::Card(v)) => {
                            Some(card_type_to_str(v.card_type).to_string())
                        }
                        (InputMode::EditCardDataAos, crate::model::SectionComponent::Card(v)) => {
                            Some(hero_aos_to_str(v.card_data_aos).to_string())
                        }
                        (InputMode::EditCardWidth, crate::model::SectionComponent::Card(v)) => {
                            Some(v.card_width.clone())
                        }
                        (
                            InputMode::EditCardItemImageUrl,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_image_url.clone())
                        }
                        (
                            InputMode::EditCardItemImageAlt,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_image_alt.clone())
                        }
                        (InputMode::EditCardItemTitle, crate::model::SectionComponent::Card(v)) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_title.clone())
                        }
                        (
                            InputMode::EditCardItemSubtitle,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_subtitle.clone())
                        }
                        (InputMode::EditCardItemCopy, crate::model::SectionComponent::Card(v)) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_copy.clone())
                        }
                        (
                            InputMode::EditCardItemLinkUrl,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_link_url.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditCardItemLinkTarget,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(
                                card_link_target_to_str(
                                    v.items[ni]
                                        .card_link_target
                                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget),
                                )
                                .to_string(),
                            )
                        }
                        (
                            InputMode::EditCardItemLinkLabel,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].card_link_label.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditBlockquoteDataAos,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(hero_aos_to_str(v.blockquote_data_aos).to_string()),
                        (
                            InputMode::EditBlockquoteImageUrl,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.blockquote_image_url.clone()),
                        (
                            InputMode::EditBlockquoteImageAlt,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.blockquote_image_alt.clone()),
                        (
                            InputMode::EditBlockquotePersonsName,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.blockquote_persons_name.clone()),
                        (
                            InputMode::EditBlockquotePersonsTitle,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.blockquote_persons_title.clone()),
                        (
                            InputMode::EditBlockquoteCopy,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.blockquote_copy.clone()),
                        (
                            InputMode::EditAccordionType,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(accordion_type_to_str(v.accordion_type).to_string()),
                        (
                            InputMode::EditAccordionClass,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(accordion_class_to_str(v.accordion_class).to_string()),
                        (
                            InputMode::EditAccordionAos,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(hero_aos_to_str(v.accordion_aos).to_string()),
                        (
                            InputMode::EditAccordionGroupName,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(v.group_name.clone()),
                        (
                            InputMode::EditAccordionFirstTitle,
                            crate::model::SectionComponent::Accordion(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].title.clone())
                        }
                        (
                            InputMode::EditAccordionFirstContent,
                            crate::model::SectionComponent::Accordion(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].content.clone())
                        }
                        _ => None,
                    }
                }
            },
        }
    }

    fn selected_component_owned(&self) -> Option<crate::model::SectionComponent> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        let PageNode::Section(section) = &page.nodes[ni] else {
            return None;
        };
        let columns = section_columns_ref(section);
        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
        let ci = component_index(columns[col_i].components.len(), self.selected_component)?;
        columns[col_i].components.get(ci).cloned()
    }

    fn select_prev(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            return;
        }
        let next = self.selected_tree_row.saturating_sub(1);
        if next != self.selected_tree_row {
            self.selected_tree_row = next;
            self.apply_tree_row_selection(rows[next]);
        }
    }

    fn select_next(&mut self) {
        let rows = self.build_node_tree_rows();
        let total = rows.len();
        if total == 0 {
            return;
        }
        let next = (self.selected_tree_row + 1).min(total - 1);
        if next != self.selected_tree_row {
            self.selected_tree_row = next;
            self.apply_tree_row_selection(rows[next]);
        }
    }

    fn select_next_page(&mut self) {
        if self.site.pages.is_empty() {
            return;
        }
        self.selected_page = (self.selected_page + 1) % self.site.pages.len();
        self.selected_node = 0;
        self.selected_tree_row = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.details_scroll_row = 0;
        self.sync_tree_row_with_selection();
    }

    fn select_prev_page(&mut self) {
        if self.site.pages.is_empty() {
            return;
        }
        if self.selected_page == 0 {
            self.selected_page = self.site.pages.len() - 1;
        } else {
            self.selected_page -= 1;
        }
        self.selected_node = 0;
        self.selected_tree_row = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.details_scroll_row = 0;
        self.sync_tree_row_with_selection();
    }

    fn details_text(&self, detail_width: usize) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "No nodes on this page.".to_string();
        }
        let mut out = Vec::new();
        out.push(format!("Page blueprint: {}", page.title));
        out.push(String::new());
        for (idx, node) in page.nodes.iter().enumerate() {
            let marker = if idx == self.selected_node { "*" } else { " " };
            match node {
                PageNode::Hero(v) => {
                    out.push(format!("{marker}[{:02}] dd-hero", idx + 1,));
                    out.push(hero_ascii_map(v, detail_width));
                }
                PageNode::Section(v) => {
                    out.push(format!("{marker}[{:02}] dd-section {}", idx + 1, v.id));
                    out.push(section_ascii_map(
                        v,
                        if idx == self.selected_node {
                            self.selected_column
                        } else {
                            0
                        },
                        detail_width,
                    ));
                }
            }
            out.push(String::new());
        }
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.selection_summary(),
            self.component_kind.label()
        ));
        out.join("\n")
    }

    fn add_hero(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let hero = crate::model::DdHero {
            image: "/assets/images/hero-new.jpg".to_string(),
            hero_class: Some(crate::model::HeroImageClass::FullFull),
            hero_aos: Some(crate::model::HeroAos::FadeIn),
            custom_css: None,
            title: "New Hero".to_string(),
            subtitle: "Add subtitle".to_string(),
            copy: None,
            cta_text: None,
            cta_link: None,
            cta_target: Some(crate::model::CtaTarget::SelfTarget),
            cta_text_2: None,
            cta_link_2: None,
            cta_target_2: Some(crate::model::CtaTarget::SelfTarget),
            image_alt: Some("Hero image".to_string()),
            image_mobile: None,
            image_tablet: None,
            image_desktop: None,
            image_class: Some(crate::model::HeroImageClass::FullFull),
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Hero(hero));
        self.selected_node = idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Inserted dd-hero at position {}.", idx + 1);
    }

    fn add_section(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let next_id = next_section_id_for_page(page);
        let section = crate::model::DdSection {
            id: next_id,
            section_title: None,
            section_class: Some(crate::model::SectionClass::FullContained),
            item_box_class: Some(crate::model::SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: Vec::new(),
            }],
            components: Vec::new(),
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Section(section));
        self.selected_node = idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Inserted dd-section at position {}.", idx + 1);
    }

    fn delete_selected_node(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No node to delete.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        page.nodes.remove(idx);
        if page.nodes.is_empty() {
            self.selected_node = 0;
            self.selected_column = 0;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        } else {
            self.selected_node = idx.min(page.nodes.len() - 1);
            self.selected_column = 0;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.status = format!("Deleted node {}.", idx + 1);
    }

    fn move_selected_up(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.len() < 2 {
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        if idx == 0 {
            return;
        }
        page.nodes.swap(idx, idx - 1);
        self.selected_node = idx - 1;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = "Moved node up.".to_string();
    }

    fn move_selected_down(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.len() < 2 {
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        if idx + 1 >= page.nodes.len() {
            return;
        }
        page.nodes.swap(idx, idx + 1);
        self.selected_node = idx + 1;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = "Moved node down.".to_string();
    }

    fn add_selected_component_to_section(&mut self) {
        let kind = self.component_kind;
        if matches!(kind, ComponentKind::Hero | ComponentKind::Section) {
            self.status = "dd-hero and dd-section are top-level insert types.".to_string();
            return;
        }
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                let inserted = kind.default_component();
                components.push(inserted);
                (
                    Some(components.len().saturating_sub(1)),
                    format!(
                        "Added {} to selected section column '{}'.",
                        kind.label(),
                        section.columns[col_i].id
                    ),
                )
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(new_selected_component) = result.0 {
            self.selected_component = new_selected_component;
            self.selected_nested_item = 0;
        }
        self.status = result.1;
    }

    fn cycle_hero_class(&mut self, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = hero
                    .hero_class
                    .unwrap_or(crate::model::HeroImageClass::FullFull);
                let next = next_hero_image_class(current, forward);
                hero.hero_class = Some(next);
                self.status = format!("Hero default class: {}", hero_image_class_to_str(next));
            }
            _ => {
                self.status =
                    "Left/Right hero class cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_hero_aos(&mut self, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = hero.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn);
                let next = next_hero_aos(current, forward);
                hero.hero_aos = Some(next);
                self.status = format!("Hero data-aos: {}", hero_aos_to_str(next));
            }
            _ => {
                self.status =
                    "Left/Right hero data-aos cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_hero_cta_target(&mut self, secondary: bool, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = if secondary {
                    hero.cta_target_2
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                } else {
                    hero.cta_target
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                };
                let next = next_hero_cta_target(current, forward);
                if secondary {
                    hero.cta_target_2 = Some(next);
                } else {
                    hero.cta_target = Some(next);
                }
                self.status = if secondary {
                    format!("Hero link_2 target: {}", cta_target_to_str(next))
                } else {
                    format!("Hero link_1 target: {}", cta_target_to_str(next))
                };
            }
            _ => {
                self.status =
                    "Left/Right hero target cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_section_class(&mut self, forward: bool) {
        self.mutate_selected_section(
            |s| {
                let current = s
                    .section_class
                    .unwrap_or(crate::model::SectionClass::FullContained);
                s.section_class = Some(next_section_class(current, forward));
            },
            "Cycled section class.",
        );
    }

    fn cycle_banner_class(&mut self, forward: bool) {
        self.mutate_selected_banner(
            |b| {
                b.banner_class = next_banner_class(b.banner_class, forward);
            },
            "Cycled dd-banner class.",
        );
    }

    fn cycle_banner_data_aos(&mut self, forward: bool) {
        self.mutate_selected_banner(
            |b| {
                b.banner_data_aos = next_hero_aos(b.banner_data_aos, forward);
            },
            "Cycled dd-banner data-aos.",
        );
    }

    fn cycle_blockquote_data_aos(&mut self, forward: bool) {
        self.mutate_selected_blockquote(
            |b| {
                b.blockquote_data_aos = next_hero_aos(b.blockquote_data_aos, forward);
            },
            "Cycled dd-blockquote data-aos.",
        );
    }

    fn cycle_card_type(&mut self, forward: bool) {
        self.mutate_selected_card(
            |c| {
                c.card_type = next_card_type(c.card_type, forward);
            },
            "Cycled dd-card type.",
        );
    }

    fn cycle_card_data_aos(&mut self, forward: bool) {
        self.mutate_selected_card(
            |c| {
                c.card_data_aos = next_hero_aos(c.card_data_aos, forward);
            },
            "Cycled dd-card data-aos.",
        );
    }

    fn cycle_card_link_target(&mut self, forward: bool) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        if let Some(item_i) = nested_index(card.items.len(), selected_nested_item) {
                            let current = card.items[item_i]
                                .card_link_target
                                .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                            let next = next_card_link_target(current, forward);
                            card.items[item_i].card_link_target = Some(next);
                            format!(
                                "dd-card item {} link target: {}",
                                item_i + 1,
                                card_link_target_to_str(next)
                            )
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_filmstrip_type(&mut self, forward: bool) {
        self.mutate_selected_filmstrip(
            |f| {
                f.filmstrip_type = next_filmstrip_type(f.filmstrip_type, forward);
            },
            "Cycled dd-filmstrip type.",
        );
    }

    fn cycle_filmstrip_data_aos(&mut self, forward: bool) {
        self.mutate_selected_filmstrip(
            |f| {
                f.filmstrip_data_aos = next_hero_aos(f.filmstrip_data_aos, forward);
            },
            "Cycled dd-filmstrip data-aos.",
        );
    }

    fn cycle_milestones_data_aos(&mut self, forward: bool) {
        self.mutate_selected_milestones(
            |m| {
                m.parent_data_aos = next_hero_aos(m.parent_data_aos, forward);
            },
            "Cycled dd-milestones parent_data_aos.",
        );
    }

    fn cycle_milestones_link_target(&mut self, forward: bool) {
        let selected_nested_item = self.selected_nested_item;
        self.mutate_selected_milestones(
            |m| {
                if let Some(ni) = nested_index(m.items.len(), selected_nested_item) {
                    let current = m.items[ni]
                        .child_link_target
                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                    m.items[ni].child_link_target = Some(next_card_link_target(current, forward));
                }
            },
            "Cycled dd-milestones child_link_target.",
        );
    }

    fn cycle_alternating_type(&mut self, forward: bool) {
        self.mutate_selected_alternating(
            |a| {
                a.alternating_type = next_alternating_type(a.alternating_type, forward);
            },
            "Cycled dd-alternating type.",
        );
    }

    fn cycle_alternating_data_aos(&mut self, forward: bool) {
        self.mutate_selected_alternating(
            |a| {
                a.alternating_data_aos = next_hero_aos(a.alternating_data_aos, forward);
            },
            "Cycled dd-alternating data-aos.",
        );
    }

    fn mutate_selected_alternating<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdAlternating),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut components[ci] {
                        mutator(alt);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_banner<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdBanner),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut components[ci] {
                        mutator(banner);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_cta_class(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                cta.cta_class = next_cta_class(cta.cta_class, forward);
            },
            "Cycled dd-cta class.",
        );
    }

    fn cycle_cta_data_aos(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                cta.cta_data_aos = next_hero_aos(cta.cta_data_aos, forward);
            },
            "Cycled dd-cta data-aos.",
        );
    }

    fn cycle_cta_link_target(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                let current = cta
                    .cta_link_target
                    .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                cta.cta_link_target = Some(next_card_link_target(current, forward));
            },
            "Cycled dd-cta link target.",
        );
    }

    fn mutate_selected_cta<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdCta),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut components[ci] {
                        mutator(cta);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_blockquote<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdBlockquote),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut components[ci]
                    {
                        mutator(blockquote);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_card<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdCard),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        mutator(card);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_filmstrip<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdFilmstrip),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut components[ci]
                    {
                        mutator(filmstrip);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_milestones<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdMilestones),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut components[ci]
                    {
                        mutator(milestones);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_accordion_type(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.accordion_type = next_accordion_type(a.accordion_type, forward);
            },
            "Cycled dd-accordion type.",
        );
    }

    fn cycle_accordion_class(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.accordion_class = next_accordion_class(a.accordion_class, forward);
            },
            "Cycled dd-accordion class.",
        );
    }

    fn cycle_accordion_aos(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.accordion_aos = next_hero_aos(a.accordion_aos, forward);
            },
            "Cycled dd-accordion data-aos.",
        );
    }

    fn mutate_selected_accordion<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdAccordion),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(accordion) =
                        &mut components[ci]
                    {
                        mutator(accordion);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn add_selected_collection_item(&mut self) {
        let component = self.selected_component_owned();
        match component {
            Some(crate::model::SectionComponent::Accordion(_)) => {
                self.add_selected_accordion_item()
            }
            Some(crate::model::SectionComponent::Alternating(_)) => {
                self.add_selected_alternating_item()
            }
            Some(crate::model::SectionComponent::Card(_)) => self.add_selected_card_item(),
            Some(crate::model::SectionComponent::Filmstrip(_)) => {
                self.add_selected_filmstrip_item()
            }
            Some(crate::model::SectionComponent::Milestones(_)) => {
                self.add_selected_milestones_item()
            }
            Some(_) => {
                self.status = "Selected component does not support collection items.".to_string();
            }
            None => {
                self.status = "No selected collection component.".to_string();
            }
        }
    }

    fn remove_selected_collection_item(&mut self) {
        let component = self.selected_component_owned();
        match component {
            Some(crate::model::SectionComponent::Accordion(_)) => {
                self.remove_selected_accordion_item()
            }
            Some(crate::model::SectionComponent::Alternating(_)) => {
                self.remove_selected_alternating_item()
            }
            Some(crate::model::SectionComponent::Card(_)) => self.remove_selected_card_item(),
            Some(crate::model::SectionComponent::Filmstrip(_)) => {
                self.remove_selected_filmstrip_item()
            }
            Some(crate::model::SectionComponent::Milestones(_)) => {
                self.remove_selected_milestones_item()
            }
            Some(_) => {
                self.status = "Selected component does not support collection items.".to_string();
            }
            None => {
                self.status = "No selected collection component.".to_string();
            }
        }
    }

    fn add_selected_accordion_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            NodeTreeKind::AccordionItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(acc.items.len()))
                            .unwrap_or(acc.items.len());
                        let next_num = acc.items.len() + 1;
                        acc.items.insert(
                            insert_idx,
                            crate::model::AccordionItem {
                                title: format!("Accordion Item {}", next_num),
                                content: "Accordion content".to_string(),
                            },
                        );
                        (
                            Some((ni, col_i, ci, insert_idx)),
                            format!("Added accordion item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-accordion.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_accordion_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.status = result.1;
    }

    fn remove_selected_accordion_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut components[ci] {
                        if acc.items.len() <= 1 {
                            (
                                None,
                                "dd-accordion must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_idx = selected_nested_item.min(acc.items.len() - 1);
                            acc.items.remove(remove_idx);
                            let next_item_idx = remove_idx.min(acc.items.len() - 1);
                            (
                                Some((ni, col_i, ci, next_item_idx)),
                                format!("Removed accordion item {}.", remove_idx + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-accordion.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_accordion_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.status = result.1;
    }

    fn add_selected_alternating_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            NodeTreeKind::AlternatingItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(alt.items.len()))
                            .unwrap_or(alt.items.len());
                        let next_num = alt.items.len() + 1;
                        alt.items.insert(
                            insert_idx,
                            crate::model::AlternatingItem {
                                image: "https://dummyimage.com/600x400/000/fff".to_string(),
                                image_alt: format!("Alternating image {}", next_num),
                                title: format!("Alternating Item {}", next_num),
                                copy: "Alternating content".to_string(),
                            },
                        );
                        (
                            Some((ni, col_i, ci, insert_idx)),
                            format!("Added alternating item {}.", insert_idx + 1),
                        )
                    } else {
                        (
                            None,
                            "Selected component is not dd-alternating.".to_string(),
                        )
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_alternating_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.status = result.1;
    }

    fn remove_selected_alternating_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut components[ci] {
                        if alt.items.len() <= 1 {
                            (
                                None,
                                "dd-alternating must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_idx = selected_nested_item.min(alt.items.len() - 1);
                            alt.items.remove(remove_idx);
                            let next_item_idx = remove_idx.min(alt.items.len() - 1);
                            (
                                Some((ni, col_i, ci, next_item_idx)),
                                format!("Removed alternating item {}.", remove_idx + 1),
                            )
                        }
                    } else {
                        (
                            None,
                            "Selected component is not dd-alternating.".to_string(),
                        )
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_alternating_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.status = result.1;
    }

    fn add_selected_card_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            NodeTreeKind::CardItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(card.items.len()))
                            .unwrap_or(card.items.len());
                        let next_num = card.items.len() + 1;
                        card.items.insert(
                            insert_idx,
                            crate::model::CardItem {
                                card_image_url: "https://dummyimage.com/720x720/000/fff"
                                    .to_string(),
                                card_image_alt: "Image alt text".to_string(),
                                card_title: format!("Title {}", next_num),
                                card_subtitle: "Subtitle".to_string(),
                                card_copy: "Copy".to_string(),
                                card_link_url: Some("/front".to_string()),
                                card_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                                card_link_label: Some("Learn More".to_string()),
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-card item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-card.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_card_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn remove_selected_card_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            NodeTreeKind::CardItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        if card.items.len() <= 1 {
                            (None, "dd-card must keep at least one item.".to_string())
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(card.items.len().saturating_sub(1))
                            });
                            card.items.remove(remove_i);
                            let next_i = remove_i.min(card.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-card item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-card.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_card_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn add_selected_filmstrip_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            NodeTreeKind::FilmstripItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut components[ci]
                    {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(filmstrip.items.len()))
                            .unwrap_or(filmstrip.items.len());
                        let next_num = filmstrip.items.len() + 1;
                        filmstrip.items.insert(
                            insert_idx,
                            crate::model::FilmstripItem {
                                image_url: "https://dummyimage.com/256x256/000/fff".to_string(),
                                image_alt: "Image alt text".to_string(),
                                title: format!("Title {}", next_num),
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-filmstrip item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-filmstrip.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_filmstrip_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn remove_selected_filmstrip_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            NodeTreeKind::FilmstripItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut components[ci]
                    {
                        if filmstrip.items.len() <= 1 {
                            (
                                None,
                                "dd-filmstrip must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(filmstrip.items.len().saturating_sub(1))
                            });
                            filmstrip.items.remove(remove_i);
                            let next_i = remove_i.min(filmstrip.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-filmstrip item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-filmstrip.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_filmstrip_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn add_selected_milestones_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            NodeTreeKind::MilestonesItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut components[ci]
                    {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(milestones.items.len()))
                            .unwrap_or(milestones.items.len());
                        let next_num = milestones.items.len() + 1;
                        milestones.items.insert(
                            insert_idx,
                            crate::model::MilestonesItem {
                                child_percentage: "70".to_string(),
                                child_title: format!("Title {}", next_num),
                                child_subtitle: "Subtitle".to_string(),
                                child_copy: "Copy".to_string(),
                                child_link_url: None,
                                child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                                child_link_label: None,
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-milestones item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-milestones.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_milestones_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn remove_selected_milestones_item(&mut self) {
        let rows = self.build_node_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            NodeTreeKind::MilestonesItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut components[ci]
                    {
                        if milestones.items.len() <= 1 {
                            (
                                None,
                                "dd-milestones must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(milestones.items.len().saturating_sub(1))
                            });
                            milestones.items.remove(remove_i);
                            let next_i = remove_i.min(milestones.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-milestones item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-milestones.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_milestones_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.status = result.1;
    }

    fn mutate_selected_section<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdSection),
    {
        let prev_selected_component = self.selected_component;
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                mutator(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let next_selected_component = prev_selected_component
                    .min(section.columns[col_i].components.len().saturating_sub(1));
                (Some(next_selected_component), success_message.to_string())
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_component) = result.0 {
            self.selected_component = next_selected_component;
        }
        self.status = result.1;
    }

    fn add_column(&mut self) {
        self.mutate_selected_section(
            |section| {
                normalize_section_columns(section);
                let next = section.columns.len() + 1;
                section.columns.push(SectionColumn {
                    id: format!("column-{}", next),
                    width_class: "dd-u-1-1".to_string(),
                    components: Vec::new(),
                });
            },
            "Added column to section.",
        );
        if let Some(total) = self.selected_section_column_total() {
            if total > 0 {
                self.selected_column = total - 1;
            }
        }
        self.selected_component = 0;
        self.selected_nested_item = 0;
    }

    fn remove_selected_column(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() <= 1 {
                    (None, "Section must keep at least one column.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    section.columns.remove(ci);
                    (
                        Some(ci.min(section.columns.len() - 1)),
                        "Removed selected column.".to_string(),
                    )
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.status = result.1;
    }

    fn select_prev_column(&mut self) {
        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no columns.".to_string();
            return;
        }
        self.selected_column = self.selected_column.saturating_sub(1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Selected column {} of {}.", self.selected_column + 1, total);
    }

    fn select_next_column(&mut self) {
        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no columns.".to_string();
            return;
        }
        self.selected_column = (self.selected_column + 1).min(total - 1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Selected column {} of {}.", self.selected_column + 1, total);
    }

    fn move_selected_column_up(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() < 2 {
                    (None, "Need at least 2 columns.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    if ci == 0 {
                        (None, "Column is already first.".to_string())
                    } else {
                        section.columns.swap(ci, ci - 1);
                        (Some(ci - 1), "Moved column up.".to_string())
                    }
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.status = result.1;
    }

    fn move_selected_column_down(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() < 2 {
                    (None, "Need at least 2 columns.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    if ci + 1 >= section.columns.len() {
                        (None, "Column is already last.".to_string())
                    } else {
                        section.columns.swap(ci, ci + 1);
                        (Some(ci + 1), "Moved column down.".to_string())
                    }
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.status = result.1;
    }

    fn begin_edit_selected_component_primary(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let ni = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[ni] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        let components = &columns[col_i].components;
                        if let Some(ci) = component_index(components.len(), self.selected_component)
                        {
                            match &components[ci] {
                                crate::model::SectionComponent::Banner(banner) => Some((
                                    InputMode::EditBannerClass,
                                    banner_class_to_str(banner.banner_class).to_string(),
                                )),
                                crate::model::SectionComponent::Cta(cta) => Some((
                                    InputMode::EditCtaClass,
                                    cta_class_to_str(cta.cta_class).to_string(),
                                )),
                                crate::model::SectionComponent::Filmstrip(filmstrip) => Some((
                                    InputMode::EditFilmstripType,
                                    filmstrip_type_to_str(filmstrip.filmstrip_type).to_string(),
                                )),
                                crate::model::SectionComponent::Milestones(milestones) => Some((
                                    InputMode::EditMilestonesDataAos,
                                    hero_aos_to_str(milestones.parent_data_aos).to_string(),
                                )),
                                crate::model::SectionComponent::Card(card) => Some((
                                    InputMode::EditCardType,
                                    card_type_to_str(card.card_type).to_string(),
                                )),
                                crate::model::SectionComponent::Accordion(acc) => Some((
                                    InputMode::EditAccordionType,
                                    accordion_type_to_str(acc.accordion_type).to_string(),
                                )),
                                crate::model::SectionComponent::Blockquote(v) => Some((
                                    InputMode::EditBlockquoteDataAos,
                                    hero_aos_to_str(v.blockquote_data_aos).to_string(),
                                )),
                                crate::model::SectionComponent::Alternating(alt) => Some((
                                    InputMode::EditAlternatingType,
                                    alternating_type_to_str(alt.alternating_type).to_string(),
                                )),
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        };

        let Some((mode, value)) = selected else {
            self.status =
                "Primary edit supports cta/filmstrip/milestones/banner/card/blockquote/accordion/alternating."
                    .to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status = match mode {
            InputMode::EditBannerClass => {
                "Editing dd-banner class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaClass => {
                "Editing dd-cta class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripType => {
                "Editing dd-filmstrip type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesDataAos => {
                "Editing dd-milestones parent_data_aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionType => {
                "Editing dd-accordion type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionClass => {
                "Editing dd-accordion class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionAos => {
                "Editing dd-accordion data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionGroupName => {
                "Editing dd-accordion group name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstTitle => {
                "Editing dd-accordion first title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteDataAos => {
                "Editing dd-blockquote data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardType => {
                "Editing dd-card type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingType => {
                "Editing dd-alternating type. Enter to save, esc to cancel.".to_string()
            }
            _ => "Editing component value.".to_string(),
        };
    }

    fn selected_section_column_total(&self) -> Option<usize> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[ni] {
            PageNode::Hero(_) => None,
            PageNode::Section(section) => Some(section_columns_ref(section).len()),
        }
    }

    fn details_max_scroll(&self) -> usize {
        let visible_rows = self.details_area.height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return 0;
        }
        let detail_width = self.details_area.width.saturating_sub(2) as usize;
        if detail_width == 0 {
            return 0;
        }
        let total_rows = self.details_text(detail_width).lines().count().max(1);
        total_rows.saturating_sub(visible_rows)
    }

    fn scroll_details_by(&mut self, delta: isize) {
        let max_scroll = self.details_max_scroll() as isize;
        let next = self.details_scroll_row as isize + delta;
        self.details_scroll_row = next.clamp(0, max_scroll) as usize;
    }
}

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn component_index(total: usize, selected_component: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_component.min(total - 1))
    }
}

fn section_columns_ref(section: &crate::model::DdSection) -> Vec<SectionColumn> {
    if !section.columns.is_empty() {
        section.columns.clone()
    } else {
        vec![SectionColumn {
            id: format!("{}-legacy-column", section.id),
            width_class: "dd-u-1-1".to_string(),
            components: section.components.clone(),
        }]
    }
}

fn normalize_section_columns(section: &mut crate::model::DdSection) {
    if section.columns.is_empty() {
        let legacy = std::mem::take(&mut section.components);
        section.columns.push(SectionColumn {
            id: "column-1".to_string(),
            width_class: "dd-u-1-1".to_string(),
            components: legacy,
        });
    }
}

fn pull_selected_column_into_legacy_components(
    section: &mut crate::model::DdSection,
    selected_column: usize,
) {
    normalize_section_columns(section);
    let col_i = selected_column.min(section.columns.len().saturating_sub(1));
    section.components = section.columns[col_i].components.clone();
}

fn push_legacy_components_into_selected_column(
    section: &mut crate::model::DdSection,
    selected_column: usize,
) {
    normalize_section_columns(section);
    let col_i = selected_column.min(section.columns.len().saturating_sub(1));
    section.columns[col_i].components = section.components.clone();
}

fn nested_index(total: usize, selected_nested_item: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_nested_item.min(total - 1))
    }
}

fn section_ascii_map(
    section: &crate::model::DdSection,
    selected_column: usize,
    panel_width: usize,
) -> String {
    const MAX_COMPONENT_ROWS: usize = 4;

    let inner_width = panel_width.saturating_sub(4).max(12);
    let columns = section_columns_ref(section);
    if columns.is_empty() {
        return "(no columns)".to_string();
    }
    let active = selected_column.min(columns.len().saturating_sub(1));

    let mut lines = vec![
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

    let item_boxes = columns
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
            if col.components.is_empty() {
                box_lines.push(format!(
                    "| {} |",
                    fit_ascii_cell("(empty)", item_inner_width)
                ));
            } else {
                for component in col.components.iter().take(MAX_COMPONENT_ROWS) {
                    match component {
                        crate::model::SectionComponent::Card(card) => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell("- dd-card", item_inner_width)
                            ));
                            for line in card_items_ascii_lines(card, item_inner_width) {
                                box_lines.push(format!(
                                    "| {} |",
                                    fit_ascii_cell(&line, item_inner_width)
                                ));
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
                        }
                    }
                }
                let more = col.components.len().saturating_sub(MAX_COMPONENT_ROWS);
                if more > 0 {
                    box_lines.push(format!(
                        "| {} |",
                        fit_ascii_cell(&format!("+{more} more"), item_inner_width)
                    ));
                }
            }
            box_lines.push(item_border);
            box_lines
        })
        .collect::<Vec<_>>();

    let item_box_widths = item_boxes
        .iter()
        .map(|item| item.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 2usize;
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
            lines.push(fit_ascii_cell("", inner_width));
        }
        let max_height = row
            .iter()
            .map(|idx| item_boxes[*idx].len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..max_height {
            let mut composed = String::new();
            for (pos, idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str("  ");
                }
                let box_lines = &item_boxes[*idx];
                let box_width = item_box_widths[*idx];
                let part = box_lines
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_width));
                composed.push_str(&part);
            }
            lines.push(fit_ascii_cell(&composed, inner_width));
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

fn card_items_ascii_lines(
    card: &crate::model::DdCard,
    container_inner_width: usize,
) -> Vec<String> {
    if card.items.is_empty() {
        return vec![fit_ascii_cell("(empty)", container_inner_width)];
    }

    let child_inner_width = section_item_ascii_inner_width(&card.card_width, container_inner_width)
        .min(container_inner_width.saturating_sub(6))
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
                    fit_ascii_cell(&format!("title: {}", item.card_title), child_inner_width)
                ),
                child_border.clone(),
            ]
        })
        .collect::<Vec<_>>();

    let box_widths = child_boxes
        .iter()
        .map(|b| b.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 2usize;
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

fn section_item_ascii_inner_width(width_class: &str, section_inner_width: usize) -> usize {
    let min_inner = 12usize;
    let max_inner = section_inner_width.saturating_sub(10).max(min_inner);
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

fn resolve_dd_u_ratio_for_panel(width_class: &str, panel_chars: usize) -> Option<(usize, usize)> {
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

fn parse_dd_u_token_ratio(token: &str) -> Option<(ResponsiveBp, (usize, usize))> {
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
enum ResponsiveBp {
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

fn breakpoint_for_panel_chars(panel_chars: usize) -> ResponsiveBp {
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

fn hero_ascii_map(hero: &crate::model::DdHero, panel_width: usize) -> String {
    let inner_width = panel_width.saturating_sub(4).max(8);
    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let lines = [
        fit_ascii_cell("HERO", inner_width),
        fit_ascii_cell(
            &format!(
                "class: {}",
                hero_image_class_to_str(
                    hero.hero_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "aos: {}",
                hero_aos_to_str(hero.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn))
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                hero.custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("title: {}", hero.title), inner_width),
        fit_ascii_cell(&format!("subtitle: {}", hero.subtitle), inner_width),
        fit_ascii_cell(
            &format!(
                "cta: {} -> {}",
                hero.cta_text.as_deref().unwrap_or("(none)"),
                hero.cta_link.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "cta_2: {} -> {}",
                hero.cta_text_2.as_deref().unwrap_or("(none)"),
                hero.cta_link_2.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("image: {}", hero.image), inner_width),
    ];
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

fn section_class_to_str(v: crate::model::SectionClass) -> &'static str {
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

fn parse_section_class(raw: &str) -> Option<crate::model::SectionClass> {
    match raw.trim() {
        "-contained" => Some(crate::model::SectionClass::Contained),
        "-contained-md" => Some(crate::model::SectionClass::ContainedMd),
        "-contained-lg" => Some(crate::model::SectionClass::ContainedLg),
        "-contained-xl" => Some(crate::model::SectionClass::ContainedXl),
        "-contained-xxl" => Some(crate::model::SectionClass::ContainedXxl),
        "-full-full" => Some(crate::model::SectionClass::FullFull),
        "-full-contained" => Some(crate::model::SectionClass::FullContained),
        "-full-contained-md" => Some(crate::model::SectionClass::FullContainedMd),
        "-full-contained-lg" => Some(crate::model::SectionClass::FullContainedLg),
        "-full-contained-xl" => Some(crate::model::SectionClass::FullContainedXl),
        "-full-contained-xxl" => Some(crate::model::SectionClass::FullContainedXxl),
        _ => None,
    }
}

fn banner_class_to_str(v: crate::model::BannerClass) -> &'static str {
    match v {
        crate::model::BannerClass::BgTopLeft => "-bg-top-left",
        crate::model::BannerClass::BgTopCenter => "-bg-top-center",
        crate::model::BannerClass::BgTopRight => "-bg-top-right",
        crate::model::BannerClass::BgCenterLeft => "-bg-center-left",
        crate::model::BannerClass::BgCenterCenter => "-bg-center-center",
        crate::model::BannerClass::BgCenterRight => "-bg-center-right",
        crate::model::BannerClass::BgBottomLeft => "-bg-bottom-left",
        crate::model::BannerClass::BgBottomCenter => "-bg-bottom-center",
        crate::model::BannerClass::BgBottomRight => "-bg-bottom-right",
    }
}

fn cta_class_to_str(v: crate::model::CtaClass) -> &'static str {
    match v {
        crate::model::CtaClass::TopLeft => "-top-left",
        crate::model::CtaClass::TopCenter => "-top-center",
        crate::model::CtaClass::TopRight => "-top-right",
        crate::model::CtaClass::CenterLeft => "-center-left",
        crate::model::CtaClass::CenterCenter => "-center-center",
        crate::model::CtaClass::CenterRight => "-center-right",
        crate::model::CtaClass::BottomLeft => "-bottom-left",
        crate::model::CtaClass::BottomCenter => "-bottom-center",
        crate::model::CtaClass::BottomRight => "-bottom-right",
    }
}

fn parse_cta_class(raw: &str) -> Option<crate::model::CtaClass> {
    match raw.trim() {
        "-top-left" => Some(crate::model::CtaClass::TopLeft),
        "-top-center" => Some(crate::model::CtaClass::TopCenter),
        "-top-right" => Some(crate::model::CtaClass::TopRight),
        "-center-left" => Some(crate::model::CtaClass::CenterLeft),
        "-center-center" => Some(crate::model::CtaClass::CenterCenter),
        "-center-right" => Some(crate::model::CtaClass::CenterRight),
        "-bottom-left" => Some(crate::model::CtaClass::BottomLeft),
        "-bottom-center" => Some(crate::model::CtaClass::BottomCenter),
        "-bottom-right" => Some(crate::model::CtaClass::BottomRight),
        _ => None,
    }
}

fn next_cta_class(current: crate::model::CtaClass, forward: bool) -> crate::model::CtaClass {
    use crate::model::CtaClass;
    let all = [
        CtaClass::TopLeft,
        CtaClass::TopCenter,
        CtaClass::TopRight,
        CtaClass::CenterLeft,
        CtaClass::CenterCenter,
        CtaClass::CenterRight,
        CtaClass::BottomLeft,
        CtaClass::BottomCenter,
        CtaClass::BottomRight,
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

fn parse_banner_class(raw: &str) -> Option<crate::model::BannerClass> {
    match raw.trim() {
        "-bg-top-left" => Some(crate::model::BannerClass::BgTopLeft),
        "-bg-top-center" => Some(crate::model::BannerClass::BgTopCenter),
        "-bg-top-right" => Some(crate::model::BannerClass::BgTopRight),
        "-bg-center-left" => Some(crate::model::BannerClass::BgCenterLeft),
        "-bg-center-center" => Some(crate::model::BannerClass::BgCenterCenter),
        "-bg-center-right" => Some(crate::model::BannerClass::BgCenterRight),
        "-bg-bottom-left" => Some(crate::model::BannerClass::BgBottomLeft),
        "-bg-bottom-center" => Some(crate::model::BannerClass::BgBottomCenter),
        "-bg-bottom-right" => Some(crate::model::BannerClass::BgBottomRight),
        _ => None,
    }
}

fn next_banner_class(
    current: crate::model::BannerClass,
    forward: bool,
) -> crate::model::BannerClass {
    use crate::model::BannerClass;
    let all = [
        BannerClass::BgTopLeft,
        BannerClass::BgTopCenter,
        BannerClass::BgTopRight,
        BannerClass::BgCenterLeft,
        BannerClass::BgCenterCenter,
        BannerClass::BgCenterRight,
        BannerClass::BgBottomLeft,
        BannerClass::BgBottomCenter,
        BannerClass::BgBottomRight,
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

fn card_type_to_str(v: crate::model::CardType) -> &'static str {
    match v {
        crate::model::CardType::Default => "-default",
        crate::model::CardType::Horizontal => "-horizontal",
    }
}

fn parse_card_type(raw: &str) -> Option<crate::model::CardType> {
    match raw.trim() {
        "-default" => Some(crate::model::CardType::Default),
        "-horizontal" => Some(crate::model::CardType::Horizontal),
        _ => None,
    }
}

fn next_card_type(current: crate::model::CardType, forward: bool) -> crate::model::CardType {
    use crate::model::CardType;
    let all = [CardType::Default, CardType::Horizontal];
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

fn filmstrip_type_to_str(v: crate::model::FilmstripType) -> &'static str {
    match v {
        crate::model::FilmstripType::Default => "-default",
        crate::model::FilmstripType::Reverse => "-reverse",
    }
}

fn parse_filmstrip_type(raw: &str) -> Option<crate::model::FilmstripType> {
    match raw.trim() {
        "-default" => Some(crate::model::FilmstripType::Default),
        "-reverse" => Some(crate::model::FilmstripType::Reverse),
        _ => None,
    }
}

fn next_filmstrip_type(
    current: crate::model::FilmstripType,
    forward: bool,
) -> crate::model::FilmstripType {
    use crate::model::FilmstripType;
    let all = [FilmstripType::Default, FilmstripType::Reverse];
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

fn card_link_target_to_str(v: crate::model::CardLinkTarget) -> &'static str {
    match v {
        crate::model::CardLinkTarget::SelfTarget => "_self",
        crate::model::CardLinkTarget::Blank => "_blank",
    }
}

fn parse_card_link_target(raw: &str) -> Option<crate::model::CardLinkTarget> {
    match raw.trim() {
        "_self" => Some(crate::model::CardLinkTarget::SelfTarget),
        "_blank" => Some(crate::model::CardLinkTarget::Blank),
        _ => None,
    }
}

fn next_card_link_target(
    current: crate::model::CardLinkTarget,
    forward: bool,
) -> crate::model::CardLinkTarget {
    use crate::model::CardLinkTarget;
    let all = [CardLinkTarget::SelfTarget, CardLinkTarget::Blank];
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

fn alternating_type_to_str(v: crate::model::AlternatingType) -> &'static str {
    match v {
        crate::model::AlternatingType::Default => "-default",
        crate::model::AlternatingType::Reverse => "-reverse",
        crate::model::AlternatingType::NoAlternate => "-no-alternate",
    }
}

fn parse_alternating_type(raw: &str) -> Option<crate::model::AlternatingType> {
    match raw.trim() {
        "-default" => Some(crate::model::AlternatingType::Default),
        "-reverse" => Some(crate::model::AlternatingType::Reverse),
        "-no-alternate" => Some(crate::model::AlternatingType::NoAlternate),
        _ => None,
    }
}

fn next_alternating_type(
    current: crate::model::AlternatingType,
    forward: bool,
) -> crate::model::AlternatingType {
    use crate::model::AlternatingType;
    let all = [
        AlternatingType::Default,
        AlternatingType::Reverse,
        AlternatingType::NoAlternate,
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

fn accordion_type_to_str(v: crate::model::AccordionType) -> &'static str {
    match v {
        crate::model::AccordionType::Default => "-default",
        crate::model::AccordionType::Faq => "-faq",
    }
}

fn parse_accordion_type(raw: &str) -> Option<crate::model::AccordionType> {
    match raw.trim() {
        "-default" => Some(crate::model::AccordionType::Default),
        "-faq" => Some(crate::model::AccordionType::Faq),
        _ => None,
    }
}

fn next_accordion_type(
    current: crate::model::AccordionType,
    forward: bool,
) -> crate::model::AccordionType {
    use crate::model::AccordionType;
    let all = [AccordionType::Default, AccordionType::Faq];
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

fn accordion_class_to_str(v: crate::model::AccordionClass) -> &'static str {
    match v {
        crate::model::AccordionClass::Borderless => "-borderless",
        crate::model::AccordionClass::Compact => "-compact",
        crate::model::AccordionClass::Primary => "-primary",
        crate::model::AccordionClass::Secondary => "-secondary",
        crate::model::AccordionClass::Tertiary => "-tertiary",
    }
}

fn parse_accordion_class(raw: &str) -> Option<crate::model::AccordionClass> {
    match raw.trim() {
        "-borderless" => Some(crate::model::AccordionClass::Borderless),
        "-compact" => Some(crate::model::AccordionClass::Compact),
        "-primary" => Some(crate::model::AccordionClass::Primary),
        "-secondary" => Some(crate::model::AccordionClass::Secondary),
        "-tertiary" => Some(crate::model::AccordionClass::Tertiary),
        _ => None,
    }
}

fn next_accordion_class(
    current: crate::model::AccordionClass,
    forward: bool,
) -> crate::model::AccordionClass {
    use crate::model::AccordionClass;
    let all = [
        AccordionClass::Borderless,
        AccordionClass::Compact,
        AccordionClass::Primary,
        AccordionClass::Secondary,
        AccordionClass::Tertiary,
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

fn next_section_class(
    current: crate::model::SectionClass,
    forward: bool,
) -> crate::model::SectionClass {
    use crate::model::SectionClass;
    let all = [
        SectionClass::Contained,
        SectionClass::ContainedMd,
        SectionClass::ContainedLg,
        SectionClass::ContainedXl,
        SectionClass::ContainedXxl,
        SectionClass::FullFull,
        SectionClass::FullContained,
        SectionClass::FullContainedMd,
        SectionClass::FullContainedLg,
        SectionClass::FullContainedXl,
        SectionClass::FullContainedXxl,
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

fn fit_ascii_cell(value: &str, width: usize) -> String {
    let shortened = truncate_ascii(value, width);
    format!("{shortened:<width$}")
}

fn input_lines_preserve(s: &str) -> Vec<String> {
    s.split('\n').map(|line| line.to_string()).collect()
}

fn cursor_from_row_col(lines: &[String], target_row: usize, target_col: usize) -> usize {
    let row = target_row.min(lines.len().saturating_sub(1));
    let mut cursor = 0usize;
    for line in lines.iter().take(row) {
        cursor += line.chars().count() + 1;
    }
    let line_len = lines.get(row).map(|line| line.chars().count()).unwrap_or(0);
    cursor + target_col.min(line_len)
}

fn byte_index_for_char(s: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len())
}

fn cursor_row_col(s: &str, cursor: usize) -> (usize, usize) {
    let mut row = 0usize;
    let mut col = 0usize;
    let mut idx = 0usize;
    for ch in s.chars() {
        if idx >= cursor {
            break;
        }
        if ch == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
        idx += 1;
    }
    (row, col)
}

fn truncate_ascii(value: &str, max_chars: usize) -> String {
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

fn component_label(component: &crate::model::SectionComponent) -> &'static str {
    match component {
        crate::model::SectionComponent::Cta(_) => "dd-cta",
        crate::model::SectionComponent::Filmstrip(_) => "dd-filmstrip",
        crate::model::SectionComponent::Milestones(_) => "dd-milestones",
        crate::model::SectionComponent::Banner(_) => "dd-banner",
        crate::model::SectionComponent::Card(_) => "dd-card",
        crate::model::SectionComponent::Blockquote(_) => "dd-blockquote",
        crate::model::SectionComponent::Accordion(_) => "dd-accordion",
        crate::model::SectionComponent::Alternating(_) => "dd-alternating",
    }
}

fn component_blueprint_label(component: &crate::model::SectionComponent) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => {
            format!("dd-cta | cta_title: {}", v.cta_title)
        }
        crate::model::SectionComponent::Filmstrip(v) => format!(
            "dd-filmstrip | child_title: {}",
            v.items
                .first()
                .map(|i| i.title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Milestones(v) => format!(
            "dd-milestones | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Accordion(v) => format!(
            "dd-accordion | accordion_title: {}",
            v.items
                .first()
                .map(|i| i.title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Alternating(v) => format!(
            "dd-alternating | alternating_title: {}",
            v.items
                .first()
                .map(|i| i.title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Card(v) => format!(
            "dd-card | card_title: {}",
            v.items
                .first()
                .map(|i| i.card_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Blockquote(v) => format!(
            "dd-blockquote | blockquote_persons_name: {} | blockquote_persons_title: {}",
            v.blockquote_persons_name, v.blockquote_persons_title
        ),
        _ => component_label(component).to_string(),
    }
}

fn component_form(
    component: &crate::model::SectionComponent,
    selected_nested_item: usize,
) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => format!(
            "fields:\n  cta.class: {}\n  cta_image_url: {}\n  cta_image_alt: {}\n  cta.data_aos: {}\n  cta_title: {}\n  cta_subtitle: {}\n  cta_copy: {}\n  cta_link_url: {}\n  cta_link_target: {}\n  cta_link_label: {}",
            cta_class_to_str(v.cta_class),
            v.cta_image_url,
            v.cta_image_alt,
            hero_aos_to_str(v.cta_data_aos),
            v.cta_title,
            v.cta_subtitle,
            v.cta_copy,
            v.cta_link_url.as_deref().unwrap_or("(none)"),
            v.cta_link_target
                .map(card_link_target_to_str)
                .unwrap_or("_self"),
            v.cta_link_label.as_deref().unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Filmstrip(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_type: {}\n  parent_data_aos: {}\n  active_item: {}\n  child_image_url: {}\n  child_image_alt: {}\n  child_title: {}",
                filmstrip_type_to_str(v.filmstrip_type),
                hero_aos_to_str(v.filmstrip_data_aos),
                active,
                item.map(|i| i.image_url.as_str()).unwrap_or("(none)"),
                item.map(|i| i.image_alt.as_str()).unwrap_or("(none)"),
                item.map(|i| i.title.as_str()).unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Milestones(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_data_aos: {}\n  parent_width: {}\n  active_item: {}\n  child_percentage: {}\n  child_title: {}\n  child_subtitle: {}\n  child_copy: {}\n  child_link_url: {}\n  child_link_target: {}\n  child_link_label: {}",
                hero_aos_to_str(v.parent_data_aos),
                v.parent_width,
                active,
                item.map(|i| i.child_percentage.as_str())
                    .unwrap_or("(none)"),
                item.map(|i| i.child_title.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_subtitle.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_copy.as_str()).unwrap_or("(none)"),
                item.and_then(|i| i.child_link_url.as_deref())
                    .unwrap_or("(none)"),
                item.and_then(|i| i.child_link_target)
                    .map(card_link_target_to_str)
                    .unwrap_or("_self"),
                item.and_then(|i| i.child_link_label.as_deref())
                    .unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Banner(v) => format!(
            "fields:\n  banner.class: {}\n  banner.data_aos: {}\n  banner_image_url: {}\n  banner_image_alt: {}",
            banner_class_to_str(v.banner_class),
            hero_aos_to_str(v.banner_data_aos),
            v.banner_image_url,
            v.banner_image_alt
        ),
        crate::model::SectionComponent::Accordion(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let title = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.title.as_str())
                .unwrap_or("(none)");
            let content = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.content.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  accordion_type: {}\n  accordion.class: {}\n  accordion.data_aos: {}\n  accordion.group_name: {}\n  active_item: {}\n  accordion_title: {}\n  accordion_copy: {}",
                accordion_type_to_str(v.accordion_type),
                accordion_class_to_str(v.accordion_class),
                hero_aos_to_str(v.accordion_aos),
                v.group_name,
                active,
                title,
                content
            )
        }
        crate::model::SectionComponent::Blockquote(v) => format!(
            "fields:\n  blockquote_data_aos: {}\n  blockquote_image_url: {}\n  blockquote_image_alt: {}\n  blockquote_persons_name: {}\n  blockquote_persons_title: {}\n  blockquote_copy: {}",
            hero_aos_to_str(v.blockquote_data_aos),
            v.blockquote_image_url,
            v.blockquote_image_alt,
            v.blockquote_persons_name,
            v.blockquote_persons_title,
            v.blockquote_copy
        ),
        crate::model::SectionComponent::Alternating(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let image = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.image.as_str())
                .unwrap_or("(none)");
            let image_alt = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.image_alt.as_str())
                .unwrap_or("(none)");
            let title = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.title.as_str())
                .unwrap_or("(none)");
            let copy = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.copy.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  alternating_type: {}\n  alternating.class: {}\n  alternating.data_aos: {}\n  active_item: {}\n  alternating_image: {}\n  alternating_image_alt: {}\n  alternating_title: {}\n  alternating_copy: {}",
                alternating_type_to_str(v.alternating_type),
                v.alternating_class,
                hero_aos_to_str(v.alternating_data_aos),
                active,
                image,
                image_alt,
                title,
                copy
            )
        }
        crate::model::SectionComponent::Card(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  card_type: {}\n  card_data_aos: {}\n  card_width: {}\n  active_item: {}\n  card_image_url: {}\n  card_image_alt: {}\n  card_title: {}\n  card_subtitle: {}\n  card_copy: {}\n  card_link_url: {}\n  card_link_target: {}\n  card_link_label: {}",
                card_type_to_str(v.card_type),
                hero_aos_to_str(v.card_data_aos),
                v.card_width,
                active,
                item.map(|i| i.card_image_url.as_str()).unwrap_or("(none)"),
                item.map(|i| i.card_image_alt.as_str()).unwrap_or("(none)"),
                item.map(|i| i.card_title.as_str()).unwrap_or("(none)"),
                item.map(|i| i.card_subtitle.as_str()).unwrap_or("(none)"),
                item.map(|i| i.card_copy.as_str()).unwrap_or("(none)"),
                item.and_then(|i| i.card_link_url.as_deref())
                    .unwrap_or("(none)"),
                item.and_then(|i| i.card_link_target)
                    .map(card_link_target_to_str)
                    .unwrap_or("_self"),
                item.and_then(|i| i.card_link_label.as_deref())
                    .unwrap_or("(none)")
            )
        }
    }
}

fn hero_image_class_to_str(v: crate::model::HeroImageClass) -> &'static str {
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

fn hero_aos_to_str(v: crate::model::HeroAos) -> &'static str {
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

fn cta_target_to_str(v: crate::model::CtaTarget) -> &'static str {
    match v {
        crate::model::CtaTarget::SelfTarget => "_self",
        crate::model::CtaTarget::Blank => "_blank",
        crate::model::CtaTarget::Parent => "_parent",
    }
}

fn parse_hero_image_class(raw: &str) -> Option<crate::model::HeroImageClass> {
    let trimmed = raw.trim();
    let normalized = trimmed.strip_prefix(".dd-hero__image.").unwrap_or(trimmed);
    match normalized {
        "-contained" => Some(crate::model::HeroImageClass::Contained),
        "-contained-md" => Some(crate::model::HeroImageClass::ContainedMd),
        "-contained-lg" => Some(crate::model::HeroImageClass::ContainedLg),
        "-contained-xl" => Some(crate::model::HeroImageClass::ContainedXl),
        "-contained-xxl" => Some(crate::model::HeroImageClass::ContainedXxl),
        "-full-full" => Some(crate::model::HeroImageClass::FullFull),
        "-full-contained" => Some(crate::model::HeroImageClass::FullContained),
        "-full-contained-md" => Some(crate::model::HeroImageClass::FullContainedMd),
        "-full-contained-lg" => Some(crate::model::HeroImageClass::FullContainedLg),
        "-full-contained-xl" => Some(crate::model::HeroImageClass::FullContainedXl),
        "-full-contained-xxl" => Some(crate::model::HeroImageClass::FullContainedXxl),
        _ => None,
    }
}

fn parse_hero_aos(raw: &str) -> Option<crate::model::HeroAos> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fade-in" => Some(crate::model::HeroAos::FadeIn),
        "fade-up" => Some(crate::model::HeroAos::FadeUp),
        "fade-right" => Some(crate::model::HeroAos::FadeRight),
        "fade-down" => Some(crate::model::HeroAos::FadeDown),
        "fade-left" => Some(crate::model::HeroAos::FadeLeft),
        "zoom-in" => Some(crate::model::HeroAos::ZoomIn),
        "zoom-in-up" => Some(crate::model::HeroAos::ZoomInUp),
        "zoom-in-down" => Some(crate::model::HeroAos::ZoomInDown),
        _ => None,
    }
}

fn parse_cta_target(raw: &str) -> Option<crate::model::CtaTarget> {
    match raw.trim() {
        "_self" => Some(crate::model::CtaTarget::SelfTarget),
        "_blank" => Some(crate::model::CtaTarget::Blank),
        _ => None,
    }
}

fn next_hero_image_class(
    current: crate::model::HeroImageClass,
    forward: bool,
) -> crate::model::HeroImageClass {
    use crate::model::HeroImageClass;
    let all = [
        HeroImageClass::Contained,
        HeroImageClass::ContainedMd,
        HeroImageClass::ContainedLg,
        HeroImageClass::ContainedXl,
        HeroImageClass::ContainedXxl,
        HeroImageClass::FullFull,
        HeroImageClass::FullContained,
        HeroImageClass::FullContainedMd,
        HeroImageClass::FullContainedLg,
        HeroImageClass::FullContainedXl,
        HeroImageClass::FullContainedXxl,
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

fn next_hero_aos(current: crate::model::HeroAos, forward: bool) -> crate::model::HeroAos {
    use crate::model::HeroAos;
    let all = [
        HeroAos::FadeIn,
        HeroAos::FadeUp,
        HeroAos::FadeRight,
        HeroAos::FadeDown,
        HeroAos::FadeLeft,
        HeroAos::ZoomIn,
        HeroAos::ZoomInUp,
        HeroAos::ZoomInDown,
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

fn next_hero_cta_target(
    current: crate::model::CtaTarget,
    forward: bool,
) -> crate::model::CtaTarget {
    use crate::model::CtaTarget;
    let all = [CtaTarget::SelfTarget, CtaTarget::Blank];
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

impl AppTheme {
    fn load() -> anyhow::Result<Self> {
        let path = theme_file_candidates()
            .into_iter()
            .find(|candidate| candidate.exists());
        let Some(path) = path else {
            return Ok(Self::default());
        };

        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("could not read '{}': {}", path.display(), e))?;
        let theme_file: ThemeFile = serde_yaml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid theme file '{}': {}", path.display(), e))?;
        Self::from_palette(theme_file.colors)
    }

    fn from_palette(p: PaletteFile) -> anyhow::Result<Self> {
        let background = parse_hex_color(p.base.as_str())?;
        let panel_background = parse_hex_color(p.mantle.as_deref().unwrap_or(p.base.as_str()))?;
        let popup_background = parse_hex_color(p.crust.as_deref().unwrap_or(p.base.as_str()))?;
        let foreground = parse_hex_color(p.text.as_str())?;
        let muted = parse_hex_color(p.subtext0.as_deref().unwrap_or(p.text.as_str()))?;
        let border = parse_hex_color(p.overlay0.as_str())?;
        let title_seed = p
            .lavender
            .as_deref()
            .or(p.blue.as_deref())
            .unwrap_or(p.text.as_str());
        let title = parse_hex_color(title_seed)?;
        let selected_background = parse_hex_color(p.surface0.as_str())?;
        let selected_foreground = foreground;
        Ok(Self {
            background,
            panel_background,
            popup_background,
            foreground,
            muted,
            border,
            title,
            selected_background,
            selected_foreground,
        })
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        // Catppuccin Mocha defaults.
        Self {
            background: Color::Rgb(30, 30, 46),
            panel_background: Color::Rgb(24, 24, 37),
            popup_background: Color::Rgb(17, 17, 27),
            foreground: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(166, 173, 200),
            border: Color::Rgb(108, 112, 134),
            title: Color::Rgb(180, 190, 254),
            selected_background: Color::Rgb(49, 50, 68),
            selected_foreground: Color::Rgb(205, 214, 244),
        }
    }
}

fn theme_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(PathBuf::from("theme.yml"));
    candidates.push(PathBuf::from(".theme.yml"));
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(
            Path::new(&home)
                .join(".config")
                .join("ldnddev")
                .join("dd_staticbuilder")
                .join(".theme.yml"),
        );
    }
    candidates
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

fn component_search_haystack(kind: ComponentKind) -> String {
    let label = kind.label();
    let underscore = label.replace('-', "_");
    let short = label
        .trim_start_matches("dd-")
        .replace('-', "_")
        .to_string();
    format!("{label} {underscore} {short}")
}

fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
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

fn next_section_id_for_page(page: &crate::model::Page) -> String {
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

fn ensure_page_section_ids(page: &mut crate::model::Page) {
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn component_edit_group_for_mode(mode: InputMode) -> Option<&'static [InputMode]> {
    match mode {
        InputMode::EditHeroImage
        | InputMode::EditHeroClass
        | InputMode::EditHeroAos
        | InputMode::EditHeroCustomCss
        | InputMode::EditHeroTitle
        | InputMode::EditHeroSubtitle
        | InputMode::EditHeroCopy
        | InputMode::EditHeroCtaText
        | InputMode::EditHeroCtaLink
        | InputMode::EditHeroCtaTarget
        | InputMode::EditHeroCtaText2
        | InputMode::EditHeroCtaLink2
        | InputMode::EditHeroCtaTarget2 => Some(&[
            InputMode::EditHeroImage,
            InputMode::EditHeroClass,
            InputMode::EditHeroAos,
            InputMode::EditHeroCustomCss,
            InputMode::EditHeroTitle,
            InputMode::EditHeroSubtitle,
            InputMode::EditHeroCopy,
            InputMode::EditHeroCtaText,
            InputMode::EditHeroCtaLink,
            InputMode::EditHeroCtaTarget,
            InputMode::EditHeroCtaText2,
            InputMode::EditHeroCtaLink2,
            InputMode::EditHeroCtaTarget2,
        ]),
        InputMode::EditSectionId | InputMode::EditSectionTitle | InputMode::EditSectionClass => {
            Some(&[
                InputMode::EditSectionId,
                InputMode::EditSectionTitle,
                InputMode::EditSectionClass,
            ])
        }
        InputMode::EditBannerClass
        | InputMode::EditBannerDataAos
        | InputMode::EditBannerImageUrl
        | InputMode::EditBannerImageAlt => Some(&[
            InputMode::EditBannerClass,
            InputMode::EditBannerDataAos,
            InputMode::EditBannerImageUrl,
            InputMode::EditBannerImageAlt,
        ]),
        InputMode::EditCtaClass
        | InputMode::EditCtaImageUrl
        | InputMode::EditCtaImageAlt
        | InputMode::EditCtaDataAos
        | InputMode::EditCtaTitle
        | InputMode::EditCtaSubtitle
        | InputMode::EditCtaCopy
        | InputMode::EditCtaLinkUrl
        | InputMode::EditCtaLinkTarget
        | InputMode::EditCtaLinkLabel => Some(&[
            InputMode::EditCtaClass,
            InputMode::EditCtaImageUrl,
            InputMode::EditCtaImageAlt,
            InputMode::EditCtaDataAos,
            InputMode::EditCtaTitle,
            InputMode::EditCtaSubtitle,
            InputMode::EditCtaCopy,
            InputMode::EditCtaLinkUrl,
            InputMode::EditCtaLinkTarget,
            InputMode::EditCtaLinkLabel,
        ]),
        InputMode::EditBlockquoteDataAos
        | InputMode::EditBlockquoteImageUrl
        | InputMode::EditBlockquoteImageAlt
        | InputMode::EditBlockquotePersonsName
        | InputMode::EditBlockquotePersonsTitle
        | InputMode::EditBlockquoteCopy => Some(&[
            InputMode::EditBlockquoteDataAos,
            InputMode::EditBlockquoteImageUrl,
            InputMode::EditBlockquoteImageAlt,
            InputMode::EditBlockquotePersonsName,
            InputMode::EditBlockquotePersonsTitle,
            InputMode::EditBlockquoteCopy,
        ]),
        InputMode::EditCardType
        | InputMode::EditCardDataAos
        | InputMode::EditCardWidth
        | InputMode::EditCardItemImageUrl
        | InputMode::EditCardItemImageAlt
        | InputMode::EditCardItemTitle
        | InputMode::EditCardItemSubtitle
        | InputMode::EditCardItemCopy
        | InputMode::EditCardItemLinkUrl
        | InputMode::EditCardItemLinkTarget
        | InputMode::EditCardItemLinkLabel => Some(&[
            InputMode::EditCardType,
            InputMode::EditCardDataAos,
            InputMode::EditCardWidth,
            InputMode::EditCardItemImageUrl,
            InputMode::EditCardItemImageAlt,
            InputMode::EditCardItemTitle,
            InputMode::EditCardItemSubtitle,
            InputMode::EditCardItemCopy,
            InputMode::EditCardItemLinkUrl,
            InputMode::EditCardItemLinkTarget,
            InputMode::EditCardItemLinkLabel,
        ]),
        InputMode::EditFilmstripType
        | InputMode::EditFilmstripDataAos
        | InputMode::EditFilmstripItemImageUrl
        | InputMode::EditFilmstripItemImageAlt
        | InputMode::EditFilmstripItemTitle => Some(&[
            InputMode::EditFilmstripType,
            InputMode::EditFilmstripDataAos,
            InputMode::EditFilmstripItemImageUrl,
            InputMode::EditFilmstripItemImageAlt,
            InputMode::EditFilmstripItemTitle,
        ]),
        InputMode::EditMilestonesDataAos
        | InputMode::EditMilestonesWidth
        | InputMode::EditMilestonesItemPercentage
        | InputMode::EditMilestonesItemTitle
        | InputMode::EditMilestonesItemSubtitle
        | InputMode::EditMilestonesItemCopy
        | InputMode::EditMilestonesItemLinkUrl
        | InputMode::EditMilestonesItemLinkTarget
        | InputMode::EditMilestonesItemLinkLabel => Some(&[
            InputMode::EditMilestonesDataAos,
            InputMode::EditMilestonesWidth,
            InputMode::EditMilestonesItemPercentage,
            InputMode::EditMilestonesItemTitle,
            InputMode::EditMilestonesItemSubtitle,
            InputMode::EditMilestonesItemCopy,
            InputMode::EditMilestonesItemLinkUrl,
            InputMode::EditMilestonesItemLinkTarget,
            InputMode::EditMilestonesItemLinkLabel,
        ]),
        InputMode::EditAccordionType
        | InputMode::EditAccordionClass
        | InputMode::EditAccordionAos
        | InputMode::EditAccordionGroupName
        | InputMode::EditAccordionFirstTitle
        | InputMode::EditAccordionFirstContent => Some(&[
            InputMode::EditAccordionType,
            InputMode::EditAccordionClass,
            InputMode::EditAccordionAos,
            InputMode::EditAccordionGroupName,
            InputMode::EditAccordionFirstTitle,
            InputMode::EditAccordionFirstContent,
        ]),
        InputMode::EditAlternatingType
        | InputMode::EditAlternatingClass
        | InputMode::EditAlternatingDataAos
        | InputMode::EditAlternatingItemImage
        | InputMode::EditAlternatingItemImageAlt
        | InputMode::EditAlternatingItemTitle
        | InputMode::EditAlternatingItemCopy => Some(&[
            InputMode::EditAlternatingType,
            InputMode::EditAlternatingClass,
            InputMode::EditAlternatingDataAos,
            InputMode::EditAlternatingItemImage,
            InputMode::EditAlternatingItemImageAlt,
            InputMode::EditAlternatingItemTitle,
            InputMode::EditAlternatingItemCopy,
        ]),
        _ => None,
    }
}

fn help_text() -> String {
    [
        "Global:",
        "  F1: Open/close this help",
        "  Ctrl+Q: Quit",
        "  s: Open save modal and enter file path",
        "  Tab / Shift+Tab: Next/previous page",
        "",
        "Node navigation and edits:",
        "  Up/Down or mouse wheel: Select row in Nodes tree",
        "  PageUp/PageDown: Scroll Details blueprint panel",
        "  Enter: Edit selected row",
        "  Space: Expand/collapse selected section or accordion/alternating/card/filmstrip/milestones items",
        "  /: Open insert fuzzy finder (hero/section/cta/banner/blockquote/accordion/alternating/card/filmstrip/milestones)",
        "  A / X: Add/remove dd-accordion, dd-alternating, dd-card, dd-filmstrip, or dd-milestones item",
        "  d: Delete selected node",
        "  J / K: Move selected node down / up",
        "",
        "Section layout:",
        "  C / V: Add/remove selected column",
        "  c / v: Select previous/next column",
        "  ( / ): Move selected column up/down",
        "  r / f: Edit selected column id / width class",
        "  Details pane shows ASCII blueprint for all page items",
        "",
        "Edit modal:",
        "  Any edit command opens a modal with editable fields",
        "  Tab / Shift+Tab: Next/previous editable field for selected row",
        "  hero.copy / alternating_copy / accordion_copy / blockquote_copy / card_copy / child_copy: Up/Down move line, wheel scroll, Enter newline, Ctrl+S save",
        "  Left / Right: Cycle section/hero/cta/banner/accordion/alternating/blockquote/card/filmstrip/milestones option fields when active",
        "  Enter: Confirm edit",
        "  Esc: Cancel edit",
        "  Backspace: Delete character",
    ]
    .join("\n")
}

impl ComponentKind {
    fn all() -> &'static [Self] {
        &[
            Self::Hero,
            Self::Section,
            Self::Cta,
            Self::Banner,
            Self::Blockquote,
            Self::Accordion,
            Self::Alternating,
            Self::Card,
            Self::Filmstrip,
            Self::Milestones,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            ComponentKind::Hero => "dd-hero",
            ComponentKind::Section => "dd-section",
            ComponentKind::Cta => "dd-cta",
            ComponentKind::Banner => "dd-banner",
            ComponentKind::Blockquote => "dd-blockquote",
            ComponentKind::Accordion => "dd-accordion",
            ComponentKind::Alternating => "dd-alternating",
            ComponentKind::Card => "dd-card",
            ComponentKind::Filmstrip => "dd-filmstrip",
            ComponentKind::Milestones => "dd-milestones",
        }
    }

    fn default_component(self) -> crate::model::SectionComponent {
        match self {
            ComponentKind::Hero | ComponentKind::Section => {
                unreachable!("top-level kinds do not map to section components")
            }
            ComponentKind::Cta => crate::model::SectionComponent::Cta(crate::model::DdCta {
                cta_class: crate::model::CtaClass::TopLeft,
                cta_image_url: "https://dummyimage.com/1920x1080/000000/fff".to_string(),
                cta_image_alt: "Image alt".to_string(),
                cta_data_aos: crate::model::HeroAos::FadeIn,
                cta_title: "Title".to_string(),
                cta_subtitle: "Subtitle".to_string(),
                cta_copy: "Copy".to_string(),
                cta_link_url: Some("/path".to_string()),
                cta_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                cta_link_label: Some("Learn More".to_string()),
            }),
            ComponentKind::Banner => {
                crate::model::SectionComponent::Banner(crate::model::DdBanner {
                    banner_class: crate::model::BannerClass::BgCenterCenter,
                    banner_data_aos: crate::model::HeroAos::FadeIn,
                    banner_image_url: "https://dummyimage.com/1920x1080/000/fff".to_string(),
                    banner_image_alt: "Banner alt text".to_string(),
                })
            }
            ComponentKind::Blockquote => {
                crate::model::SectionComponent::Blockquote(crate::model::DdBlockquote {
                    blockquote_data_aos: crate::model::HeroAos::FadeIn,
                    blockquote_image_url: "https://dummyimage.com/512x512/000/fff".to_string(),
                    blockquote_image_alt: "blockquote Persons Name".to_string(),
                    blockquote_persons_name: "blockquote Persons Name".to_string(),
                    blockquote_persons_title: "blockquote Persons Title".to_string(),
                    blockquote_copy: "blockquote content".to_string(),
                })
            }
            ComponentKind::Accordion => {
                crate::model::SectionComponent::Accordion(crate::model::DdAccordion {
                    accordion_type: crate::model::AccordionType::Default,
                    accordion_class: crate::model::AccordionClass::Primary,
                    accordion_aos: crate::model::HeroAos::FadeIn,
                    group_name: "group1".to_string(),
                    items: vec![crate::model::AccordionItem {
                        title: "Accordion Item".to_string(),
                        content: "Accordion content".to_string(),
                    }],
                    multiple: Some(false),
                })
            }
            ComponentKind::Alternating => {
                crate::model::SectionComponent::Alternating(crate::model::DdAlternating {
                    alternating_type: crate::model::AlternatingType::Default,
                    alternating_class: "-default".to_string(),
                    alternating_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::AlternatingItem {
                        image: "https://dummyimage.com/600x400/000/fff".to_string(),
                        image_alt: "Alternating image".to_string(),
                        title: "Alternating Item".to_string(),
                        copy: "Alternating content".to_string(),
                    }],
                })
            }
            ComponentKind::Card => crate::model::SectionComponent::Card(crate::model::DdCard {
                card_type: crate::model::CardType::Default,
                card_data_aos: crate::model::HeroAos::FadeIn,
                card_width: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string(),
                items: vec![crate::model::CardItem {
                    card_image_url: "https://dummyimage.com/720x720/000/fff".to_string(),
                    card_image_alt: "Image alt text".to_string(),
                    card_title: "Title".to_string(),
                    card_subtitle: "Subtitle".to_string(),
                    card_copy: "Copy".to_string(),
                    card_link_url: Some("/front".to_string()),
                    card_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                    card_link_label: Some("Learn More".to_string()),
                }],
            }),
            ComponentKind::Filmstrip => {
                crate::model::SectionComponent::Filmstrip(crate::model::DdFilmstrip {
                    filmstrip_type: crate::model::FilmstripType::Default,
                    filmstrip_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::FilmstripItem {
                        image_url: "https://dummyimage.com/256x256/000/fff".to_string(),
                        image_alt: "Image alt text".to_string(),
                        title: "Title".to_string(),
                    }],
                })
            }
            ComponentKind::Milestones => {
                crate::model::SectionComponent::Milestones(crate::model::DdMilestones {
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_width: "dd-u-1-1 dd-u-md-12-24".to_string(),
                    items: vec![crate::model::MilestonesItem {
                        child_percentage: "70".to_string(),
                        child_title: "Title".to_string(),
                        child_subtitle: "Subtitle".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: None,
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: None,
                    }],
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn app_with_card() -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            normalize_section_columns(section);
            section.columns[0]
                .components
                .push(ComponentKind::Card.default_component());
        } else {
            panic!("expected starter node 2 to be dd-section");
        }
        app.selected_column = 0;
        app.selected_component = 0;
        app.selected_nested_item = 0;
        app.sync_tree_row_with_selection();
        app
    }

    fn send_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
        app.handle_event(Event::Key(KeyEvent::new(code, modifiers)))
            .expect("key event should be handled");
    }

    fn selected_card(app: &App) -> &crate::model::DdCard {
        let page = &app.site.pages[app.selected_page];
        let section = match &page.nodes[app.selected_node] {
            PageNode::Section(section) => section,
            _ => panic!("selected node is not dd-section"),
        };
        let component = &section.columns[app.selected_column].components[app.selected_component];
        match component {
            crate::model::SectionComponent::Card(card) => card,
            _ => panic!("selected component is not dd-card"),
        }
    }

    fn app_with_cta() -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            normalize_section_columns(section);
            section.columns[0]
                .components
                .push(ComponentKind::Cta.default_component());
        } else {
            panic!("expected starter node 2 to be dd-section");
        }
        app.selected_column = 0;
        app.selected_component = 0;
        app.selected_nested_item = 0;
        app.sync_tree_row_with_selection();
        app
    }

    fn selected_cta(app: &App) -> &crate::model::DdCta {
        let page = &app.site.pages[app.selected_page];
        let section = match &page.nodes[app.selected_node] {
            PageNode::Section(section) => section,
            _ => panic!("selected node is not dd-section"),
        };
        let component = &section.columns[app.selected_column].components[app.selected_component];
        match component {
            crate::model::SectionComponent::Cta(cta) => cta,
            _ => panic!("selected component is not dd-cta"),
        }
    }

    #[test]
    fn dd_card_keyflow_enter_tab_backtab_and_left_right_parent_fields() {
        let mut app = app_with_card();
        let rows = app.build_node_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    NodeTreeKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("dd-card component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.input_mode, Some(InputMode::EditCardType)));
        assert_eq!(app.input_buffer, "-default");

        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.input_buffer, "-horizontal");
        assert_eq!(
            selected_card(&app).card_type,
            crate::model::CardType::Horizontal
        );

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert!(matches!(app.input_mode, Some(InputMode::EditCardDataAos)));
        let prev_aos = app.input_buffer.clone();
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_ne!(app.input_buffer, prev_aos);

        send_key(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert!(matches!(app.input_mode, Some(InputMode::EditCardType)));
    }

    #[test]
    fn dd_card_keyflow_item_row_enter_and_link_target_cycle() {
        let mut app = app_with_card();

        let rows = app.build_node_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| matches!(row.kind, NodeTreeKind::CardItem { .. }))
            .expect("card item row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(
            app.input_mode,
            Some(InputMode::EditCardItemImageUrl)
        ));
        let fields = app.current_modal_fields();
        assert!(fields.contains("- card_title:"));
        assert!(!fields.contains("- section.id:"));

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // image_alt
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // title
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // subtitle
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // copy
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // link_url
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // link_target
        assert!(matches!(
            app.input_mode,
            Some(InputMode::EditCardItemLinkTarget)
        ));
        assert_eq!(app.input_buffer, "_self");

        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.input_buffer, "_blank");
    }

    #[test]
    fn dd_card_keyflow_add_remove_items_with_min_guard() {
        let mut app = app_with_card();
        assert_eq!(selected_card(&app).items.len(), 1);

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 2);

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 1);

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 1);
        assert!(app.status.contains("must keep at least one item"));
    }

    #[test]
    fn dd_cta_keyflow_enter_tab_backtab_and_left_right_cycle_fields() {
        let mut app = app_with_cta();
        let rows = app.build_node_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    NodeTreeKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("dd-cta component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaClass)));
        assert_eq!(app.input_buffer, "-top-left");

        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.input_buffer, "-top-center");
        assert_eq!(
            selected_cta(&app).cta_class,
            crate::model::CtaClass::TopCenter
        );

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // image_url
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaImageUrl)));
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // image_alt
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // data_aos
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaDataAos)));
        let prev_aos = app.input_buffer.clone();
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_ne!(app.input_buffer, prev_aos);

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // title
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // subtitle
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // copy
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaCopy)));

        send_key(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaSubtitle)));
    }

    #[test]
    fn dd_cta_keyflow_link_target_cycle_and_optional_fields() {
        let mut app = app_with_cta();
        let rows = app.build_node_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    NodeTreeKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("dd-cta component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        for _ in 0..8 {
            send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        }
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaLinkTarget)));
        assert_eq!(app.input_buffer, "_self");

        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.input_buffer, "_blank");
        assert_eq!(
            selected_cta(&app).cta_link_target,
            Some(crate::model::CardLinkTarget::Blank)
        );

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert!(matches!(app.input_mode, Some(InputMode::EditCtaLinkLabel)));
        let fields = app.current_modal_fields();
        assert!(fields.contains("cta_title"));
        assert!(!fields.contains("section.id"));
    }
}
