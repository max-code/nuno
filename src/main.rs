use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
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

use git2::{Branch, Repository};

#[derive(Default)]
enum StatusType {
    #[default]
    Info,
    Success,
    Error,
}

impl StatusType {
    fn get_emoji(&self) -> &'static str {
        match self {
            StatusType::Info => "ℹ️ ",
            StatusType::Success => "✅",
            StatusType::Error => "❌",
        }
    }
}
struct OperationStatus {
    message: String,
    status_type: StatusType,
    timestamp: std::time::Instant,
}

impl Default for OperationStatus {
    fn default() -> Self {
        Self {
            message: String::new(),
            status_type: StatusType::default(),
            timestamp: std::time::Instant::now(),
        }
    }
}

pub struct App<'repo> {
    branch_manager: BranchManager<'repo>,
    state: ListState,
    exit: bool,
    operation_status: OperationStatus,
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
        .centered()
        .render(area, buf);
    }

    fn set_status(&mut self, message: &str, status_type: StatusType) {
        self.operation_status = OperationStatus {
            message: message.to_string(),
            status_type,
            timestamp: std::time::Instant::now(),
        }
    }

    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        if self.operation_status.timestamp.elapsed().as_secs() > 3
            && !self.operation_status.message.is_empty()
        {
            self.operation_status = OperationStatus::default();
        }

        let style = match self.operation_status.status_type {
            StatusType::Info => Style::default().fg(WHITE),
            StatusType::Success => Style::default().fg(GREEN.c500),
            StatusType::Error => Style::default().fg(RED.c500),
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
        Paragraph::new("Switch <s> | Fetch <f> | Refresh <r> | Quit <q>")
            .bold()
            .centered()
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
                            GREEN.c500
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

        match key.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_previous(),
            KeyCode::Char('s') => {
                self.switch_branch();
            }
            KeyCode::Char('f') => {
                self.fetch_branch();
            }
            KeyCode::Char('r') => {
                self.refresh_branches();
            }
            _ => {}
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

        self.set_status(
            &format!("Switching to {}...", branch_name),
            StatusType::Info,
        );

        let result = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .map(|branch| self.branch_manager.switch_to_branch(branch));

        match result {
            Some(Ok(_)) => {
                self.set_status(
                    &format!("Successfully switched to branch {}", branch_name),
                    StatusType::Success,
                );
            }
            Some(Err(e)) => {
                self.set_status(
                    &format!("Error switching to {}: {}", branch_name, e),
                    StatusType::Error,
                );
            }
            None => {
                self.set_status("No branch selected", StatusType::Info);
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
        self.set_status(&format!("Fetching {}...", branch_name), StatusType::Info);

        // Perform the fetch operation
        let result = self
            .state
            .selected()
            .and_then(|i| self.branch_manager.local_branches.get(i))
            .map(|branch| self.branch_manager.fetch_on_branch(branch));

        // Update status based on result
        match result {
            Some(Ok(_)) => {
                self.set_status(
                    &format!("Successfully fetched {}", branch_name),
                    StatusType::Success,
                );
            }
            Some(Err(e)) => {
                self.set_status(
                    &format!("Error fetching {}: {}", branch_name, e),
                    StatusType::Error,
                );
            }
            None => {
                self.set_status("No branch selected", StatusType::Info);
            }
        }
    }

    fn refresh_branches(&mut self) {
        if let Err(e) = self.branch_manager.refresh_branches() {
            eprintln!("Failed to refresh branches: {}", e);
        }
    }
}

struct BranchManager<'repo> {
    repo: &'repo Repository,
    local_branches: Vec<Branch<'repo>>,
}

impl<'repo> BranchManager<'repo> {
    fn new(repo: &'repo Repository) -> Result<Self, git2::Error> {
        let local_branches = repo
            .branches(Some(git2::BranchType::Local))?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect();

        Ok(Self {
            repo,
            local_branches,
        })
    }

    fn refresh_branches(&mut self) -> Result<(), git2::Error> {
        self.local_branches = self
            .repo
            .branches(Some(git2::BranchType::Local))?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect::<Vec<_>>();
        Ok(())
    }

    fn get_all_local_branch_names(&self) -> Result<Vec<String>, git2::Error> {
        Ok(self
            .local_branches
            .iter()
            .filter_map(|branch| branch.name().ok()?.map(String::from))
            .collect())
    }

    fn get_current_branch(&self) -> Result<String, git2::Error> {
        let head = self.repo.head()?;

        if head.is_branch() {
            Ok(head.shorthand().unwrap_or("HEAD").to_string())
        } else {
            // Detached head state
            let commit = head.peel_to_commit()?;
            Ok(commit.id().to_string())
        }
    }

    fn switch_to_branch(&self, branch: &Branch) -> Result<(), git2::Error> {
        let branch_name = branch
            .name()?
            .ok_or_else(|| git2::Error::from_str("Invalid UTF-8 in branch name."))?;

        let current_head_name = self.get_current_branch()?;

        let statuses = self.repo.statuses(None)?;
        if !statuses.is_empty() {
            return Err(git2::Error::from_str(&format!(
                "Uncommitted local changes on branch {}",
                current_head_name,
            )));
        }

        let mut opts = git2::build::CheckoutBuilder::new();

        self.repo.set_head(&format!("refs/heads/{}", branch_name))?;
        self.repo.checkout_head(Some(&mut opts))
    }

    fn fetch_on_branch(&self, branch: &Branch) -> Result<(), git2::Error> {
        let branch_name = branch
            .name()?
            .ok_or_else(|| git2::Error::from_str("Invalid UTF-8 in branch name"))?;

        let mut remote = self.repo.find_remote("origin")?;
        let refspec = format!(
            "+refs/heads/{}:refs/remotes/origin/{}",
            branch_name, branch_name
        );

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.download_tags(git2::AutotagOption::None);

        remote.fetch(&[&refspec], Some(&mut fetch_options), None)?;

        Ok(())
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
