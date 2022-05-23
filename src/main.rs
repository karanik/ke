use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, queue, terminal};
use std::io::prelude::*;
use std::io::stdout;
use std::str::{FromStr};
use std::time::Duration;

struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

#[derive(Copy, Clone, PartialEq)]
struct Point {
    x: usize,
    y: usize,
}

struct EditBuffer {
    lines: Vec<String>,
}

impl EditBuffer {
    pub fn new() -> EditBuffer {
        EditBuffer { lines: Vec::new() }
    }
    pub fn load(&mut self, path: &str) -> Result<()> {
        self.lines.clear();
        let mut f = std::fs::File::open(path)?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;
        for line in buffer.split('\n') {
            let s = String::from_str(line)?;
            self.lines.push(s);
        }
        Ok(())
    }

    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }
}

struct Rect {
    pos: Point,
    size: Point,
}

impl Rect {
    pub fn new() -> Rect {
        Rect {
            pos: Point { x: 0, y: 0 },
            size: Point { x: 0, y: 0 },
        }
    }
}

struct Editor {
    exit: bool,
    view: Rect,
    buffer: EditBuffer,
    cursor: Point,
}

impl Editor {
    pub fn new() -> Result<Editor> {
        Ok(Editor {
            exit: false,
            view: Rect::new(),
            buffer: EditBuffer::new(),
            cursor: Point { x: 0, y: 0 },
        })
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) -> Result<bool> {
        let new_y = std::cmp::max(
            0,
            std::cmp::min(y, self.buffer.num_lines() -1),
        );
        let line_max = self.buffer.lines[new_y as usize].chars().count();
//        let line_max = line.chars().count();
        let new_x = std::cmp::max(0, std::cmp::min(x, line_max));

        let new_curs = Point {
            x: new_x as usize,
            y: new_y as usize,
        };

        if self.cursor == new_curs {
            return Ok(false);
        }

        self.cursor = new_curs;

        // Scroll view on Y
        if self.cursor.y > (self.view.pos.y + self.view.size.y - 1) {
            self.view.pos.y = self.cursor.y - self.view.size.y + 1;
        } else if self.cursor.y < self.view.pos.y {
            self.view.pos.y = self.cursor.y;
        }

        // Scroll view on X
        if self.cursor.x > (self.view.pos.x + self.view.size.x - 1) {
            self.view.pos.x = self.cursor.x - self.view.size.x + 1;
        } else if self.cursor.x < self.view.pos.x {
            self.view.pos.x = self.cursor.x;
        }
        Ok(true)
    }

    pub fn offset_cursor(&mut self, x: i32, y: i32) -> Result<bool> {
        self.set_cursor((self.cursor.x as i32 + x) as usize, (self.cursor.y as i32 + y) as usize)
    }

    fn insert_char(&mut self, ch: char) {
        assert!(self.cursor.y < self.buffer.lines.len());
        let line = &mut self.buffer.lines[self.cursor.y];

        let idx =  line.char_indices().nth(self.cursor.x);
        if idx.is_none() {
            return;
        }
        let idx = idx.unwrap();
        line.insert(idx.0, ch);        
        self.cursor.x += 1;
        self.redraw().unwrap();
    }

    pub fn on_key_event(&mut self, kevent: KeyEvent) -> Result<()> {
        let mut redraw = false;
        match kevent {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
            } => self.exit = true,
            // Down
            KeyEvent {
                code: KeyCode::Down,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(0, 1)?,
            // Up
            KeyEvent {
                code: KeyCode::Up,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(0, -1)?,
            // PgDown
            KeyEvent {
                code: KeyCode::PageDown,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(0, (self.view.size.y as i32)-1)?,
            // PgUp
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(0, -(self.view.size.y as i32)-2)?,
            // Left
            KeyEvent {
                code: KeyCode::Left,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(-1, 0)?,
            // Right
            KeyEvent {
                code: KeyCode::Right,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(1, 0)?,
            // Home
            KeyEvent {
                code: KeyCode::Home,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.set_cursor(0, self.cursor.y)?,
            // Right
            KeyEvent {
                code: KeyCode::End,
                modifiers: event::KeyModifiers::NONE,
            } => {
                let line_max = self.buffer.lines[self.cursor.y].len();
                redraw = self.set_cursor(line_max, self.cursor.y)?
            },
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
            } => self.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Char(ch) => ch,
                _ => unreachable!(),
            }),
            _ => {}
        }

        if redraw {
            self.redraw()?
        }

        Ok(())
    }

    pub fn on_idle(&mut self) {
        //  println!("No input yet\r");
    }

    pub fn on_resize(&mut self, new_width: usize, new_height: usize) -> Result<()> {
        self.view.size.x = new_width;
        self.view.size.y = new_height;
        self.redraw()?;
        Ok(())
    }

    fn redraw(&self) -> Result<()> {
        let mut out = String::new();

        // Clear screen
        queue!(
            stdout(),
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        for y in 0..self.view.size.y {
            let buffer_y = self.view.pos.y + y;
            let bline = if buffer_y < self.buffer.num_lines() {
                self.buffer.lines[buffer_y].clone()
            } else {
                String::new()
            };

            let line_char_count = bline.chars().count();

            for x in 0..self.view.size.x {
                let buffer_x = self.view.pos.x + x;
                let outc = if buffer_x <line_char_count {
                    bline.chars().nth(buffer_x).unwrap()
                } else {
                    ' '
                };
                //execute!(stdout(), cursor::MoveTo(x as u16, y as u16))?;
                //print!("{}", outc);
                out.push(outc);
            }
        }

        write!(stdout(), "{}", out)?;
        // Draw status
        //queue!(stdout(), cursor::MoveTo(0, (self.view.size.y - 1) as u16))?;
        //write!(stdout(), "Status bar")?;

        queue!(stdout(), cursor::MoveTo(0, 0))?;
        write!(
            stdout(),
            "Pos({},{}) View({},{}) Size({},{})",
            self.cursor.x,
            self.cursor.y,
            self.view.pos.x,
            self.view.pos.y,
            self.view.size.x,
            self.view.size.y
        )?;

        queue!(
            stdout(),
            cursor::MoveTo(
                (self.cursor.x - self.view.pos.x) as u16,
                (self.cursor.y - self.view.pos.y) as u16
            ),
            cursor::Show
        )?;
        stdout().flush()?;
        Ok(())
    }

    pub fn run_loop(&mut self) -> Result<()> {
        let new_size = crossterm::terminal::size()?;
        //println!("{:?}\r", new_size);
        //return Ok(());

        self.on_resize(new_size.0 as usize, new_size.1 as usize)?;
        loop {
            if !event::poll(Duration::from_millis(500))? {
                self.on_idle()
            } else {
                let event = event::read()?;
                match event {
                    Event::Key(kevent) => {
                        self.on_key_event(kevent)?;
                    }
                    Event::Resize(x, y) => {
                        self.on_resize(x as usize, y as usize)?;
                    }
                    _ => {}
                };

                if self.exit {
                    break;
                } else {
                    //println!("{:?}\r", event);
                }
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let _clean_up = CleanUp;
    terminal::enable_raw_mode()?; /* modify */
    let mut editor = Editor::new()?;
    editor
        .buffer
        .load("/Users/kostas/rs/github/rstenduke/src/tenduke.rs")?;
    editor.run_loop()?;
    Ok(())
}
