use crate::buffer::Buffer;
use crate::mode::Mode;

/// Represents the cursor position in the editor
#[derive(Debug, Clone, Copy, Default)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

/// The main editor state
pub struct Editor {
    /// The text buffer being edited
    pub buffer: Buffer,
    /// Current cursor position
    pub cursor: Cursor,
    /// Current editing mode
    pub mode: Mode,
    /// Viewport offset (first visible line)
    pub viewport_offset: usize,
    /// Command line input buffer (for : commands)
    pub command_buffer: String,
    /// Status message to display
    pub status_message: Option<String>,
}

impl Editor {
    /// Create a new editor with an empty buffer
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::default(),
            mode: Mode::default(),
            viewport_offset: 0,
            command_buffer: String::new(),
            status_message: None,
        }
    }

    /// Create a new editor with a file loaded
    pub fn with_file(path: &str) -> Result<Self, std::io::Error> {
        let buffer = Buffer::from_file(path)?;
        Ok(Self {
            buffer,
            cursor: Cursor::default(),
            mode: Mode::default(),
            viewport_offset: 0,
            command_buffer: String::new(),
            status_message: None,
        })
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        let max_col = if self.mode == Mode::Insert {
            line_len
        } else {
            line_len.saturating_sub(1)
        };
        if self.cursor.col < max_col {
            self.cursor.col += 1;
        }
    }

    /// Move cursor up
    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.clamp_cursor_col();
            self.adjust_viewport();
        }
    }

    /// Move cursor down
    pub fn move_down(&mut self) {
        if self.cursor.line < self.buffer.line_count().saturating_sub(1) {
            self.cursor.line += 1;
            self.clamp_cursor_col();
            self.adjust_viewport();
        }
    }

    /// Move cursor to start of line
    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
    }

    /// Move cursor to end of line
    pub fn move_to_line_end(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        self.cursor.col = if self.mode == Mode::Insert {
            line_len
        } else {
            line_len.saturating_sub(1).max(0)
        };
    }

    /// Move cursor to next word
    pub fn move_word_forward(&mut self) {
        if let Some(line) = self.buffer.line(self.cursor.line) {
            let line_str: String = line.chars().collect();
            let chars: Vec<char> = line_str.chars().collect();
            let mut col = self.cursor.col;

            // Skip current word (non-whitespace)
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }
            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }

            if col >= chars.len() && self.cursor.line < self.buffer.line_count() - 1 {
                // Move to next line
                self.cursor.line += 1;
                self.cursor.col = 0;
                self.adjust_viewport();
            } else {
                self.cursor.col = col.min(self.buffer.line_len(self.cursor.line).saturating_sub(1));
            }
        }
    }

    /// Move cursor to previous word
    pub fn move_word_backward(&mut self) {
        if self.cursor.col == 0 {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                self.move_to_line_end();
                self.adjust_viewport();
            }
            return;
        }

        if let Some(line) = self.buffer.line(self.cursor.line) {
            let line_str: String = line.chars().collect();
            let chars: Vec<char> = line_str.chars().collect();
            let mut col = self.cursor.col.saturating_sub(1);

            // Skip whitespace backwards
            while col > 0 && chars[col].is_whitespace() {
                col -= 1;
            }
            // Skip word backwards
            while col > 0 && !chars[col - 1].is_whitespace() {
                col -= 1;
            }

            self.cursor.col = col;
        }
    }

    /// Clamp cursor column to valid range for current line
    fn clamp_cursor_col(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        let max_col = if self.mode == Mode::Insert {
            line_len
        } else {
            line_len.saturating_sub(1).max(0)
        };
        self.cursor.col = self.cursor.col.min(max_col);
    }

    /// Adjust viewport to keep cursor visible
    fn adjust_viewport(&mut self) {
        // This will be called with the viewport height from the UI
        // For now, we'll handle basic scrolling
    }

    /// Adjust viewport with a specific height
    pub fn adjust_viewport_with_height(&mut self, height: usize) {
        if self.cursor.line < self.viewport_offset {
            self.viewport_offset = self.cursor.line;
        } else if self.cursor.line >= self.viewport_offset + height {
            self.viewport_offset = self.cursor.line - height + 1;
        }
    }

    /// Enter insert mode
    pub fn enter_insert_mode(&mut self) {
        self.mode = Mode::Insert;
    }

    /// Enter insert mode after current character
    pub fn enter_insert_mode_append(&mut self) {
        self.mode = Mode::Insert;
        self.move_right();
    }

    /// Enter insert mode at end of line
    pub fn enter_insert_mode_end(&mut self) {
        self.mode = Mode::Insert;
        self.cursor.col = self.buffer.line_len(self.cursor.line);
    }

    /// Enter insert mode at start of line
    pub fn enter_insert_mode_start(&mut self) {
        self.mode = Mode::Insert;
        self.cursor.col = 0;
    }

    /// Enter normal mode
    pub fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        // Move cursor back one if we're past the end
        self.clamp_cursor_col();
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.mode = Mode::Command;
        self.command_buffer.clear();
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert_char(self.cursor.line, self.cursor.col, ch);
        self.cursor.col += 1;
    }

    /// Insert a newline at cursor position
    pub fn insert_newline(&mut self) {
        self.buffer.insert_newline(self.cursor.line, self.cursor.col);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.adjust_viewport();
    }

    /// Delete character at cursor (like 'x' in vim)
    pub fn delete_char_at_cursor(&mut self) {
        self.buffer.delete_char(self.cursor.line, self.cursor.col);
        self.clamp_cursor_col();
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if let Some((new_line, new_col)) =
            self.buffer.delete_char_before(self.cursor.line, self.cursor.col)
        {
            self.cursor.line = new_line;
            self.cursor.col = new_col;
        }
    }

    /// Set a status message
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Save the current buffer
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        self.buffer.save()?;
        if let Some(name) = self.buffer.filename() {
            self.set_status(&format!("\"{}\" written", name));
        } else {
            self.set_status("File saved");
        }
        Ok(())
    }

    /// Execute a command from the command buffer
    pub fn execute_command(&mut self) -> Option<EditorCommand> {
        let cmd = self.command_buffer.trim();
        let result = match cmd {
            "q" | "quit" => Some(EditorCommand::Quit),
            "q!" | "quit!" => Some(EditorCommand::ForceQuit),
            "w" | "write" => {
                match self.save() {
                    Ok(_) => {}
                    Err(e) => self.set_status(&format!("Error saving: {}", e)),
                }
                None
            }
            "wq" => {
                match self.save() {
                    Ok(_) => Some(EditorCommand::Quit),
                    Err(e) => {
                        self.set_status(&format!("Error saving: {}", e));
                        None
                    }
                }
            }
            _ => {
                // Check for :w <filename>
                if let Some(filename) = cmd.strip_prefix("w ").or_else(|| cmd.strip_prefix("write "))
                {
                    match self.buffer.save_as(filename.trim()) {
                        Ok(_) => self.set_status(&format!("\"{}\" written", filename.trim())),
                        Err(e) => self.set_status(&format!("Error saving: {}", e)),
                    }
                    None
                } else {
                    self.set_status(&format!("Unknown command: {}", cmd));
                    None
                }
            }
        };
        self.command_buffer.clear();
        self.mode = Mode::Normal;
        result
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

/// Commands that affect the application state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    Quit,
    ForceQuit,
}
