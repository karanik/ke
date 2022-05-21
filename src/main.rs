use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, terminal};
use std::io::stdout;
use std::str::FromStr;
use std::time::Duration;
//use std::fs::read_to_string;
use std::io::prelude::*;

    struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn clear_screen() -> Result<()> {
    execute!(stdout(), terminal::Clear(ClearType::All))?;
    execute!(stdout(), cursor::MoveTo(0, 0))?;
    Ok(())
}

struct Cursor {
    x: u32,
    y: u32,
}

struct EditBuffer {
    lines : Vec<String> 
}

impl EditBuffer {
    pub fn new() -> EditBuffer {
        EditBuffer { lines: Vec::new() }
    }
    pub fn load(&mut self, path : &str) -> Result<()> {
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
}


struct Editor {
    exit: bool,
    width : u16,
    height : u16,
    viewOffsetX : u32,
    viewOfssetY : u32,
    buffer : EditBuffer,
//    cursor: Cursor,
}

impl Editor {
    pub fn new() -> Result<Editor> {
        Ok(Editor {
            exit: false,
            width : 0,
            height : 0,
            viewOffsetX : 0,
            viewOfssetY : 0,
            buffer : EditBuffer::new(),
  //          cursor: Cursor { x: 0, y: 0 },
        })
    }

    pub fn on_key_event(&mut self, kevent: KeyEvent) -> Result<()> {
        match kevent {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
            } => self.exit = true,
            _ => {}
        }
        Ok(())
    }
    pub fn on_idle(&mut self) {
      //  println!("No input yet\r");
    }

    pub fn on_resize(&mut self, new_width : u16, new_height : u16) {
        self.width = new_width;
        self.height = new_height;
    }

    fn draw(&self) -> Result<()> {
        // Clear screen
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))?;

        for y in 0..self.height {
            let buffer_y = (self.viewOfssetY + y as u32) as usize;
            let bline  = if buffer_y < self.buffer.lines.len() {
                self.buffer.lines[buffer_y].clone()
            } else {
                String::new()
            };
            for x in 0..self.width {
                execute!(stdout(), cursor::MoveTo(x, y))?;
                let buffer_x = (self.viewOffsetX + x as u32) as usize;
                let outc = if buffer_x < bline.len() {
                    bline.chars().nth(buffer_x).unwrap()
                } else {
                   ' '
                };
                print!("{}", outc);
            }
        }

        // Draw status
        execute!(stdout(), cursor::MoveTo(0, self.height-1))?;
        print!("Status bar");


        execute!(stdout(), cursor::MoveTo(0, 0))?;
        Ok(())
    }

    pub fn run_loop(&mut self) -> Result<()> {
        let new_size = crossterm::terminal::size()?;
        //println!("{:?}\r", new_size);
        //return Ok(());

        self.on_resize(new_size.0, new_size.1);
        self.draw()?;
        loop {
            if !event::poll(Duration::from_millis(500))? {
                self.on_idle()
            } else {
                let event = event::read()?;
                match event {
                    Event::Key(kevent) => {
                        self.on_key_event(kevent)?;
                    }
                    _ => {}
                };

                if self.exit {
                    break;
                } else {
                    println!("{:?}\r", event);
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
    editor.buffer.load("/Users/kostas/rs/github/rstenduke/src/tenduke.rs")?;
    editor.run_loop()?;
    Ok(())
}
