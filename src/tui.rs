use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
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
    status: String,
    path: Option<PathBuf>,
    should_quit: bool,
    save_prompt_open: bool,
    save_input: String,
    input_mode: Option<InputMode>,
    input_buffer: String,
    component_picker: Option<ComponentPickerState>,
    component_kind: ComponentKind,
    show_help: bool,
    expanded_sections: HashSet<(usize, usize)>,
    expanded_accordion_items: HashSet<(usize, usize, usize, usize)>,
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
    EditHeroTitle,
    EditHeroSubtitle,
    EditHeroCopy,
    EditHeroCtaText,
    EditHeroCtaLink,
    EditHeroCtaText2,
    EditHeroCtaLink2,
    EditSectionId,
    EditSectionTitle,
    EditSectionClass,
    EditColumnId,
    EditColumnWidthClass,
    EditCardTitle,
    EditCardCopy,
    EditCtaTitle,
    EditCtaLink,
    EditAlertMessage,
    EditAlertTitle,
    EditBannerMessage,
    EditBannerLinkUrl,
    EditTabsFirstTitle,
    EditTabsFirstContent,
    EditAccordionType,
    EditAccordionClass,
    EditAccordionAos,
    EditAccordionGroupName,
    EditAccordionFirstTitle,
    EditAccordionFirstContent,
    EditModalTitle,
    EditModalContent,
    EditSliderFirstTitle,
    EditSliderFirstCopy,
    EditSpacerHeight,
    EditTimelineFirstTitle,
    EditTimelineFirstDescription,
}

#[derive(Clone, Copy)]
enum ComponentKind {
    Hero,
    Section,
    Card,
    Alert,
    Banner,
    Tabs,
    Accordion,
    Cta,
    Modal,
    Slider,
    Spacer,
    Timeline,
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
            status: "Ready.".to_string(),
            path,
            should_quit: false,
            save_prompt_open: false,
            save_input: String::new(),
            input_mode: None,
            input_buffer: String::new(),
            component_picker: None,
            component_kind: ComponentKind::Card,
            show_help: false,
            expanded_sections: HashSet::new(),
            expanded_accordion_items: HashSet::new(),
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

        let details = Paragraph::new(self.details_text(main[1].width.saturating_sub(2) as usize))
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
            .wrap(Wrap { trim: true });
        frame.render_widget(details, main[1]);

