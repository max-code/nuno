use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{palette::tailwind::SLATE, Modifier, Style, Stylize},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
    DefaultTerminal,
};

use git2::{Branch, Repository};

pub struct App<'a> {
    exit: bool,
    state: ListState,
    explorer: BranchExplorer<'a>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        App::new()
    }
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

impl<'a> App<'a> {
    fn new() -> Self {
        let mut explorer = BranchExplorer::new().expect("Failed to initialize git repository");
        explorer.refresh_branches();

        App {
            state: ListState::default().with_selected(Some(0)),
            exit: false,
            explorer,
        }
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
        let current_branch = match self.explorer.get_current_head_branch() {
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
        Paragraph::new("Switch <s> | Quit <q>")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_body(&mut self, area: Rect, buf: &mut Buffer) {
        match self.explorer.get_all_local_branch_names() {
            Ok(branches) => {
                let items = branches
                    .iter()
                    .enumerate()
                    .map(|(i, branch_name)| {
                        let colour = if i % 2 == 0 { SLATE.c950 } else { SLATE.c900 };
                        ListItem::new(branch_name.clone()).bg(colour)
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
            if let Err(err) = self.explorer.switch_branch(&self.rendered_branches[i]) {
                println!("Err {err}");
            }
        }
    }
}

struct BranchExplorer<'repo> {
    repo: Repository,
    local_branches: Vec<Branch<'repo>>,
    all_branches: Vec<Branch<'repo>>,
}

impl<'repo> BranchExplorer<'repo> {
    fn new() -> Result<Self, git2::Error> {
        let repo = Repository::open(".")?;
        Ok(Self {
            repo,
            local_branches: Vec::new(),
            all_branches: Vec::new(),
        })
    }

    fn refresh_branches(&'repo mut self) -> Result<(), git2::Error> {
        let local_branches = self
            .repo
            .branches(Some(git2::BranchType::Local))?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect();

        let all_branches = self
            .repo
            .branches(None)?
            .filter_map(Result::ok)
            .map(|(branch, _)| branch)
            .collect();

        self.local_branches = local_branches;
        self.all_branches = all_branches;
        Ok(())
    }

    fn get_all_local_branch_names(&self) -> Result<Vec<String>, git2::Error> {
        Ok(self
            .local_branches
            .iter()
            .filter_map(|branch| branch.name().ok()?.map(String::from))
            .collect())
    }

    fn get_current_head_branch(&self) -> Result<String, git2::Error> {
        let head = self.repo.head()?;

        if head.is_branch() {
            return Ok(head.shorthand().unwrap_or("HEAD").to_string());
        }

        // Detached head state
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    fn switch_branch(&self, branch_name: &str) -> Result<(), git2::Error> {
        if self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .is_ok()
        {
            let mut opts: git2::build::CheckoutBuilder<'_> = git2::build::CheckoutBuilder::new();
            self.repo.set_head(&format!("refs/heads/{}", branch_name))?;
            self.repo.checkout_head(Some(&mut opts))
        } else {
            Err(git2::Error::from_str(&format!(
                "Branch '{}' not found",
                branch_name
            )))
        }
    }

    // fn fetch_branch(&self, branch_name: &str) -> Result<(), git2::Error> {
    //     self.repo.find_branch(name, branch_type)
    // }
}

fn main() -> io::Result<()> {
    // let explorer = BranchExplorer::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // explorer
    //     .get_all_local_branch_names()
    //     .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    //     .iter()
    //     .for_each(|f| {
    //         println!("branch: {f}");
    //     });

    // if let Err(err) = explorer.switch_branch("branch1") {
    //     eprintln!("Error when switching branch, exiting. error={err}");
    //     std::process::exit(1);
    // }

    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    app_result
}
