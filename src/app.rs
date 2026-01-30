use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::editor::Editor;
use crate::input::{handle_key_event, InputResult};
use crate::ui;

/// The main application struct
pub struct App {
    /// The editor state
    pub editor: Editor,
    /// Whether the application is still running
    running: bool,
}

impl App {
    /// Create a new application with an empty buffer
    pub fn new() -> Self {
        Self {
            editor: Editor::new(),
            running: true,
        }
    }

    /// Create a new application with a file loaded
    pub fn with_file(path: &str) -> io::Result<Self> {
        let editor = Editor::with_file(path)?;
        Ok(Self {
            editor,
            running: true,
        })
    }

    /// Run the main application loop
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        while self.running {
            // Render the UI
            terminal.draw(|frame| ui::render(frame, &mut self.editor))?;

            // Poll for events with a timeout
            if event::poll(Duration::from_millis(100))? {
                // Handle the event
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events (not release)
                    if key.kind == KeyEventKind::Press {
                        match handle_key_event(&mut self.editor, key) {
                            InputResult::Continue => {}
                            InputResult::Exit => self.running = false,
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
