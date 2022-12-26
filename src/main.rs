use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, queue, terminal};
use std::io::prelude::*;
use std::io::stdout;
use std::str::FromStr;
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
    file_path: String,
}

impl EditBuffer {
    pub fn new() -> EditBuffer {
        EditBuffer {
            lines: Vec::new(),
            file_path: String::new(),
        }
    }
    pub fn load(&mut self, path: &String) -> Result<()> {
        self.lines.clear();

        //let mut ferr = std::fs::File::open(path);

        match std::fs::File::open(path) {
            Ok(mut f) => {
                let mut buffer = String::new();
                f.read_to_string(&mut buffer)?;
                for line in buffer.split('\n') {
                    let s = String::from_str(line)?;
                    self.lines.push(s);
                }
            }
            Err(err) => eprintln!("error: {}", err),
        }

        self.file_path = path.clone();
        Ok(())
    }

    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 0
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

fn clamp<T: Ord>(v: T, min: T, max: T) -> T {
    std::cmp::max(min, std::cmp::min(max, v))
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
        let max_y = if self.buffer.is_empty() {
            0
        } else {
            self.buffer.num_lines() - 1
        };
        let new_y = clamp(y, 0, max_y);

        let line_max = if self.buffer.is_empty() {
            0
        } else {
            self.buffer.lines[new_y as usize].chars().count()
        };
        let new_x = clamp(x, 0, line_max);

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

    pub fn  offset_cursor(&mut self, x: i32, y: i32) -> Result<bool> {
        let new_y = clamp(
            self.cursor.y as i32 + y,
            0,
            (self.buffer.num_lines() as i32) - 1,
        );
        let line_max = if self.buffer.is_empty() {
            0
        } else {
            self.buffer.lines[new_y as usize].chars().count()
        };
        let new_x = clamp(self.cursor.x as i32 + x, 0, line_max as i32);
        self.set_cursor(new_x as usize, new_y as usize)
    }

    fn insert_char(&mut self, ch: char) {
        if self.buffer.lines.is_empty() {
            self.buffer.lines.push(ch.to_string())
        } else {
            assert!(self.cursor.y < self.buffer.lines.len());

            let line = &mut self.buffer.lines[self.cursor.y];
            let idx = line.char_indices().nth(self.cursor.x);
            if idx.is_none() {
                line.push(ch);
            } else {
                let idx = idx.unwrap();
                line.insert(idx.0, ch);
            }
        }
        self.cursor.x += 1;
        self.redraw().unwrap();
    }

    fn key_enter(&mut self) {
        let mut line = String::new();
        if !self.buffer.lines.is_empty() {
            line = self.buffer.lines[self.cursor.y].clone();
        }
        let (left, right) = line.split_at(self.cursor.x);

        if self.buffer.lines.is_empty() {
            self.buffer.lines.push(left.to_string());
            self.buffer.lines.push(right.to_string());
        } else {
            self.buffer.lines[self.cursor.y] = left.to_string();
            self.buffer
                .lines
                .insert(self.cursor.y + 1, right.to_string());
        }
        self.cursor.y += 1;
        self.cursor.x = 0;
        self.redraw().unwrap();
    }

    fn key_backspace(&mut self) {
        if self.cursor.x == 0 && self.cursor.y == 0 {
            return;
        }

        // First char of line? Merge with previous
        if self.cursor.x == 0 {
            let curr_line = self.buffer.lines[self.cursor.y].clone();
            let prev_line = &mut self.buffer.lines[self.cursor.y-1];
            let old_len = prev_line.len();
            prev_line.push_str(curr_line.as_str());
            self.buffer.lines.remove(self.cursor.y);
            self.cursor.y -= 1;
            self.cursor.x = old_len;
        } else {
            let curr_line = &mut self.buffer.lines[self.cursor.y];
            curr_line.remove(self.cursor.x-1);
            self.cursor.x -= 1;
        } 
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
            } => redraw = self.offset_cursor(0, (self.view.size.y as i32) - 1)?,
            // PgUp
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: event::KeyModifiers::NONE,
            } => redraw = self.offset_cursor(0, -(self.view.size.y as i32) - 2)?,
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
            }
            // Enter
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: event::KeyModifiers::NONE,
            } => self.key_enter(),
            // Backspace
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: event::KeyModifiers::NONE,
            } => self.key_backspace(),
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
            } => self.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Enter => '\n',
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
                let outc = if buffer_x < line_char_count {
                    bline.chars().nth(buffer_x).unwrap()
                } else {
                    ' '
                };
                //execute!(stdout(), cursor::MoveTo(x as u16, y as u16))?;
                //print!("{}", outc);
                out.push(outc);
            }
        }

        queue!(stdout(), crossterm::style::Print(out))?;
        // Draw status
        //queue!(stdout(), cursor::MoveTo(0, (self.view.size.y - 1) as u16))?;
        //write!(stdout(), "Status bar")?;

        queue!(stdout(), cursor::MoveTo(0, 0))?;
        queue!(
            stdout(),
            crossterm::style::Print(format!(
                "Pos({},{}) View({},{}) Size({},{}) | {}",
                self.cursor.x,
                self.cursor.y,
                self.view.pos.x,
                self.view.pos.y,
                self.view.size.x,
                self.view.size.y,
                if self.buffer.file_path.is_empty() {
                    "Untitled"
                } else {
                    &self.buffer.file_path.as_str()
                }
            ))
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
    terminal::enable_raw_mode()?;
    let mut editor = Editor::new()?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        editor.buffer.load(&args[1])?;
    }
    editor.run_loop()?;
    Ok(())
}