        let footer_text = format!(
            "F1 help | q quit | s save | / insert | Enter edit | Space expand/collapse | A add accordion item | X remove accordion item | {}",
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
            let modal = Paragraph::new(format!(
                "Editing: {}\n\nValue:\n{}\n\nEditable fields:\n{}\n\nEnter: save | Esc: cancel",
                self.current_input_mode_label(),
                self.input_buffer,
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
            lines.push("Type to fuzzy search (e.g. card, dd_card).".to_string());
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

        self.set_cursor_for_active_input(frame);
    }

    fn set_cursor_for_active_input(&self, frame: &mut ratatui::Frame) {
        if self.input_mode.is_some() {
            let area = centered_rect(72, 60, frame.area());
            let inner_width = area.width.saturating_sub(2) as usize;
            let x = area.x.saturating_add(1).saturating_add(
                self.input_buffer
                    .chars()
                    .count()
                    .min(inner_width.saturating_sub(1)) as u16,
            );
            let y = area.y.saturating_add(4);
            frame.set_cursor_position((x, y));
            return;
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
            return;
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
        }
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
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Up => self.select_prev(),
                KeyCode::Down => self.select_next(),
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
                KeyCode::Char('A') => self.add_selected_accordion_item(),
                KeyCode::Char('X') => self.remove_selected_accordion_item(),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => self.select_prev(),
                MouseEventKind::ScrollDown => self.select_next(),
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
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = None;
                    self.input_buffer.clear();
                    self.status = "Edit cancelled.".to_string();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Enter => {
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
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
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
                    _ => {}
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
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
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
                    _ => {}
                },
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }
        }
        Ok(())
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
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
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
            InputMode::EditCardTitle => {
                "Editing dd-card title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardCopy => {
                "Editing dd-card copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLink => {
                "Editing dd-cta link. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertMessage => {
                "Editing dd-alert message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertTitle => {
                "Editing dd-alert title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerMessage => {
                "Editing dd-banner message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerLinkUrl => {
                "Editing dd-banner link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstTitle => {
                "Editing dd-tabs first title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstContent => {
                "Editing dd-tabs first content. Enter to save, esc to cancel.".to_string()
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
                "Editing dd-accordion first content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalTitle => {
                "Editing dd-modal title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalContent => {
                "Editing dd-modal content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstTitle => {
                "Editing dd-slider first slide title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstCopy => {
                "Editing dd-slider first slide copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSpacerHeight => {
                "Editing dd-spacer height (sm|md|lg|xl|xxl). Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditTimelineFirstTitle => {
                "Editing dd-timeline first event title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTimelineFirstDescription => {
                "Editing dd-timeline first event description. Enter to save, esc to cancel."
                    .to_string()
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
        self.status =
            "Editing selected column width class. Enter to save, esc to cancel.".to_string();
    }

    fn commit_input_edit(&mut self) -> bool {
        let Some(mode) = self.input_mode else {
            return false;
        };
        let value = self.input_buffer.trim().to_string();
        let allow_empty = matches!(
            mode,
            InputMode::EditHeroImage
                | InputMode::EditHeroClass
                | InputMode::EditHeroSubtitle
                | InputMode::EditHeroCopy
                | InputMode::EditHeroCtaText
                | InputMode::EditHeroCtaLink
                | InputMode::EditHeroCtaText2
                | InputMode::EditHeroCtaLink2
                | InputMode::EditSectionTitle
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
            (PageNode::Section(v), InputMode::EditCardTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        card.title = value;
                        applied = true;
                        "Updated dd-card title.".to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        card.copy = Some(value);
                        applied = true;
                        "Updated dd-card copy.".to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.title = value;
                        applied = true;
                        "Updated dd-cta title.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLink) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_link = value;
                        applied = true;
                        "Updated dd-cta link.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlertMessage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alert(alert) = &mut v.components[ci] {
                        alert.message = value;
                        applied = true;
                        "Updated dd-alert message.".to_string()
                    } else {
                        "Selected component is not dd-alert.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlertTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alert(alert) = &mut v.components[ci] {
                        alert.title = Some(value);
                        applied = true;
                        "Updated dd-alert title.".to_string()
                    } else {
                        "Selected component is not dd-alert.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerMessage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.message = value;
                        applied = true;
                        "Updated dd-banner message.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerLinkUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.link_url = Some(value);
                        applied = true;
                        "Updated dd-banner link_url.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditTabsFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Tabs(tabs) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tabs.tabs.len(), selected_nested_item) {
                            tabs.tabs[ni].title = value;
                            applied = true;
                            format!("Updated dd-tabs item {} title.", ni + 1)
                        } else {
                            "dd-tabs has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-tabs.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditTabsFirstContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Tabs(tabs) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tabs.tabs.len(), selected_nested_item) {
                            tabs.tabs[ni].content = value;
                            applied = true;
                            format!("Updated dd-tabs item {} content.", ni + 1)
                        } else {
                            "dd-tabs has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-tabs.".to_string()
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
            (PageNode::Section(v), InputMode::EditModalTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.components[ci] {
                        modal.title = value;
                        applied = true;
                        "Updated dd-modal title.".to_string()
                    } else {
                        "Selected component is not dd-modal.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditModalContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.components[ci] {
                        modal.content = value;
                        applied = true;
                        "Updated dd-modal content.".to_string()
                    } else {
                        "Selected component is not dd-modal.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(slider.slides.len(), selected_nested_item) {
                            slider.slides[ni].title = value;
                            applied = true;
                            format!("Updated dd-slider slide {} title.", ni + 1)
                        } else {
                            "dd-slider has no slides.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderFirstCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(slider.slides.len(), selected_nested_item) {
                            slider.slides[ni].copy = value;
                            applied = true;
                            format!("Updated dd-slider slide {} copy.", ni + 1)
                        } else {
                            "dd-slider has no slides.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSpacerHeight) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Spacer(spacer) = &mut v.components[ci] {
                        if let Some(height) = parse_spacer_height(&value) {
                            spacer.height = height;
                            applied = true;
                            "Updated dd-spacer height.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid spacer height. Use sm|md|lg|xl|xxl.".to_string()
                        }
                    } else {
                        "Selected component is not dd-spacer.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditTimelineFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Timeline(tl) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tl.events.len(), selected_nested_item) {
                            tl.events[ni].title = value;
                            applied = true;
                            format!("Updated dd-timeline event {} title.", ni + 1)
                        } else {
                            "dd-timeline has no events.".to_string()
                        }
                    } else {
                        "Selected component is not dd-timeline.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditTimelineFirstDescription) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Timeline(tl) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tl.events.len(), selected_nested_item) {
                            tl.events[ni].description = value;
                            applied = true;
                            format!("Updated dd-timeline event {} description.", ni + 1)
                        } else {
                            "dd-timeline has no events.".to_string()
                        }
                    } else {
                        "Selected component is not dd-timeline.".to_string()
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
        }
        let node_idx = match row.kind {
            NodeTreeKind::Section { node_idx } => node_idx,
            NodeTreeKind::Column { node_idx, .. } => node_idx,
            NodeTreeKind::Component { node_idx, .. } => node_idx,
            NodeTreeKind::AccordionItem { node_idx, .. } => node_idx,
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
            Some(InputMode::EditHeroTitle) => "hero.title",
            Some(InputMode::EditHeroSubtitle) => "hero.subtitle",
            Some(InputMode::EditHeroCopy) => "hero.copy",
            Some(InputMode::EditHeroCtaText) => "hero.link_1.text",
            Some(InputMode::EditHeroCtaLink) => "hero.link_1.url",
            Some(InputMode::EditHeroCtaText2) => "hero.link_2.text",
            Some(InputMode::EditHeroCtaLink2) => "hero.link_2.url",
            Some(InputMode::EditSectionId) => "section.id",
            Some(InputMode::EditSectionTitle) => "section.title",
            Some(InputMode::EditSectionClass) => "section.class",
            Some(InputMode::EditColumnId) => "section.column.id",
            Some(InputMode::EditColumnWidthClass) => "section.column.width_class",
            Some(InputMode::EditCardTitle) => "dd-card.title",
            Some(InputMode::EditCardCopy) => "dd-card.copy",
            Some(InputMode::EditCtaTitle) => "dd-cta.title",
            Some(InputMode::EditCtaLink) => "dd-cta.cta_link",
            Some(InputMode::EditAlertMessage) => "dd-alert.message",
            Some(InputMode::EditAlertTitle) => "dd-alert.title",
            Some(InputMode::EditBannerMessage) => "dd-banner.message",
            Some(InputMode::EditBannerLinkUrl) => "dd-banner.link_url",
            Some(InputMode::EditTabsFirstTitle) => "dd-tabs.active.title",
            Some(InputMode::EditTabsFirstContent) => "dd-tabs.active.content",
            Some(InputMode::EditAccordionType) => "dd-accordion.type",
            Some(InputMode::EditAccordionClass) => "dd-accordion.class",
            Some(InputMode::EditAccordionAos) => "dd-accordion.data_aos",
            Some(InputMode::EditAccordionGroupName) => "dd-accordion.group_name",
            Some(InputMode::EditAccordionFirstTitle) => "dd-accordion.active.title",
            Some(InputMode::EditAccordionFirstContent) => "dd-accordion.active.content",
            Some(InputMode::EditModalTitle) => "dd-modal.title",
            Some(InputMode::EditModalContent) => "dd-modal.content",
            Some(InputMode::EditSliderFirstTitle) => "dd-slider.active.title",
            Some(InputMode::EditSliderFirstCopy) => "dd-slider.active.copy",
            Some(InputMode::EditSpacerHeight) => "dd-spacer.height",
            Some(InputMode::EditTimelineFirstTitle) => "dd-timeline.active.title",
            Some(InputMode::EditTimelineFirstDescription) => "dd-timeline.active.description",
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
                "- hero.image: {}\n- hero.class: {}\n- hero.data_aos: {}\n- hero.title: {}\n- hero.subtitle: {}\n- hero.copy: {}\n- hero.link_1.text: {}\n- hero.link_1.url: {}\n- hero.link_2.text: {}\n- hero.link_2.url: {}",
                v.image,
                hero_image_class_to_str(
                    v.hero_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
                hero_aos_to_str(v.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn)),
                v.title,
                v.subtitle,
                v.copy.as_deref().unwrap_or("(none)"),
                v.cta_text.as_deref().unwrap_or("(none)"),
                v.cta_link.as_deref().unwrap_or("(none)"),
                v.cta_text_2.as_deref().unwrap_or("(none)"),
                v.cta_link_2.as_deref().unwrap_or("(none)")
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
                    | Some(NodeTreeKind::AccordionItem { .. }) => {
                        let columns = section_columns_ref(section);
                        if let Some(col) =
                            columns.get(self.selected_column.min(columns.len().saturating_sub(1)))
                        {
                            if let Some(component) = col.components.get(
                                self.selected_component
                                    .min(col.components.len().saturating_sub(1)),
                            ) {
                                if let crate::model::SectionComponent::Accordion(acc) = component {
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
                                    let mut lines = vec![
                                        format!("- column.id: {}", col.id),
                                        format!("- column.width_class: {}", col.width_class),
                                    ];
                                    lines
                                        .push(component_form(component, self.selected_nested_item));
                                    lines.join("\n")
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
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
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
            InputMode::EditCardTitle => {
                "Editing dd-card title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardCopy => {
                "Editing dd-card copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLink => {
                "Editing dd-cta link. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertMessage => {
                "Editing dd-alert message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertTitle => {
                "Editing dd-alert title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerMessage => {
                "Editing dd-banner message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerLinkUrl => {
                "Editing dd-banner link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstTitle => {
                "Editing dd-tabs item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstContent => {
                "Editing dd-tabs item content. Enter to save, esc to cancel.".to_string()
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
                "Editing dd-accordion item content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalTitle => {
                "Editing dd-modal title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalContent => {
                "Editing dd-modal content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstTitle => {
                "Editing dd-slider slide title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstCopy => {
                "Editing dd-slider slide copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSpacerHeight => {
                "Editing dd-spacer height (sm|md|lg|xl|xxl). Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditTimelineFirstTitle => {
                "Editing dd-timeline event title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTimelineFirstDescription => {
                "Editing dd-timeline event description. Enter to save, esc to cancel.".to_string()
            }
            _ => "Editing field. Enter to save, esc to cancel.".to_string(),
        };
        true
    }

    fn value_for_component_mode(&self, mode: InputMode) -> Option<String> {
        let page = self.current_page();
        if !page.nodes.is_empty() {
            let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
            if let PageNode::Hero(hero) = &page.nodes[ni] {
                match mode {
                    InputMode::EditHeroImage => return Some(hero.image.clone()),
                    InputMode::EditHeroClass => {
                        return Some(
                            hero_image_class_to_str(
                                hero.hero_class
                                    .unwrap_or(crate::model::HeroImageClass::FullFull),
                            )
                            .to_string(),
                        );
                    }
                    InputMode::EditHeroAos => {
                        return Some(
                            hero_aos_to_str(hero.hero_aos.unwrap_or(crate::model::HeroAos::FadeIn))
                                .to_string(),
                        );
                    }
                    InputMode::EditHeroTitle => return Some(hero.title.clone()),
                    InputMode::EditHeroSubtitle => return Some(hero.subtitle.clone()),
                    InputMode::EditHeroCopy => {
                        return Some(hero.copy.clone().unwrap_or_default());
                    }
                    InputMode::EditHeroCtaText => {
                        return Some(hero.cta_text.clone().unwrap_or_default());
                    }
                    InputMode::EditHeroCtaLink => {
                        return Some(hero.cta_link.clone().unwrap_or_default());
                    }
                    InputMode::EditHeroCtaText2 => {
                        return Some(hero.cta_text_2.clone().unwrap_or_default());
                    }
                    InputMode::EditHeroCtaLink2 => {
                        return Some(hero.cta_link_2.clone().unwrap_or_default());
                    }
                    _ => {}
                }
            }
            if let PageNode::Section(section) = &page.nodes[ni] {
                match mode {
                    InputMode::EditSectionId => return Some(section.id.clone()),
                    InputMode::EditSectionTitle => {
                        return Some(section.section_title.clone().unwrap_or_default());
                    }
                    InputMode::EditSectionClass => {
                        return Some(
                            section_class_to_str(
                                section
                                    .section_class
                                    .unwrap_or(crate::model::SectionClass::FullContained),
                            )
                            .to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }

        let component = self.selected_component_owned()?;
        match (mode, component) {
            (InputMode::EditCardTitle, crate::model::SectionComponent::Card(v)) => {
                Some(v.title.clone())
            }
            (InputMode::EditCardCopy, crate::model::SectionComponent::Card(v)) => {
                Some(v.copy.clone().unwrap_or_default())
            }
            (InputMode::EditCtaTitle, crate::model::SectionComponent::Cta(v)) => {
                Some(v.title.clone())
            }
            (InputMode::EditCtaLink, crate::model::SectionComponent::Cta(v)) => {
                Some(v.cta_link.clone())
            }
            (InputMode::EditAlertMessage, crate::model::SectionComponent::Alert(v)) => {
                Some(v.message.clone())
            }
            (InputMode::EditAlertTitle, crate::model::SectionComponent::Alert(v)) => {
                Some(v.title.clone().unwrap_or_default())
            }
            (InputMode::EditBannerMessage, crate::model::SectionComponent::Banner(v)) => {
                Some(v.message.clone())
            }
            (InputMode::EditBannerLinkUrl, crate::model::SectionComponent::Banner(v)) => {
                Some(v.link_url.clone().unwrap_or_default())
            }
            (InputMode::EditModalTitle, crate::model::SectionComponent::Modal(v)) => {
                Some(v.title.clone())
            }
            (InputMode::EditModalContent, crate::model::SectionComponent::Modal(v)) => {
                Some(v.content.clone())
            }
            (InputMode::EditSpacerHeight, crate::model::SectionComponent::Spacer(v)) => {
                Some(spacer_height_to_str(v.height).to_string())
            }
            (InputMode::EditTabsFirstTitle, crate::model::SectionComponent::Tabs(v)) => {
                let ni = nested_index(v.tabs.len(), self.selected_nested_item)?;
                Some(v.tabs[ni].title.clone())
            }
            (InputMode::EditTabsFirstContent, crate::model::SectionComponent::Tabs(v)) => {
                let ni = nested_index(v.tabs.len(), self.selected_nested_item)?;
                Some(v.tabs[ni].content.clone())
            }
            (InputMode::EditAccordionType, crate::model::SectionComponent::Accordion(v)) => {
                Some(accordion_type_to_str(v.accordion_type).to_string())
            }
            (InputMode::EditAccordionClass, crate::model::SectionComponent::Accordion(v)) => {
                Some(accordion_class_to_str(v.accordion_class).to_string())
            }
            (InputMode::EditAccordionAos, crate::model::SectionComponent::Accordion(v)) => {
                Some(hero_aos_to_str(v.accordion_aos).to_string())
            }
            (InputMode::EditAccordionGroupName, crate::model::SectionComponent::Accordion(v)) => {
                Some(v.group_name.clone())
            }
            (InputMode::EditAccordionFirstTitle, crate::model::SectionComponent::Accordion(v)) => {
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
            (InputMode::EditSliderFirstTitle, crate::model::SectionComponent::Slider(v)) => {
                let ni = nested_index(v.slides.len(), self.selected_nested_item)?;
                Some(v.slides[ni].title.clone())
            }
            (InputMode::EditSliderFirstCopy, crate::model::SectionComponent::Slider(v)) => {
                let ni = nested_index(v.slides.len(), self.selected_nested_item)?;
                Some(v.slides[ni].copy.clone())
            }
            (InputMode::EditTimelineFirstTitle, crate::model::SectionComponent::Timeline(v)) => {
                let ni = nested_index(v.events.len(), self.selected_nested_item)?;
                Some(v.events[ni].title.clone())
            }
            (
                InputMode::EditTimelineFirstDescription,
                crate::model::SectionComponent::Timeline(v),
            ) => {
                let ni = nested_index(v.events.len(), self.selected_nested_item)?;
                Some(v.events[ni].description.clone())
            }
            _ => None,
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
                                crate::model::SectionComponent::Card(card) => {
                                    Some((InputMode::EditCardTitle, card.title.clone()))
                                }
                                crate::model::SectionComponent::Cta(cta) => {
                                    Some((InputMode::EditCtaTitle, cta.title.clone()))
                                }
                                crate::model::SectionComponent::Alert(alert) => {
                                    Some((InputMode::EditAlertMessage, alert.message.clone()))
                                }
                                crate::model::SectionComponent::Banner(banner) => {
                                    Some((InputMode::EditBannerMessage, banner.message.clone()))
                                }
                                crate::model::SectionComponent::Tabs(tabs) => {
                                    if let Some(ni) =
                                        nested_index(tabs.tabs.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditTabsFirstTitle,
                                            tabs.tabs[ni].title.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                crate::model::SectionComponent::Accordion(acc) => Some((
                                    InputMode::EditAccordionType,
                                    accordion_type_to_str(acc.accordion_type).to_string(),
                                )),
                                crate::model::SectionComponent::Modal(modal) => {
                                    Some((InputMode::EditModalTitle, modal.title.clone()))
                                }
                                crate::model::SectionComponent::Slider(slider) => {
                                    if let Some(ni) =
                                        nested_index(slider.slides.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditSliderFirstTitle,
                                            slider.slides[ni].title.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                crate::model::SectionComponent::Spacer(spacer) => Some((
                                    InputMode::EditSpacerHeight,
                                    spacer_height_to_str(spacer.height).to_string(),
                                )),
                                crate::model::SectionComponent::Timeline(tl) => {
                                    if let Some(ni) =
                                        nested_index(tl.events.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditTimelineFirstTitle,
                                            tl.events[ni].title.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
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
                "Primary edit supports card/cta/alert/banner/tabs/accordion/modal/slider/spacer/timeline.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.status = match mode {
            InputMode::EditCardTitle => {
                "Editing dd-card title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertMessage => {
                "Editing dd-alert message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerMessage => {
                "Editing dd-banner message. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstTitle => {
                "Editing dd-tabs first title. Enter to save, esc to cancel.".to_string()
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
            InputMode::EditModalTitle => {
                "Editing dd-modal title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstTitle => {
                "Editing dd-slider first slide title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSpacerHeight => {
                "Editing dd-spacer height (sm|md|lg|xl|xxl). Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditTimelineFirstTitle => {
                "Editing dd-timeline first title. Enter to save, esc to cancel.".to_string()
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
                    box_lines.push(format!(
                        "| {} |",
                        fit_ascii_cell(
                            &format!("- {}", component_blueprint_label(component)),
                            item_inner_width
                        )
                    ));
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
        crate::model::SectionComponent::Card(_) => "dd-card",
        crate::model::SectionComponent::Alert(_) => "dd-alert",
        crate::model::SectionComponent::Banner(_) => "dd-banner",
        crate::model::SectionComponent::Tabs(_) => "dd-tabs",
        crate::model::SectionComponent::Accordion(_) => "dd-accordion",
        crate::model::SectionComponent::Cta(_) => "dd-cta",
        crate::model::SectionComponent::Modal(_) => "dd-modal",
        crate::model::SectionComponent::Slider(_) => "dd-slider",
        crate::model::SectionComponent::Spacer(_) => "dd-spacer",
        crate::model::SectionComponent::Timeline(_) => "dd-timeline",
    }
}

fn component_blueprint_label(component: &crate::model::SectionComponent) -> String {
    match component {
        crate::model::SectionComponent::Accordion(v) => format!(
            "dd-accordion | accordion_title: {}",
            v.items
                .first()
                .map(|i| i.title.as_str())
                .unwrap_or("(none)")
        ),
        _ => component_label(component).to_string(),
    }
}

fn component_form(
    component: &crate::model::SectionComponent,
    selected_nested_item: usize,
) -> String {
    match component {
        crate::model::SectionComponent::Card(v) => format!(
            "fields:\n  title: {}\n  image: {}\n  copy: {}\n  cta_link: {}",
            v.title,
            v.image,
            v.copy.as_deref().unwrap_or("(none)"),
            v.cta_link.as_deref().unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Alert(v) => format!(
            "fields:\n  type: {:?}\n  message: {}\n  title: {}",
            v.alert_type,
            v.message,
            v.title.as_deref().unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Banner(v) => format!(
            "fields:\n  message: {}\n  background: {}\n  link_url: {}",
            v.message,
            v.background,
            v.link_url.as_deref().unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Tabs(v) => {
            let active = nested_index(v.tabs.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let title = nested_index(v.tabs.len(), selected_nested_item)
                .and_then(|i| v.tabs.get(i))
                .map(|t| t.title.as_str())
                .unwrap_or("(none)");
            let content = nested_index(v.tabs.len(), selected_nested_item)
                .and_then(|i| v.tabs.get(i))
                .map(|t| t.content.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  tabs_count: {}\n  active_tab: {}\n  active_title: {}\n  active_content: {}",
                v.tabs.len(),
                active,
                title,
                content
            )
        }
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
        crate::model::SectionComponent::Cta(v) => {
            format!("fields:\n  title: {}", v.title)
        }
        crate::model::SectionComponent::Modal(v) => format!(
            "fields:\n  trigger_text: {}\n  title: {}\n  content: {}",
            v.trigger_text, v.title, v.content
        ),
        crate::model::SectionComponent::Slider(v) => {
            let active = nested_index(v.slides.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let title = nested_index(v.slides.len(), selected_nested_item)
                .and_then(|i| v.slides.get(i))
                .map(|s| s.title.as_str())
                .unwrap_or("(none)");
            let copy = nested_index(v.slides.len(), selected_nested_item)
                .and_then(|i| v.slides.get(i))
                .map(|s| s.copy.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  slides_count: {}\n  active_slide: {}\n  active_title: {}\n  active_copy: {}",
                v.slides.len(),
                active,
                title,
                copy
            )
        }
        crate::model::SectionComponent::Spacer(v) => {
            format!("fields:\n  height: {}", spacer_height_to_str(v.height))
        }
        crate::model::SectionComponent::Timeline(v) => {
            let active = nested_index(v.events.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let title = nested_index(v.events.len(), selected_nested_item)
                .and_then(|i| v.events.get(i))
                .map(|e| e.title.as_str())
                .unwrap_or("(none)");
            let desc = nested_index(v.events.len(), selected_nested_item)
                .and_then(|i| v.events.get(i))
                .map(|e| e.description.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  events_count: {}\n  active_event: {}\n  active_title: {}\n  active_description: {}",
                v.events.len(),
                active,
                title,
                desc
            )
        }
    }
}

fn spacer_height_to_str(v: crate::model::SpacerHeight) -> &'static str {
    match v {
        crate::model::SpacerHeight::Sm => "sm",
        crate::model::SpacerHeight::Md => "md",
        crate::model::SpacerHeight::Lg => "lg",
        crate::model::SpacerHeight::Xl => "xl",
        crate::model::SpacerHeight::Xxl => "xxl",
    }
}

fn parse_spacer_height(raw: &str) -> Option<crate::model::SpacerHeight> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sm" => Some(crate::model::SpacerHeight::Sm),
        "md" => Some(crate::model::SpacerHeight::Md),
        "lg" => Some(crate::model::SpacerHeight::Lg),
        "xl" => Some(crate::model::SpacerHeight::Xl),
        "xxl" => Some(crate::model::SpacerHeight::Xxl),
        _ => None,
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
        | InputMode::EditHeroTitle
        | InputMode::EditHeroSubtitle
        | InputMode::EditHeroCopy
        | InputMode::EditHeroCtaText
        | InputMode::EditHeroCtaLink
        | InputMode::EditHeroCtaText2
        | InputMode::EditHeroCtaLink2 => Some(&[
            InputMode::EditHeroImage,
            InputMode::EditHeroClass,
            InputMode::EditHeroAos,
            InputMode::EditHeroTitle,
            InputMode::EditHeroSubtitle,
            InputMode::EditHeroCopy,
            InputMode::EditHeroCtaText,
            InputMode::EditHeroCtaLink,
            InputMode::EditHeroCtaText2,
            InputMode::EditHeroCtaLink2,
        ]),
        InputMode::EditSectionId | InputMode::EditSectionTitle | InputMode::EditSectionClass => {
            Some(&[
                InputMode::EditSectionId,
                InputMode::EditSectionTitle,
                InputMode::EditSectionClass,
            ])
        }
        InputMode::EditCardTitle | InputMode::EditCardCopy => {
            Some(&[InputMode::EditCardTitle, InputMode::EditCardCopy])
        }
        InputMode::EditCtaTitle | InputMode::EditCtaLink => {
            Some(&[InputMode::EditCtaTitle, InputMode::EditCtaLink])
        }
        InputMode::EditAlertMessage | InputMode::EditAlertTitle => {
            Some(&[InputMode::EditAlertMessage, InputMode::EditAlertTitle])
        }
        InputMode::EditBannerMessage | InputMode::EditBannerLinkUrl => {
            Some(&[InputMode::EditBannerMessage, InputMode::EditBannerLinkUrl])
        }
        InputMode::EditTabsFirstTitle | InputMode::EditTabsFirstContent => Some(&[
            InputMode::EditTabsFirstTitle,
            InputMode::EditTabsFirstContent,
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
        InputMode::EditModalTitle | InputMode::EditModalContent => {
            Some(&[InputMode::EditModalTitle, InputMode::EditModalContent])
        }
        InputMode::EditSliderFirstTitle | InputMode::EditSliderFirstCopy => Some(&[
            InputMode::EditSliderFirstTitle,
            InputMode::EditSliderFirstCopy,
        ]),
        InputMode::EditTimelineFirstTitle | InputMode::EditTimelineFirstDescription => Some(&[
            InputMode::EditTimelineFirstTitle,
            InputMode::EditTimelineFirstDescription,
        ]),
        InputMode::EditSpacerHeight => Some(&[InputMode::EditSpacerHeight]),
        _ => None,
    }
}

fn help_text() -> String {
    [
        "Global:",
        "  F1: Open/close this help",
        "  q: Quit",
        "  s: Open save modal and enter file path",
        "  Tab / Shift+Tab: Next/previous page",
        "",
        "Node navigation and edits:",
        "  Up/Down or mouse wheel: Select row in Nodes tree",
        "  Enter: Edit selected row",
        "  Space: Expand/collapse selected section or accordion items",
        "  /: Open insert fuzzy finder (hero/section/components)",
        "  A / X: Add/remove dd-accordion item on selected accordion",
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
        "  Tab / Shift+Tab: Next/previous editable field for selected hero/section",
        "  Left / Right: Cycle section/hero/accordion option fields when active",
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
            Self::Card,
            Self::Alert,
            Self::Banner,
            Self::Tabs,
            Self::Accordion,
            Self::Cta,
            Self::Modal,
            Self::Slider,
            Self::Spacer,
            Self::Timeline,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            ComponentKind::Hero => "dd-hero",
            ComponentKind::Section => "dd-section",
            ComponentKind::Card => "dd-card",
            ComponentKind::Alert => "dd-alert",
            ComponentKind::Banner => "dd-banner",
            ComponentKind::Tabs => "dd-tabs",
            ComponentKind::Accordion => "dd-accordion",
            ComponentKind::Cta => "dd-cta",
            ComponentKind::Modal => "dd-modal",
            ComponentKind::Slider => "dd-slider",
            ComponentKind::Spacer => "dd-spacer",
            ComponentKind::Timeline => "dd-timeline",
        }
    }

    fn default_component(self) -> crate::model::SectionComponent {
        match self {
            ComponentKind::Hero | ComponentKind::Section => {
                unreachable!("top-level kinds do not map to section components")
            }
            ComponentKind::Card => crate::model::SectionComponent::Card(crate::model::DdCard {
                title: "New Card".to_string(),
                image: "/assets/images/card.jpg".to_string(),
                subtitle: None,
                copy: Some("Card copy".to_string()),
                cta_text: None,
                cta_link: None,
                image_alt: Some("Card image".to_string()),
                columns: Some(crate::model::CardColumns::Three),
                animate: Some(crate::model::CardAnimate::FadeUp),
            }),
            ComponentKind::Alert => crate::model::SectionComponent::Alert(crate::model::DdAlert {
                alert_type: crate::model::AlertType::Info,
                message: "Informational message".to_string(),
                title: Some("Notice".to_string()),
                dismissible: Some(false),
            }),
            ComponentKind::Banner => {
                crate::model::SectionComponent::Banner(crate::model::DdBanner {
                    message: "Banner message".to_string(),
                    background: "#ffca76".to_string(),
                    link_text: None,
                    link_url: None,
                    dismissible: Some(false),
                })
            }
            ComponentKind::Tabs => crate::model::SectionComponent::Tabs(crate::model::DdTabs {
                tabs: vec![
                    crate::model::TabItem {
                        title: "Tab One".to_string(),
                        content: "First tab content".to_string(),
                    },
                    crate::model::TabItem {
                        title: "Tab Two".to_string(),
                        content: "Second tab content".to_string(),
                    },
                ],
                default_tab: Some(0),
                orientation: Some(crate::model::TabsOrientation::Horizontal),
            }),
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
            ComponentKind::Cta => crate::model::SectionComponent::Cta(crate::model::DdCta {
                title: "Ready to continue?".to_string(),
                copy: "Call to action copy".to_string(),
                cta_text: "Continue".to_string(),
                cta_link: "/continue".to_string(),
            }),
            ComponentKind::Modal => crate::model::SectionComponent::Modal(crate::model::DdModal {
                trigger_text: "Open modal".to_string(),
                title: "Modal title".to_string(),
                content: "Modal content".to_string(),
            }),
            ComponentKind::Slider => {
                crate::model::SectionComponent::Slider(crate::model::DdSlider {
                    slides: vec![crate::model::SlideItem {
                        image: "/assets/images/slide-1.jpg".to_string(),
                        title: "Slide One".to_string(),
                        copy: "Slide copy".to_string(),
                    }],
                    autoplay: Some(false),
                    speed: Some(400),
                })
            }
            ComponentKind::Spacer => {
                crate::model::SectionComponent::Spacer(crate::model::DdSpacer {
                    height: crate::model::SpacerHeight::Md,
                })
            }
            ComponentKind::Timeline => {
                crate::model::SectionComponent::Timeline(crate::model::DdTimeline {
                    events: vec![crate::model::TimelineEvent {
                        date: "2026-02-19".to_string(),
                        title: "Milestone".to_string(),
                        description: "Timeline event description".to_string(),
                    }],
                })
            }
        }
    }
}
