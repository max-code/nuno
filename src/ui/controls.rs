use crossterm::event::KeyCode;

#[derive(Copy, Clone)]
pub enum Control {
    Switch,
    Fetch,
    Refresh,
    Up,
    Down,
    Quit,
}

impl Control {
    pub fn key(&self) -> KeyCode {
        match self {
            Control::Switch => KeyCode::Char('s'),
            Control::Fetch => KeyCode::Char('f'),
            Control::Refresh => KeyCode::Char('r'),
            Control::Up => KeyCode::Up,
            Control::Down => KeyCode::Down,
            Control::Quit => KeyCode::Char('q'),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Control::Switch => "Switch",
            Control::Fetch => "Fetch",
            Control::Refresh => "Refresh",
            Control::Up => "Up",
            Control::Down => "Down",
            Control::Quit => "Quit",
        }
    }

    pub fn format_key(&self) -> String {
        let key_text = match self.key() {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            _ => String::new(),
        };
        format!("<{}>", key_text)
    }
}

pub struct Controls {
    controls: Vec<Control>,
}

impl Controls {
    pub fn new() -> Self {
        Self {
            controls: vec![
                Control::Switch,
                Control::Fetch,
                Control::Refresh,
                Control::Up,
                Control::Down,
                Control::Quit,
            ],
        }
    }

    pub fn format_help(&self) -> String {
        self.controls
            .iter()
            .map(|control| format!("{} {}", control.display_name(), control.format_key()))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    pub fn handle_key(&self, code: KeyCode) -> Option<Control> {
        self.controls
            .iter()
            .find(|control| control.key() == code)
            .copied()
    }
}
