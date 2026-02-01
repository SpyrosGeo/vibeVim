use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::dir::DirectoryState;
use crate::editor::Editor;
use crate::input::{handle_key_event, InputResult};
use crate::ui;

/// The main application struct
pub struct App {
    /// The editor state
    pub editor: Editor,
    /// Directory state when opened with a directory (e.g. `vibeVim .`)
    pub directory_state: Option<DirectoryState>,
    /// Whether the file explorer sidebar is visible (when directory_state is Some)
    pub sidebar_visible: bool,
    /// When true and directory_state is Some, keys go to the file explorer; else to the editor
    pub focus_on_explorer: bool,
    /// Ctrl+w pressed, waiting for second key (w) to toggle focus
    pub pending_ctrl_w: bool,
    /// Space pressed in normal mode, waiting for 'e' to toggle sidebar / open dir
    pub pending_space_e: bool,
    /// Whether the application is still running
    running: bool,
}

impl App {
    /// Create a new application with an empty buffer
    pub fn new() -> Self {
        Self {
            editor: Editor::new(),
            directory_state: None,
            sidebar_visible: true,
            focus_on_explorer: false,
            pending_ctrl_w: false,
            pending_space_e: false,
            running: true,
        }
    }

    /// Create a new application with a file loaded
    pub fn with_file(path: &str) -> io::Result<Self> {
        let editor = Editor::with_file(path)?;
        Ok(Self {
            editor,
            directory_state: None,
            sidebar_visible: true,
            focus_on_explorer: false,
            pending_ctrl_w: false,
            pending_space_e: false,
            running: true,
        })
    }

    /// Create a new application with a directory (file explorer sidebar).
    pub fn with_directory(path: &Path) -> io::Result<Self> {
        let directory_state = DirectoryState::new(path)?;
        Ok(Self {
            editor: Editor::new(),
            directory_state: Some(directory_state),
            sidebar_visible: true,
            focus_on_explorer: true,
            pending_ctrl_w: false,
            pending_space_e: false,
            running: true,
        })
    }

    /// Toggle the file explorer sidebar visibility (when directory_state is Some).
    pub fn toggle_sidebar(&mut self) {
        if self.directory_state.is_some() {
            self.sidebar_visible = !self.sidebar_visible;
        }
    }

    /// Toggle sidebar visibility if a directory is open; otherwise open current directory in the sidebar.
    pub fn toggle_sidebar_or_open_current_dir(&mut self) {
        if self.directory_state.is_some() {
            self.toggle_sidebar();
        } else {
            match std::env::current_dir() {
                Ok(cwd) => {
                    match DirectoryState::new(&cwd) {
                        Ok(dir) => {
                            self.directory_state = Some(dir);
                            self.sidebar_visible = true;
                            self.focus_on_explorer = true;
                            self.editor.set_status("Opened current directory");
                        }
                        Err(_) => {
                            self.editor.set_status("Failed to open current directory");
                        }
                    }
                }
                Err(_) => {
                    self.editor.set_status("No current directory");
                }
            }
        }
    }

    /// Run the main application loop
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        while self.running {
            // Render the UI
            terminal.draw(|frame| ui::render(frame, self))?;

            // Poll for events with a timeout
            if event::poll(Duration::from_millis(100))? {
                // Handle the event
                if let Event::Key(key) = event::read()? {
                    // Handle key press and repeat (not release)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        match handle_key_event(self, key) {
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
