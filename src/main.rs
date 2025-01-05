mod git;
mod ui;

use std::io;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{
        palette::tailwind::{GREEN, RED, SLATE, WHITE},
        Modifier, Style, Stylize,
    },
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget,
        Widget,
    },
    DefaultTerminal,
};

use git2::Repository;

use git::branch_manager::BranchManager;
use ui::controls::{Control, Controls};
use ui::status::{OperationStatus, OperationStatusType};

pub struct App<'repo> {
    branch_manager: BranchManager<'repo>,
    state: ListState,
    exit: bool,
    operation_status: OperationStatus,
    controls: Controls,
}

impl<'a> Widget for &mut App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Percentage(5),
            Constraint::Fill(1),
            Constraint::Percentage(5),
        ])
        .areas(area);

        let [title_area, status_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Fill(1)])
                .areas(header_area);

        self.render_header(title_area, buf);
        self.render_status(status_area, buf);
        self.render_body(main_area, buf);
        self.render_footer(footer_area, buf);
    }
}

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const BRANCH_EMOJI: &str = "";
const BRANCH_EMOJI_WITH_SPACE: &str = " ";

impl<'repo> App<'repo> {
    fn new(repo: &'repo Repository) -> Result<Self, git2::Error> {
        let branch_manager = BranchManager::new(repo)?;

        Ok(App {
            state: ListState::default().with_selected(Some(0)),
            exit: false,
            branch_manager,
            operation_status: OperationStatus::default(),
            controls: Controls::new(),
        })
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    fn render_header(&mut self, area: Rect, buf: &mut Buffer) {
        let current_branch = match self.branch_manager.get_current_branch() {
            Ok(name) => name,
            Err(_) => "ERROR".to_string(),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(WHITE));

        Paragraph::new(format!(
            "Git Branch Explorer ({} {})",
            BRANCH_EMOJI, current_branch
        ))
        .block(block)
        .bold()
        .alignment(Alignment::Center)
        .fg(GREEN.c400)
        .render(area, buf);
    }

    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        if self.operation_status.is_expired_or_empty() {
            self.operation_status = OperationStatus::default();
        }

        let style = match self.operation_status.status_type {
            OperationStatusType::Info => Style::default().fg(WHITE),
            OperationStatusType::Success => Style::default().fg(GREEN.c400),
            OperationStatusType::Error => Style::default().fg(RED.c500),
        };

        let emoji = self.operation_status.status_type.get_emoji();
        let status_text = if self.operation_status.message.is_empty() {
            "Ready".to_string()
        } else {
            format!("{} {}", emoji, self.operation_status.message)
        };

        let block = Block::default().borders(Borders::ALL).border_style(style);

        Paragraph::new(status_text)
            .style(style)
            .block(block)
            .centered()
            .render(area, buf);
    }

    fn render_footer(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.controls.format_help())
            .bold()
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    fn render_body(&mut self, area: Rect, buf: &mut Buffer) {
        match self.branch_manager.get_all_local_branch_names() {
            Ok(branches) => {
                let current_branch_name = match self.branch_manager.get_current_branch() {
                    Ok(name) => name,
                    Err(_) => String::from("None"),
                };

                let items = branches
                    .iter()
                    .enumerate()
                    .map(|(i, branch_name)| {
                        let bg_colour = if i % 2 == 0 { SLATE.c950 } else { SLATE.c900 };
                        let text_colour = if branch_name == &current_branch_name {
                            GREEN.c400
                        } else {
                            WHITE
                        };
                        ListItem::new(branch_name.clone())
                            .bg(bg_colour)
                            .fg(text_colour)
                    })
                    .collect::<Vec<ListItem>>();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(WHITE))
                            .title("Branches"),
                    )
                    .highlight_style(SELECTED_STYLE)
                    .highlight_symbol(BRANCH_EMOJI_WITH_SPACE)
                    .highlight_spacing(HighlightSpacing::Always);

                StatefulWidget::render(list, area, buf, &mut self.state);
            }
            Err(err) => {
                eprintln!("Failed to fetch branch names: {}", err);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some(control) = self.controls.handle_key(key.code) {
            match control {
                Control::Quit => self.exit = true,
                Control::Down => self.select_next(),
                Control::Up => self.select_previous(),
                Control::Switch => self.switch_branch(),
                Control::Fetch => self.fetch_branch(),
                Control::Refresh => self.refresh_branches(),
            }
        }
    }

    fn select_next(&mut self) {
        self.state.select_next();
    }
    fn select_previous(&mut self) {
        self.state.select_previous();
    }

    fn switch_branch(&mut self) {
        let branch_name = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .and_then(|branch| branch.name().ok().flatten())
            .unwrap_or("unknown branch")
            .to_string();

        self.operation_status.set(
            format!("Switching to {}...", branch_name),
            OperationStatusType::Info,
        );

        let result = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .map(|branch| self.branch_manager.switch_to_branch(branch));

        match result {
            Some(Ok(_)) => {
                self.operation_status.set(
                    format!("Successfully switched to branch {}", branch_name),
                    OperationStatusType::Success,
                );
            }
            Some(Err(e)) => {
                self.operation_status.set(
                    format!("Error switching to {}: {}", branch_name, e),
                    OperationStatusType::Error,
                );
            }
            None => {
                self.operation_status
                    .set("No branch selected".to_string(), OperationStatusType::Info);
            }
        }
    }

    fn fetch_branch(&mut self) {
        // Get the branch name first, before any status updates
        let branch_name = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .and_then(|branch| branch.name().ok().flatten())
            .unwrap_or("unknown branch")
            .to_string(); // Clone the string so we own it

        // Now we can update status and use the branch
        self.operation_status.set(
            format!("Fetching {}...", branch_name),
            OperationStatusType::Info,
        );

        // Perform the fetch operation
        let result = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .map(|branch| self.branch_manager.fetch_on_branch(branch));

        // Update status based on result
        match result {
            Some(Ok(_)) => {
                self.operation_status.set(
                    format!("Successfully fetched {}", branch_name),
                    OperationStatusType::Success,
                );
            }
            Some(Err(e)) => {
                self.operation_status.set(
                    format!("Error fetching {}: {}", branch_name, e),
                    OperationStatusType::Error,
                );
            }
            None => {
                self.operation_status
                    .set("No branch selected".to_string(), OperationStatusType::Info);
            }
        }
    }

    fn refresh_branches(&mut self) {
        self.operation_status
            .set("Refreshing".to_string(), OperationStatusType::Info);
        match self.branch_manager.refresh_branches() {
            Ok(_) => self.operation_status.set(
                "Refreshed Branches".to_string(),
                OperationStatusType::Success,
            ),
            Err(e) => {
                self.operation_status.set(
                    format!("Failed to refresh branches: {e}"),
                    OperationStatusType::Error,
                );
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let repo = Repository::open(".").map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let app = App::new(&repo).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let result = app.run(&mut terminal);

    ratatui::restore();
    result
}
