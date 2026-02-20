use std::io;
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
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::model::{PageNode, Site};
use crate::storage::save_site;

pub fn run_tui(site: Site, path: Option<PathBuf>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(site, path);
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
    selected_page: usize,
    selected_node: usize,
    selected_component: usize,
    selected_nested_item: usize,
    list_area: Rect,
    status: String,
    path: Option<PathBuf>,
    should_quit: bool,
    input_mode: Option<InputMode>,
    input_buffer: String,
    component_kind: ComponentKind,
}

#[derive(Clone, Copy)]
enum InputMode {
    EditHeroTitle,
    EditHeroSubtitle,
    EditSectionId,
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

impl App {
    fn new(site: Site, path: Option<PathBuf>) -> Self {
        Self {
            site,
            selected_page: 0,
            selected_node: 0,
            selected_component: 0,
            selected_nested_item: 0,
            list_area: Rect::default(),
            status: "q quit | s save | tab page | h/n add hero/section | d delete | J/K move nodes | </> move component | {/} move nested item | [/ ] pick component kind | a add component | x remove component | ,/. select section component | j/k select nested item | i/o add/remove nested item | m/l edit fields | b/w/g/t section layout | e/u edit hero".to_string(),
            path,
            should_quit: false,
            input_mode: None,
            input_buffer: String::new(),
            component_kind: ComponentKind::Card,
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
                Constraint::Length(2),
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(root[1]);

        let header = Paragraph::new(format!(
            "Page: {} ({}/{})",
            page.title,
            self.selected_page + 1,
            self.site.pages.len()
        ));
        frame.render_widget(header, root[0]);

        let node_lines = page
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, n)| ListItem::new(format!("{} {}", idx + 1, node_label(n))))
            .collect::<Vec<_>>();
        let list = List::new(node_lines)
            .block(Block::default().title("Nodes").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !page.nodes.is_empty() {
            state.select(Some(self.selected_node.min(page.nodes.len() - 1)));
        }
        frame.render_stateful_widget(list, main[0], &mut state);
        self.list_area = main[0];

        let details = Paragraph::new(self.details_text())
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, main[1]);

        let footer_text = if self.input_mode.is_some() {
            format!("{} | {}", self.status, self.input_buffer)
        } else {
            self.status.clone()
        };
        let footer = Paragraph::new(footer_text)
            .block(Block::default().title("Status").borders(Borders::ALL));
        frame.render_widget(footer, root[2]);
    }

    fn handle_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if self.input_mode.is_some() {
            return self.handle_input_mode(evt);
        }
        match evt {
            Event::Key(k) => match k.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Up => self.select_prev(),
                KeyCode::Down => self.select_next(),
                KeyCode::Tab => self.select_next_page(),
                KeyCode::BackTab => self.select_prev_page(),
                KeyCode::Char('s') => self.save()?,
                KeyCode::Char('h') => self.add_hero(),
                KeyCode::Char('n') => self.add_section(),
                KeyCode::Char('d') => self.delete_selected_node(),
                KeyCode::Char('J') => self.move_selected_down(),
                KeyCode::Char('K') => self.move_selected_up(),
                KeyCode::Char('a') => self.add_selected_component_to_section(),
                KeyCode::Char('x') => self.remove_last_component_from_selected_section(),
                KeyCode::Char(']') => self.next_component_kind(),
                KeyCode::Char('[') => self.prev_component_kind(),
                KeyCode::Char('b') => self.cycle_section_background(),
                KeyCode::Char('w') => self.cycle_section_width(),
                KeyCode::Char('g') => self.cycle_section_spacing(),
                KeyCode::Char('t') => self.cycle_section_align(),
                KeyCode::Char(',') => self.select_prev_component(),
                KeyCode::Char('.') => self.select_next_component(),
                KeyCode::Char('<') => self.move_selected_component_up(),
                KeyCode::Char('>') => self.move_selected_component_down(),
                KeyCode::Char('j') => self.select_prev_nested_item(),
                KeyCode::Char('k') => self.select_next_nested_item(),
                KeyCode::Char('{') => self.move_selected_nested_item_up(),
                KeyCode::Char('}') => self.move_selected_nested_item_down(),
                KeyCode::Char('i') => self.add_nested_item_to_selected_component(),
                KeyCode::Char('o') => self.remove_nested_item_from_selected_component(),
                KeyCode::Char('e') => self.begin_edit_selected(),
                KeyCode::Char('u') => self.begin_edit_hero_subtitle(),
                KeyCode::Char('m') => self.begin_edit_selected_component_primary(),
                KeyCode::Char('l') => self.begin_edit_selected_component_secondary(),
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
        Ok(())
    }

    fn handle_input_mode(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = None;
                    self.input_buffer.clear();
                    self.status = "Edit cancelled.".to_string();
                }
                KeyCode::Enter => {
                    self.commit_input_edit();
                }
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

