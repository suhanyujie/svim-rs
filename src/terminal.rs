use std::io::{self, stdout, Write};
use termion::{self, event::Key, input::TermRead};

pub use crate::editor::Position;

pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub struct Terminal {
    size: Size,
}

impl Terminal {
    pub fn default() -> Result<Self, std::io::Error> {
        let size = termion::terminal_size()?;
        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1,
            },
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn clear_screen() {
        print!("{}", termion::clear::All);
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn cursor_position(posi: &Position) {
        let x = posi.x.saturating_add(1);
        let y = posi.y.saturating_add(1);
        print!("{}", termion::cursor::Goto(x as u16, y as u16));
    }

    pub fn flush() -> Result<(), io::Error> {
        io::stdout().flush()
    }

    pub fn read_key() -> Result<Key, std::io::Error> {
        loop {
            if let Some(key) = io::stdin().lock().keys().next() {
                return key;
            }
        }
    }

    pub fn cursor_show() {
        print!("{}", termion::cursor::Show);
    }

    pub fn cursor_hide() {
        print!("{}", termion::cursor::Hide);
    }

    pub fn clear_current_line() {
        print!("{}", termion::clear::CurrentLine);
    }
}
