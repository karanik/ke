use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, terminal};
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

#[derive(Copy, Clone)]
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
    pos : Point,
    size : Point
}

impl Rect {
    pub fn new () -> Rect {
        Rect { pos: Point { x:0, y:0 } , size: Point { x: 0, y: 0 } }
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
            view : Rect::new(),
            buffer: EditBuffer::new(),
            cursor: Point { x: 0, y: 0 },
        })
    }

    pub fn on_key_event(&mut self, kevent: KeyEvent) -> Result<()> {
        let mut redraw = false;
        let mut new_cursor = self.cursor;
        match kevent {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
            } => self.exit = true,
            KeyEvent {
                code: KeyCode::Down,
                modifiers: event::KeyModifiers::NONE,
            } => {
                if self.cursor.y < self.buffer.num_lines() {
                    self.cursor.y = self.cursor.y + 1;
                    redraw = true;
                }
            }
            KeyEvent {
                code: KeyCode::Up,
                modifiers: event::KeyModifiers::NONE,
            } => {
                if self.cursor.y > 0 {
                    self.cursor.y = self.cursor.y - 1;
                    redraw = true;
                }
            }
            _ => {}
        }

        if self.cursor.y > (self.view.pos.y + self.view.size.y - 1) {
            self.view.pos.y = self.cursor.y - self.view.size.y + 1;
        } else if self.cursor.y < self.view.pos.y  {
            self.view.pos.y = self.cursor.y;
        }

        if redraw {
            self.draw()?
        }

        Ok(())
    }
    pub fn on_idle(&mut self) {
        //  println!("No input yet\r");
    }

    pub fn on_resize(&mut self, new_width: usize, new_height: usize) -> Result<()> {
        self.view.size.x = new_width;
        self.view.size.y = new_height;
        self.draw()?;
        Ok(())
    }

//    fn buffer_to_view(&self, bufferPoint : Point) -> (i32, i32) {
 //       i32 x = self
  //  }

//    fn 

    fn draw(&self) -> Result<()> {
        let mut out = String::new();

        // Clear screen
        queue!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;        

        for y in 0..self.view.size.y {
            let buffer_y = self.view.pos.y + y;
            let bline = if buffer_y < self.buffer.num_lines() {
                self.buffer.lines[buffer_y].clone()
            } else {
                String::new()
            };

            for x in 0..self.view.size.x{
                let buffer_x = self.view.pos.x + x;
                let outc = if buffer_x < bline.len() {
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
        write!(stdout(), "Pos({},{}) View({},{}) Size({},{})", 
            self.cursor.x, self.cursor.y,
            self.view.pos.x, self.view.pos.y,
            self.view.size.x, self.view.size.y
        )?;
       
        
        queue!(stdout(), cursor::MoveTo((self.cursor.x - self.view.pos.x) as u16, (self.cursor.y - self.view.pos.y) as u16))?;
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