    fn begin_edit_selected(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                Some(match &page.nodes[idx] {
                    PageNode::Hero(v) => (InputMode::EditHeroTitle, v.title.clone()),
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
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionId => {
                "Editing section id. Enter to save, esc to cancel.".to_string()
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

    fn begin_edit_hero_subtitle(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[idx] {
                    PageNode::Hero(v) => Some((InputMode::EditHeroSubtitle, v.subtitle.clone())),
                    PageNode::Section(_) => None,
                }
            }
        };
        let Some((mode, value)) = selected else {
            self.status = "Selected node is not a hero.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.status = "Editing hero subtitle. Enter to save, esc to cancel.".to_string();
    }

    fn commit_input_edit(&mut self) {
        let Some(mode) = self.input_mode else {
            return;
        };
        let value = self.input_buffer.trim().to_string();
        if value.is_empty() {
            self.status = "Value cannot be empty.".to_string();
            return;
        }
        let selected = self.selected_node;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            self.status = "No page available.".to_string();
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No node selected.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match (&mut page.nodes[idx], mode) {
            (PageNode::Hero(v), InputMode::EditHeroTitle) => {
                v.title = value;
                self.status = "Updated hero title.".to_string();
            }
            (PageNode::Hero(v), InputMode::EditHeroSubtitle) => {
                v.subtitle = value;
                self.status = "Updated hero subtitle.".to_string();
            }
            (PageNode::Section(v), InputMode::EditSectionId) => {
                v.id = value;
                self.status = "Updated section id.".to_string();
            }
            (PageNode::Section(v), InputMode::EditCardTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        card.title = value;
                        self.status = "Updated dd-card title.".to_string();
                    } else {
                        self.status = "Selected component is not dd-card.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditCardCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.components[ci] {
                        card.copy = Some(value);
                        self.status = "Updated dd-card copy.".to_string();
                    } else {
                        self.status = "Selected component is not dd-card.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditCtaTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.title = value;
                        self.status = "Updated dd-cta title.".to_string();
                    } else {
                        self.status = "Selected component is not dd-cta.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLink) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.components[ci] {
                        cta.cta_link = value;
                        self.status = "Updated dd-cta link.".to_string();
                    } else {
                        self.status = "Selected component is not dd-cta.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditAlertMessage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alert(alert) = &mut v.components[ci] {
                        alert.message = value;
                        self.status = "Updated dd-alert message.".to_string();
                    } else {
                        self.status = "Selected component is not dd-alert.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditAlertTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alert(alert) = &mut v.components[ci] {
                        alert.title = Some(value);
                        self.status = "Updated dd-alert title.".to_string();
                    } else {
                        self.status = "Selected component is not dd-alert.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditBannerMessage) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.message = value;
                        self.status = "Updated dd-banner message.".to_string();
                    } else {
                        self.status = "Selected component is not dd-banner.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditBannerLinkUrl) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.components[ci] {
                        banner.link_url = Some(value);
                        self.status = "Updated dd-banner link_url.".to_string();
                    } else {
                        self.status = "Selected component is not dd-banner.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditTabsFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Tabs(tabs) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tabs.tabs.len(), selected_nested_item) {
                            tabs.tabs[ni].title = value;
                            self.status = format!("Updated dd-tabs item {} title.", ni + 1);
                        } else {
                            self.status = "dd-tabs has no items.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-tabs.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditTabsFirstContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Tabs(tabs) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tabs.tabs.len(), selected_nested_item) {
                            tabs.tabs[ni].content = value;
                            self.status = format!("Updated dd-tabs item {} content.", ni + 1);
                        } else {
                            self.status = "dd-tabs has no items.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-tabs.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].title = value;
                            self.status = format!("Updated dd-accordion item {} title.", ni + 1);
                        } else {
                            self.status = "dd-accordion has no items.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-accordion.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].content = value;
                            self.status = format!("Updated dd-accordion item {} content.", ni + 1);
                        } else {
                            self.status = "dd-accordion has no items.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-accordion.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditModalTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.components[ci] {
                        modal.title = value;
                        self.status = "Updated dd-modal title.".to_string();
                    } else {
                        self.status = "Selected component is not dd-modal.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditModalContent) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.components[ci] {
                        modal.content = value;
                        self.status = "Updated dd-modal content.".to_string();
                    } else {
                        self.status = "Selected component is not dd-modal.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditSliderFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(slider.slides.len(), selected_nested_item) {
                            slider.slides[ni].title = value;
                            self.status = format!("Updated dd-slider slide {} title.", ni + 1);
                        } else {
                            self.status = "dd-slider has no slides.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-slider.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditSliderFirstCopy) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(slider.slides.len(), selected_nested_item) {
                            slider.slides[ni].copy = value;
                            self.status = format!("Updated dd-slider slide {} copy.", ni + 1);
                        } else {
                            self.status = "dd-slider has no slides.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-slider.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditSpacerHeight) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Spacer(spacer) = &mut v.components[ci] {
                        if let Some(height) = parse_spacer_height(&value) {
                            spacer.height = height;
                            self.status = "Updated dd-spacer height.".to_string();
                        } else {
                            self.status = "Invalid spacer height. Use sm|md|lg|xl|xxl.".to_string();
                            return;
                        }
                    } else {
                        self.status = "Selected component is not dd-spacer.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditTimelineFirstTitle) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Timeline(tl) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tl.events.len(), selected_nested_item) {
                            tl.events[ni].title = value;
                            self.status = format!("Updated dd-timeline event {} title.", ni + 1);
                        } else {
                            self.status = "dd-timeline has no events.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-timeline.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            (PageNode::Section(v), InputMode::EditTimelineFirstDescription) => {
                if let Some(ci) = component_index(v.components.len(), selected_component) {
                    if let crate::model::SectionComponent::Timeline(tl) = &mut v.components[ci] {
                        if let Some(ni) = nested_index(tl.events.len(), selected_nested_item) {
                            tl.events[ni].description = value;
                            self.status =
                                format!("Updated dd-timeline event {} description.", ni + 1);
                        } else {
                            self.status = "dd-timeline has no events.".to_string();
                        }
                    } else {
                        self.status = "Selected component is not dd-timeline.".to_string();
                    }
                } else {
                    self.status = "Section has no components.".to_string();
                }
            }
            _ => {
                self.status = "Edit type no longer matches selected node.".to_string();
            }
        }
        self.input_mode = None;
        self.input_buffer.clear();
    }

    fn handle_click(&mut self, x: u16, y: u16) {
        if !contains(self.list_area, x, y) {
            return;
        }
        let page = self.current_page();
        if page.nodes.is_empty() {
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
        if idx < page.nodes.len() {
            self.selected_node = idx;
            self.selected_component = 0;
            self.selected_nested_item = 0;
            self.status = format!("Selected node {}", idx + 1);
        }
    }

    fn save(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            save_site(path, &self.site)?;
            self.status = format!("Saved {}", path.display());
        } else {
            self.status = "No save path set for this session.".to_string();
        }
        Ok(())
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

    fn select_prev(&mut self) {
        if self.current_page().nodes.is_empty() {
            return;
        }
        let next = self.selected_node.saturating_sub(1);
        if next != self.selected_node {
            self.selected_node = next;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
    }

    fn select_next(&mut self) {
        let total = self.current_page().nodes.len();
        if total == 0 {
            return;
        }
        let next = (self.selected_node + 1).min(total - 1);
        if next != self.selected_node {
            self.selected_node = next;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
    }

    fn select_next_page(&mut self) {
        if self.site.pages.is_empty() {
            return;
        }
        self.selected_page = (self.selected_page + 1) % self.site.pages.len();
        self.selected_node = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
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
        self.selected_component = 0;
        self.selected_nested_item = 0;
    }

    fn details_text(&self) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "No nodes on this page.".to_string();
        }
        let idx = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[idx] {
            PageNode::Hero(v) => format!(
                "Type: dd-hero\nTitle: {}\nSubtitle: {}\nImage: {}\nCTA: {} -> {}\n",
                v.title,
                v.subtitle,
                v.image,
                v.cta_text.as_deref().unwrap_or("(none)"),
                v.cta_link.as_deref().unwrap_or("(none)")
            ),
            PageNode::Section(v) => format!(
                "Type: dd-section\nId: {}\nBackground: {:?}\nSpacing: {:?}\nWidth: {:?}\nAlign: {:?}\nComponent count: {}\nActive component: {}\nComponents: {}\nComponent details:\n{}\nInsert mode: {}\n",
                v.id,
                v.background,
                v.spacing,
                v.width,
                v.align,
                v.components.len(),
                active_component_label(v, self.selected_component),
                section_component_summary(v),
                selected_component_details(v, self.selected_component, self.selected_nested_item),
                self.component_kind.label()
            ),
        }
    }

    fn add_hero(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let hero = crate::model::DdHero {
            image: "/assets/images/hero-new.jpg".to_string(),
            title: "New Hero".to_string(),
            subtitle: "Add subtitle".to_string(),
            copy: None,
            cta_text: None,
            cta_link: None,
            cta_target: Some(crate::model::CtaTarget::SelfTarget),
            image_alt: Some("Hero image".to_string()),
            image_mobile: None,
            image_tablet: None,
            image_desktop: None,
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Hero(hero));
        self.selected_node = idx;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Inserted dd-hero at position {}.", idx + 1);
    }

    fn add_section(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let next_num = page.nodes.len() + 1;
        let section = crate::model::DdSection {
            id: format!("section-{}", next_num),
            background: crate::model::SectionBackground::White,
            spacing: crate::model::SectionSpacing::Normal,
            width: crate::model::SectionWidth::Normal,
            align: crate::model::SectionAlign::Left,
            components: Vec::new(),
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Section(section));
        self.selected_node = idx;
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
            self.selected_component = 0;
            self.selected_nested_item = 0;
        } else {
            self.selected_node = idx.min(page.nodes.len() - 1);
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
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = "Moved node down.".to_string();
    }

    fn add_selected_component_to_section(&mut self) {
        let kind = self.component_kind;
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                let inserted = kind.default_component();
                section.components.push(inserted);
                self.selected_component = section.components.len().saturating_sub(1);
                self.selected_nested_item = 0;
                self.status = format!("Added {} to selected section.", kind.label());
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn remove_last_component_from_selected_section(&mut self) {
        let prev_selected_component = self.selected_component;
        let selected = self.selected_node;
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
                if section.components.pop().is_some() {
                    let new_selected_component = if section.components.is_empty() {
                        0
                    } else {
                        prev_selected_component.min(section.components.len() - 1)
                    };
                    if section.components.is_empty() {
                        (
                            Some(new_selected_component),
                            "Removed last component from selected section.".to_string(),
                        )
                    } else {
                        (
                            Some(new_selected_component),
                            format!(
                                "Removed last component. Selected component {} of {}.",
                                new_selected_component + 1,
                                section.components.len()
                            ),
                        )
                    }
                } else {
                    (None, "Section has no components to remove.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(new_selected_component) = result.0 {
            self.selected_component = new_selected_component;
            self.selected_nested_item = 0;
        }
        self.status = result.1;
    }

    fn next_component_kind(&mut self) {
        self.component_kind = self.component_kind.next();
        self.status = format!("Component insert mode: {}", self.component_kind.label());
    }

    fn prev_component_kind(&mut self) {
        self.component_kind = self.component_kind.prev();
        self.status = format!("Component insert mode: {}", self.component_kind.label());
    }

    fn cycle_section_background(&mut self) {
        self.mutate_selected_section(
            |s| {
                s.background = match s.background {
                    crate::model::SectionBackground::Primary => {
                        crate::model::SectionBackground::Secondary
                    }
                    crate::model::SectionBackground::Secondary => {
                        crate::model::SectionBackground::Tertiary
                    }
                    crate::model::SectionBackground::Tertiary => {
                        crate::model::SectionBackground::Gray
                    }
                    crate::model::SectionBackground::Gray => crate::model::SectionBackground::White,
                    crate::model::SectionBackground::White => {
                        crate::model::SectionBackground::Black
                    }
                    crate::model::SectionBackground::Black => {
                        crate::model::SectionBackground::Primary
                    }
                };
            },
            "Cycled section background.",
        );
    }

    fn cycle_section_spacing(&mut self) {
        self.mutate_selected_section(
            |s| {
                s.spacing = match s.spacing {
                    crate::model::SectionSpacing::Tight => crate::model::SectionSpacing::Normal,
                    crate::model::SectionSpacing::Normal => crate::model::SectionSpacing::Loose,
                    crate::model::SectionSpacing::Loose => crate::model::SectionSpacing::ExtraLoose,
                    crate::model::SectionSpacing::ExtraLoose => crate::model::SectionSpacing::Tight,
                };
            },
            "Cycled section spacing.",
        );
    }

    fn cycle_section_width(&mut self) {
        self.mutate_selected_section(
            |s| {
                s.width = match s.width {
                    crate::model::SectionWidth::Narrow => crate::model::SectionWidth::Normal,
                    crate::model::SectionWidth::Normal => crate::model::SectionWidth::Wide,
                    crate::model::SectionWidth::Wide => crate::model::SectionWidth::Full,
                    crate::model::SectionWidth::Full => crate::model::SectionWidth::Narrow,
                };
            },
            "Cycled section width.",
        );
    }

    fn cycle_section_align(&mut self) {
        self.mutate_selected_section(
            |s| {
                s.align = match s.align {
                    crate::model::SectionAlign::Left => crate::model::SectionAlign::Center,
                    crate::model::SectionAlign::Center => crate::model::SectionAlign::Right,
                    crate::model::SectionAlign::Right => crate::model::SectionAlign::Left,
                };
            },
            "Cycled section alignment.",
        );
    }

    fn mutate_selected_section<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdSection),
    {
        let prev_selected_component = self.selected_component;
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                mutator(section);
                self.selected_component =
                    prev_selected_component.min(section.components.len().saturating_sub(1));
                self.status = success_message.to_string();
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn select_prev_component(&mut self) {
        let total = match self.selected_section_component_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no components.".to_string();
            return;
        }
        self.selected_component = self.selected_component.saturating_sub(1);
        self.selected_nested_item = 0;
        self.status = format!(
            "Selected component {} of {}.",
            self.selected_component + 1,
            total
        );
    }

    fn select_next_component(&mut self) {
        let total = match self.selected_section_component_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no components.".to_string();
            return;
        }
        self.selected_component = (self.selected_component + 1).min(total - 1);
        self.selected_nested_item = 0;
        self.status = format!(
            "Selected component {} of {}.",
            self.selected_component + 1,
            total
        );
    }

    fn move_selected_component_up(&mut self) {
        let selected = self.selected_node;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                if section.components.len() < 2 {
                    self.status = "Need at least 2 components to reorder.".to_string();
                    return;
                }
                let ci = selected_component.min(section.components.len() - 1);
                if ci == 0 {
                    self.status = "Component is already first.".to_string();
                    return;
                }
                section.components.swap(ci, ci - 1);
                self.selected_component = ci - 1;
                self.selected_nested_item = 0;
                self.status = "Moved component up.".to_string();
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn move_selected_component_down(&mut self) {
        let selected = self.selected_node;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                if section.components.len() < 2 {
                    self.status = "Need at least 2 components to reorder.".to_string();
                    return;
                }
                let ci = selected_component.min(section.components.len() - 1);
                if ci + 1 >= section.components.len() {
                    self.status = "Component is already last.".to_string();
                    return;
                }
                section.components.swap(ci, ci + 1);
                self.selected_component = ci + 1;
                self.selected_nested_item = 0;
                self.status = "Moved component down.".to_string();
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn select_prev_nested_item(&mut self) {
        let total = match self.selected_nested_item_total() {
            Some(v) => v,
            None => {
                self.status = "Selected component has no nested items.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected component has no nested items.".to_string();
            return;
        }
        self.selected_nested_item = self.selected_nested_item.saturating_sub(1);
        self.status = format!(
            "Selected nested item {} of {}.",
            self.selected_nested_item + 1,
            total
        );
    }

    fn select_next_nested_item(&mut self) {
        let total = match self.selected_nested_item_total() {
            Some(v) => v,
            None => {
                self.status = "Selected component has no nested items.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected component has no nested items.".to_string();
            return;
        }
        self.selected_nested_item = (self.selected_nested_item + 1).min(total - 1);
        self.status = format!(
            "Selected nested item {} of {}.",
            self.selected_nested_item + 1,
            total
        );
    }

    fn move_selected_nested_item_up(&mut self) {
        let selected = self.selected_node;
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
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                let Some(ci) = component_index(section.components.len(), selected_component) else {
                    self.status = "Section has no components.".to_string();
                    return;
                };
                match &mut section.components[ci] {
                    crate::model::SectionComponent::Tabs(tabs) => {
                        if tabs.tabs.len() < 2 {
                            self.status = "Need at least 2 tabs to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(tabs.tabs.len() - 1);
                        if i == 0 {
                            self.status = "Nested item is already first.".to_string();
                            return;
                        }
                        tabs.tabs.swap(i, i - 1);
                        self.selected_nested_item = i - 1;
                        self.status = "Moved tab item up.".to_string();
                    }
                    crate::model::SectionComponent::Accordion(acc) => {
                        if acc.items.len() < 2 {
                            self.status = "Need at least 2 accordion items to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(acc.items.len() - 1);
                        if i == 0 {
                            self.status = "Nested item is already first.".to_string();
                            return;
                        }
                        acc.items.swap(i, i - 1);
                        self.selected_nested_item = i - 1;
                        self.status = "Moved accordion item up.".to_string();
                    }
                    crate::model::SectionComponent::Slider(slider) => {
                        if slider.slides.len() < 2 {
                            self.status = "Need at least 2 slides to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(slider.slides.len() - 1);
                        if i == 0 {
                            self.status = "Nested item is already first.".to_string();
                            return;
                        }
                        slider.slides.swap(i, i - 1);
                        self.selected_nested_item = i - 1;
                        self.status = "Moved slide up.".to_string();
                    }
                    crate::model::SectionComponent::Timeline(tl) => {
                        if tl.events.len() < 2 {
                            self.status = "Need at least 2 events to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(tl.events.len() - 1);
                        if i == 0 {
                            self.status = "Nested item is already first.".to_string();
                            return;
                        }
                        tl.events.swap(i, i - 1);
                        self.selected_nested_item = i - 1;
                        self.status = "Moved timeline event up.".to_string();
                    }
                    _ => {
                        self.status =
                            "Nested reorder supported for tabs/accordion/slider/timeline."
                                .to_string();
                    }
                }
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn move_selected_nested_item_down(&mut self) {
        let selected = self.selected_node;
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
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                let Some(ci) = component_index(section.components.len(), selected_component) else {
                    self.status = "Section has no components.".to_string();
                    return;
                };
                match &mut section.components[ci] {
                    crate::model::SectionComponent::Tabs(tabs) => {
                        if tabs.tabs.len() < 2 {
                            self.status = "Need at least 2 tabs to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(tabs.tabs.len() - 1);
                        if i + 1 >= tabs.tabs.len() {
                            self.status = "Nested item is already last.".to_string();
                            return;
                        }
                        tabs.tabs.swap(i, i + 1);
                        self.selected_nested_item = i + 1;
                        self.status = "Moved tab item down.".to_string();
                    }
                    crate::model::SectionComponent::Accordion(acc) => {
                        if acc.items.len() < 2 {
                            self.status = "Need at least 2 accordion items to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(acc.items.len() - 1);
                        if i + 1 >= acc.items.len() {
                            self.status = "Nested item is already last.".to_string();
                            return;
                        }
                        acc.items.swap(i, i + 1);
                        self.selected_nested_item = i + 1;
                        self.status = "Moved accordion item down.".to_string();
                    }
                    crate::model::SectionComponent::Slider(slider) => {
                        if slider.slides.len() < 2 {
                            self.status = "Need at least 2 slides to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(slider.slides.len() - 1);
                        if i + 1 >= slider.slides.len() {
                            self.status = "Nested item is already last.".to_string();
                            return;
                        }
                        slider.slides.swap(i, i + 1);
                        self.selected_nested_item = i + 1;
                        self.status = "Moved slide down.".to_string();
                    }
                    crate::model::SectionComponent::Timeline(tl) => {
                        if tl.events.len() < 2 {
                            self.status = "Need at least 2 events to reorder.".to_string();
                            return;
                        }
                        let i = selected_nested_item.min(tl.events.len() - 1);
                        if i + 1 >= tl.events.len() {
                            self.status = "Nested item is already last.".to_string();
                            return;
                        }
                        tl.events.swap(i, i + 1);
                        self.selected_nested_item = i + 1;
                        self.status = "Moved timeline event down.".to_string();
                    }
                    _ => {
                        self.status =
                            "Nested reorder supported for tabs/accordion/slider/timeline."
                                .to_string();
                    }
                }
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn add_nested_item_to_selected_component(&mut self) {
        let selected = self.selected_node;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                let Some(ci) = component_index(section.components.len(), selected_component) else {
                    self.status = "Section has no components.".to_string();
                    return;
                };
                match &mut section.components[ci] {
                    crate::model::SectionComponent::Tabs(tabs) => {
                        let next = tabs.tabs.len() + 1;
                        tabs.tabs.push(crate::model::TabItem {
                            title: format!("Tab {}", next),
                            content: "Tab content".to_string(),
                        });
                        self.selected_nested_item = tabs.tabs.len().saturating_sub(1);
                        self.status = format!("Added tab item {}.", next);
                    }
                    crate::model::SectionComponent::Accordion(acc) => {
                        let next = acc.items.len() + 1;
                        acc.items.push(crate::model::AccordionItem {
                            title: format!("Item {}", next),
                            content: "Accordion content".to_string(),
                        });
                        self.selected_nested_item = acc.items.len().saturating_sub(1);
                        self.status = format!("Added accordion item {}.", next);
                    }
                    crate::model::SectionComponent::Slider(slider) => {
                        let next = slider.slides.len() + 1;
                        slider.slides.push(crate::model::SlideItem {
                            image: format!("/assets/images/slide-{}.jpg", next),
                            title: format!("Slide {}", next),
                            copy: "Slide copy".to_string(),
                        });
                        self.selected_nested_item = slider.slides.len().saturating_sub(1);
                        self.status = format!("Added slide {}.", next);
                    }
                    crate::model::SectionComponent::Timeline(tl) => {
                        let next = tl.events.len() + 1;
                        tl.events.push(crate::model::TimelineEvent {
                            date: "2026-02-20".to_string(),
                            title: format!("Event {}", next),
                            description: "Timeline event description".to_string(),
                        });
                        self.selected_nested_item = tl.events.len().saturating_sub(1);
                        self.status = format!("Added timeline event {}.", next);
                    }
                    _ => {
                        self.status =
                            "Nested item add supported for tabs/accordion/slider/timeline."
                                .to_string();
                    }
                }
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
    }

    fn remove_nested_item_from_selected_component(&mut self) {
        let prev_selected_nested_item = self.selected_nested_item;
        let selected = self.selected_node;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                let Some(ci) = component_index(section.components.len(), selected_component) else {
                    self.status = "Section has no components.".to_string();
                    return;
                };
                let result = match &mut section.components[ci] {
                    crate::model::SectionComponent::Tabs(tabs) => {
                        if tabs.tabs.pop().is_some() {
                            let next_selected =
                                prev_selected_nested_item.min(tabs.tabs.len().saturating_sub(1));
                            (
                                Some(next_selected),
                                format!("Removed tab item. {} remaining.", tabs.tabs.len()),
                            )
                        } else {
                            (None, "No tab items to remove.".to_string())
                        }
                    }
                    crate::model::SectionComponent::Accordion(acc) => {
                        if acc.items.pop().is_some() {
                            let next_selected =
                                prev_selected_nested_item.min(acc.items.len().saturating_sub(1));
                            (
                                Some(next_selected),
                                format!("Removed accordion item. {} remaining.", acc.items.len()),
                            )
                        } else {
                            (None, "No accordion items to remove.".to_string())
                        }
                    }
                    crate::model::SectionComponent::Slider(slider) => {
                        if slider.slides.pop().is_some() {
                            let next_selected = prev_selected_nested_item
                                .min(slider.slides.len().saturating_sub(1));
                            (
                                Some(next_selected),
                                format!("Removed slide. {} remaining.", slider.slides.len()),
                            )
                        } else {
                            (None, "No slides to remove.".to_string())
                        }
                    }
                    crate::model::SectionComponent::Timeline(tl) => {
                        if tl.events.pop().is_some() {
                            let next_selected =
                                prev_selected_nested_item.min(tl.events.len().saturating_sub(1));
                            (
                                Some(next_selected),
                                format!("Removed timeline event. {} remaining.", tl.events.len()),
                            )
                        } else {
                            (None, "No timeline events to remove.".to_string())
                        }
                    }
                    _ => (
                        None,
                        "Nested item remove supported for tabs/accordion/slider/timeline."
                            .to_string(),
                    ),
                };
                if let Some(next_selected) = result.0 {
                    self.selected_nested_item = next_selected;
                }
                self.status = result.1;
            }
            _ => self.status = "Selected node is not a section.".to_string(),
        }
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
                        if let Some(ci) =
                            component_index(section.components.len(), self.selected_component)
                        {
                            match &section.components[ci] {
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
                                crate::model::SectionComponent::Accordion(acc) => {
                                    if let Some(ni) =
                                        nested_index(acc.items.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditAccordionFirstTitle,
                                            acc.items[ni].title.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
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

    fn begin_edit_selected_component_secondary(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let ni = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[ni] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        if let Some(ci) =
                            component_index(section.components.len(), self.selected_component)
                        {
                            match &section.components[ci] {
                                crate::model::SectionComponent::Card(card) => Some((
                                    InputMode::EditCardCopy,
                                    card.copy.clone().unwrap_or_default(),
                                )),
                                crate::model::SectionComponent::Cta(cta) => {
                                    Some((InputMode::EditCtaLink, cta.cta_link.clone()))
                                }
                                crate::model::SectionComponent::Alert(alert) => Some((
                                    InputMode::EditAlertTitle,
                                    alert.title.clone().unwrap_or_default(),
                                )),
                                crate::model::SectionComponent::Banner(banner) => Some((
                                    InputMode::EditBannerLinkUrl,
                                    banner.link_url.clone().unwrap_or_default(),
                                )),
                                crate::model::SectionComponent::Tabs(tabs) => {
                                    if let Some(ni) =
                                        nested_index(tabs.tabs.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditTabsFirstContent,
                                            tabs.tabs[ni].content.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                crate::model::SectionComponent::Accordion(acc) => {
                                    if let Some(ni) =
                                        nested_index(acc.items.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditAccordionFirstContent,
                                            acc.items[ni].content.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                crate::model::SectionComponent::Modal(modal) => {
                                    Some((InputMode::EditModalContent, modal.content.clone()))
                                }
                                crate::model::SectionComponent::Slider(slider) => {
                                    if let Some(ni) =
                                        nested_index(slider.slides.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditSliderFirstCopy,
                                            slider.slides[ni].copy.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                crate::model::SectionComponent::Spacer(_) => None,
                                crate::model::SectionComponent::Timeline(tl) => {
                                    if let Some(ni) =
                                        nested_index(tl.events.len(), self.selected_nested_item)
                                    {
                                        Some((
                                            InputMode::EditTimelineFirstDescription,
                                            tl.events[ni].description.clone(),
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
                "Secondary edit supports card/cta/alert/banner/tabs/accordion/modal/slider/timeline.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.status = match mode {
            InputMode::EditCardCopy => {
                "Editing dd-card copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLink => {
                "Editing dd-cta link. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertTitle => {
                "Editing dd-alert title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerLinkUrl => {
                "Editing dd-banner link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTabsFirstContent => {
                "Editing dd-tabs first content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstContent => {
                "Editing dd-accordion first content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalContent => {
                "Editing dd-modal content. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderFirstCopy => {
                "Editing dd-slider first slide copy. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditTimelineFirstDescription => {
                "Editing dd-timeline first description. Enter to save, esc to cancel.".to_string()
            }
            _ => "Editing component value.".to_string(),
        };
    }

    fn selected_section_component_total(&self) -> Option<usize> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[ni] {
            PageNode::Hero(_) => None,
            PageNode::Section(section) => Some(section.components.len()),
        }
    }

    fn selected_nested_item_total(&self) -> Option<usize> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len() - 1);
        let PageNode::Section(section) = &page.nodes[ni] else {
            return None;
        };
        let ci = component_index(section.components.len(), self.selected_component)?;
        match &section.components[ci] {
            crate::model::SectionComponent::Tabs(t) => Some(t.tabs.len()),
            crate::model::SectionComponent::Accordion(a) => Some(a.items.len()),
            crate::model::SectionComponent::Slider(s) => Some(s.slides.len()),
            crate::model::SectionComponent::Timeline(t) => Some(t.events.len()),
            _ => None,
        }
    }
}

fn node_label(node: &PageNode) -> &'static str {
    match node {
        PageNode::Hero(_) => "dd-hero",
        PageNode::Section(_) => "dd-section",
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

fn nested_index(total: usize, selected_nested_item: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_nested_item.min(total - 1))
    }
}

fn section_component_summary(section: &crate::model::DdSection) -> String {
    if section.components.is_empty() {
        return "(none)".to_string();
    }
    section
        .components
        .iter()
        .map(|c| match c {
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
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn active_component_label(section: &crate::model::DdSection, selected_component: usize) -> String {
    let Some(idx) = component_index(section.components.len(), selected_component) else {
        return "(none)".to_string();
    };
    format!(
        "{} ({}/{})",
        component_label(&section.components[idx]),
        idx + 1,
        section.components.len()
    )
}

fn selected_component_details(
    section: &crate::model::DdSection,
    selected_component: usize,
    selected_nested_item: usize,
) -> String {
    let Some(idx) = component_index(section.components.len(), selected_component) else {
        return "No component selected.".to_string();
    };
    let component = &section.components[idx];
    let form = component_form(component, selected_nested_item);
    let validation = component_inline_validation(component);
    if validation.is_empty() {
        format!("{form}\nValidation: OK")
    } else {
        format!("{form}\nValidation:\n{}", validation.join("\n"))
    }
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
                "fields:\n  items_count: {}\n  active_item: {}\n  active_title: {}\n  active_content: {}",
                v.items.len(),
                active,
                title,
                content
            )
        }
        crate::model::SectionComponent::Cta(v) => format!(
            "fields:\n  title: {}\n  copy: {}\n  cta_text: {}\n  cta_link: {}",
            v.title, v.copy, v.cta_text, v.cta_link
        ),
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

fn component_inline_validation(component: &crate::model::SectionComponent) -> Vec<String> {
    let mut issues = Vec::new();
    match component {
        crate::model::SectionComponent::Card(v) => {
            if v.title.trim().is_empty() {
                issues.push("- title is required".to_string());
            }
            if v.image.trim().is_empty() {
                issues.push("- image is required".to_string());
            }
        }
        crate::model::SectionComponent::Alert(v) => {
            if v.message.trim().is_empty() {
                issues.push("- message is required".to_string());
            }
        }
        crate::model::SectionComponent::Banner(v) => {
            if v.message.trim().is_empty() {
                issues.push("- message is required".to_string());
            }
            if v.background.trim().is_empty() {
                issues.push("- background is required".to_string());
            }
        }
        crate::model::SectionComponent::Tabs(v) => {
            if v.tabs.is_empty() {
                issues.push("- at least one tab is required".to_string());
            }
        }
        crate::model::SectionComponent::Accordion(v) => {
            if v.items.is_empty() {
                issues.push("- at least one accordion item is required".to_string());
            }
        }
        crate::model::SectionComponent::Cta(v) => {
            if v.title.trim().is_empty() || v.copy.trim().is_empty() || v.cta_text.trim().is_empty()
            {
                issues.push("- title/copy/cta_text are required".to_string());
            }
            if !is_valid_link(v.cta_link.as_str()) {
                issues.push("- cta_link must be /path, #anchor, http://, or https://".to_string());
            }
        }
        crate::model::SectionComponent::Modal(v) => {
            if v.trigger_text.trim().is_empty()
                || v.title.trim().is_empty()
                || v.content.trim().is_empty()
            {
                issues.push("- trigger_text/title/content are required".to_string());
            }
        }
        crate::model::SectionComponent::Slider(v) => {
            if v.slides.is_empty() {
                issues.push("- at least one slide is required".to_string());
            }
        }
        crate::model::SectionComponent::Spacer(_) => {}
        crate::model::SectionComponent::Timeline(v) => {
            if v.events.is_empty() {
                issues.push("- at least one timeline event is required".to_string());
            }
        }
    }
    issues
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

fn is_valid_link(v: &str) -> bool {
    let s = v.trim();
    !s.is_empty()
        && (s.starts_with('/')
            || s.starts_with('#')
            || s.starts_with("http://")
            || s.starts_with("https://"))
}

impl ComponentKind {
    fn label(self) -> &'static str {
        match self {
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

    fn next(self) -> Self {
        match self {
            ComponentKind::Card => ComponentKind::Alert,
            ComponentKind::Alert => ComponentKind::Banner,
            ComponentKind::Banner => ComponentKind::Tabs,
            ComponentKind::Tabs => ComponentKind::Accordion,
            ComponentKind::Accordion => ComponentKind::Cta,
            ComponentKind::Cta => ComponentKind::Modal,
            ComponentKind::Modal => ComponentKind::Slider,
            ComponentKind::Slider => ComponentKind::Spacer,
            ComponentKind::Spacer => ComponentKind::Timeline,
            ComponentKind::Timeline => ComponentKind::Card,
        }
    }

    fn prev(self) -> Self {
        match self {
            ComponentKind::Card => ComponentKind::Timeline,
            ComponentKind::Alert => ComponentKind::Card,
            ComponentKind::Banner => ComponentKind::Alert,
            ComponentKind::Tabs => ComponentKind::Banner,
            ComponentKind::Accordion => ComponentKind::Tabs,
            ComponentKind::Cta => ComponentKind::Accordion,
            ComponentKind::Modal => ComponentKind::Cta,
            ComponentKind::Slider => ComponentKind::Modal,
            ComponentKind::Spacer => ComponentKind::Slider,
            ComponentKind::Timeline => ComponentKind::Spacer,
        }
    }

    fn default_component(self) -> crate::model::SectionComponent {
        match self {
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
