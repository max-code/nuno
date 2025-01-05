use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{
        palette::tailwind::GREEN, palette::tailwind::SLATE, palette::tailwind::WHITE, Modifier,
        Style, Stylize,
    },
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
    DefaultTerminal,
};

use git2::{Branch, Repository};

pub struct App<'repo> {
    branch_manager: BranchManager<'repo>,
    state: ListState,
    exit: bool,
}

impl<'a> Widget for &mut App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Percentage(5),
            Constraint::Fill(1),
            Constraint::Percentage(5),
        ])
        .areas(area);

        self.render_header(header_area, buf);
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

        Paragraph::new(format!(
            "Git Branch Explorer ({} {})",
            BRANCH_EMOJI, current_branch
        ))
        .bold()
        .centered()
        .render(area, buf);
    }

    fn render_footer(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Switch <s> | Refresh <r> | Quit <q>")
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
                    .block(Block::new())
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
        if let Some(i) = self.state.selected() {
            if let Some(branch) = self.branch_manager.local_branches.get(i) {
                if let Err(e) = self.branch_manager.switch_to_branch(branch) {
                    eprintln!("Failed to switch branch: {}", e);
                }
            }
        }
    }

    fn fetch_branch(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(branch) = self.branch_manager.local_branches.get(i) {
                if let Err(e) = self.branch_manager.fetch_on_branch(branch) {
                    eprintln!("Failed to switch branch: {}", e);
                }
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
