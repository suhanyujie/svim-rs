use std::fmt::format;
use std::io::{self, stdout, Write};
use termion::event::Key;
use termion::{input::TermRead, raw::IntoRawMode};

use crate::document::Document;
use crate::row::Row;
use crate::terminal::{self, Terminal};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_posi: Position,
    offset: Position,
    document: Document,
}

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
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

        // for key in io::stdin().keys() {
        //     match key {
        //         Ok(key) => match key {
        //             Key::Char(c) => {
        //                 if c.is_control() {
        //                     println!("{}", c as u8);
        //                 } else {
        //                     println!("{:?}, {}", c as u8, c);
        //                 }
        //             }
        //             Key::Ctrl('q') => break,
        //             _ => println!("{:?}", key),
        //         },
        //         Err(err) => die(&err),
        //     }
        // }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::clear_screen();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::clear_screen();
            println!("Exit.")
        } else {
            self.draw_rows();
            Terminal::cursor_position(&Position::default());
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
                self.should_quit = true;
            }
            Key::Up | Key::Down | Key::Left | Key::Right | Key::PageDown | Key::PageUp => {
                self.move_cursor(press_key)
            }
            Key::Char(c) => {
                println!("char: {:?}", c);
            }
            _ => (),
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
                    y - terminal_height
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y + terminal_height as usize
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
        let end = self.offset.x + width;
        let row = row.render(start, end);
        println!("{}\r", row);
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height - 1 {
            Terminal::clear_current_line();
            if let Some(row) = self.document.row(terminal_row as usize + self.offset.y) {
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
        let document = if args.len() > 1 {
            let file_name = &args[1];
            Document::open(&file_name).unwrap_or_default()
        } else {
            Document::default()
        };
        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Failed to init terminal."),
            cursor_posi: Position::default(),
            offset: Position::default(),
            document,
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
