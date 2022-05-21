use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::{cursor, event, execute, terminal};
use crossterm::terminal::ClearType; 
use anyhow::Result;
use std::time::Duration;
use std::io::stdout;

struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn clear_screen() -> crossterm::Result<()> {
    execute!(stdout(), terminal::Clear(ClearType::All));
    execute!(stdout(), cursor::MoveTo(0, 0))
}


struct Editor {
    exit : bool
}

impl Editor {
    pub fn new() ->Result<Editor> {
        Ok(Editor {
            exit: false
        })
    }

    pub fn on_key_event(&mut self, kevent : KeyEvent) -> Result<()> {
        match kevent {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => self.exit = true,
            _ => { }
        }
        Ok(())
    }
    pub fn on_idle(&mut self) {
        println!("No input yet\r");
    }

    pub fn run_loop(&mut self) ->  Result<()> {
        loop {
            if event::poll(Duration::from_millis(500))? /* modify */ {
                let event = event::read()?;
                
                match event {
                    Event::Key(kevent) => {
                        self.on_key_event(kevent)?;
                    },
                    _ => {
                    }
                };
    
                if self.exit {
                    break;
                } else {
                    println!("{:?}\r", event);
                }
            } else {
                self.on_idle()
            }
        }
        Ok(())
    }
}




fn main() -> anyhow::Result<()> {
    let _clean_up = CleanUp;
    terminal::enable_raw_mode()?; /* modify */
    let mut editor = Editor::new()?;
    editor.run_loop()?;
    Ok(())
}
