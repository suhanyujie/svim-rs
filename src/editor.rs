use std::io::{self, stdout, Error};
use std::time::{Duration, Instant};
use termion::event::Key;
use termion::{color, input::TermRead, raw::IntoRawMode};

use crate::document::{self, Document};
use crate::row::Row;
use crate::terminal::{self, Terminal};

const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);
const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_posi: Position,
    offset: Position,
    document: Document,
    status_msg: StatusMessage,
    quit_times: u8,
}

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(msg: String) -> Self {
        Self {
            text: msg,
            time: Instant::now(),
        }
    }
}

impl Editor {
    pub fn run(&mut self) {
        let _stdout = stdout().into_raw_mode().unwrap();

        loop {
            if let Err(error) = self.refresh_screen() {
                die(&error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::clear_screen();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::clear_screen();
            println!("Exit.")
        } else {
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            Terminal::cursor_position(&Position {
                x: self.cursor_posi.x.saturating_sub(self.offset.x),
                y: self.cursor_posi.y.saturating_sub(self.offset.y),
            })
        }
        Terminal::cursor_show();
        Terminal::flush()
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let press_key = Terminal::read_key()?;
        match press_key {
            Key::Ctrl('q') => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_msg = StatusMessage::from(format!(
                        "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                        self.quit_times
                    ));
                    self.quit_times -= 1;
                    return Ok(());
                }
                self.should_quit = true;
            }
            Key::Ctrl('s') => self.save(),
            Key::Up | Key::Down | Key::Left | Key::Right | Key::PageDown | Key::PageUp => {
                self.move_cursor(press_key)
            }
            Key::Backspace => {
                if self.cursor_posi.x > 0 || self.cursor_posi.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_posi);
                }
            }
            Key::Char(c) => {
                println!("char: {:?}", c);
                self.document.insert(&self.cursor_posi, c);
                self.move_cursor(Key::Right);
            }
            _ => (),
        }
        self.scroll();
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_msg = StatusMessage::from(String::new());
        }

        Ok(())
    }

    fn move_cursor(&mut self, key: Key) {
        let Position { mut y, mut x } = self.cursor_posi;
        let size = self.terminal.size();
        let height = self.document.len();
        let terminal_height = self.terminal.size().height as usize;
        // let width = size.width.saturating_sub(1) as usize;
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };

        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            Key::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            Key::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            }
            Key::PageUp => {
                y = if y > terminal_height {
                    y.saturating_sub(terminal_height)
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y.saturating_add(terminal_height)
                } else {
                    height
                }
            }
            Key::Home => x = 0,
            Key::End => x = width,
            _ => (),
        }

        width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        if x > width {
            x = width;
        }

        self.cursor_posi = Position { x, y }
    }

    fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x.saturating_add(width);
        let row = row.render(start, end);
        println!("{}\r", row);
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.y.saturating_add(terminal_row as usize))
            {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcom_msg();
            } else {
                print!("~\r");
            }
        }
    }

    fn draw_welcom_msg(&self) {
        let mut welcom_msg = format!("svim editor -- version {}", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcom_msg.len();
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));

        welcom_msg = format!("~{}{}", spaces, welcom_msg);
        welcom_msg.truncate(width);
        println!("{}\r", welcom_msg);
    }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_posi;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;

        let mut offset = &mut self.offset;
        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_add(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_add(width).saturating_add(1);
        }
    }

    pub fn default() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut initial_status = String::from("HELP: Ctrl-Q = quit | HELP: Ctrl-S = save");
        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(&file_name);
            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("Err: Cound not open file: {}", file_name);
                Document::default()
            }
        } else {
            Document::default()
        };
        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Failed to init terminal."),
            cursor_posi: Position::default(),
            offset: Position::default(),
            document,
            status_msg: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES,
        }
    }

    fn draw_status_bar(&self) {
        let space = " ".repeat(self.terminal.size().width as usize);
        Terminal::set_bg_color(STATUS_BG_COLOR);

        let mut status;
        let width = self.terminal.size().width as usize;
        let modified_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };
        let mut file_name = "[NoName]".to_string();
        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }
        status = format!(
            "{} - {} lines {}",
            file_name,
            self.document.len(),
            modified_indicator
        );
        if width > status.len() {
            status.push_str(&" ".repeat(width - status.len()));
        }
        let line_indicator = format!(
            "{}/{}",
            self.cursor_posi.y.saturating_add(1),
            self.document.len()
        );
        let len = status.len() + line_indicator.len();
        status.push_str(&" ".repeat(width.saturating_sub(len)));
        status = format!("{}{}", status, line_indicator);
        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{}\r", status);
        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let msg = &self.status_msg;
        if Instant::now() - msg.time < Duration::new(5, 0) {
            let mut text = msg.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text);
        }
    }

    fn prompt(&mut self, tips: &str) -> Result<Option<String>, Error> {
        let mut result = String::new();
        loop {
            self.status_msg = StatusMessage::from(format!("{}{}", tips, result));
            self.refresh_screen()?;
            match crate::editor::read_key()? {
                Key::Backspace => result.truncate(result.len().saturating_sub(1)),
                Key::Char('\n') => {
                    break;
                }
                Key::Char(c) => {
                    if !c.is_control() {
                        result.push(c);
                    }
                }
                Key::Esc => {
                    result.truncate(0);
                    break;
                }
                _ => {}
            }
            self.status_msg = StatusMessage::from(String::new());
            if result.is_empty() {
                return Ok(None);
            }
        }
        Ok(Some(result))
    }

    fn save(&mut self) {
        if self.document.file_name.is_none() {
            let new_name = self.prompt("Save as: ").unwrap_or(None);
            if new_name.is_none() {
                self.status_msg = StatusMessage::from("Save aborted.".to_string());
                return;
            }
            self.document.file_name = new_name;
        }
        if self.document.save().is_ok() {
            self.status_msg = StatusMessage::from("File saved successfully.".to_string());
        } else {
            self.status_msg = StatusMessage::from("File save failed.".to_string());
        }
    }
}

fn read_key() -> Result<Key, std::io::Error> {
    loop {
        if let Some(key) = io::stdin().lock().keys().next() {
            return key;
        }
    }
}

fn die(e: &io::Error) {
    print!("{}", termion::clear::All);
    panic!("{}", e);
}
