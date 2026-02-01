use crate::buffer::Buffer;
use crate::mode::Mode;

/// Pending two-key or replace action in normal mode (gg, dd, r)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingNormal {
    None,
    SecondG,
    SecondD,
    ReplaceChar,
}

/// Represents the cursor position in the editor
#[derive(Debug, Clone, Copy, Default)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

/// The main editor state
pub struct Editor {
    /// All open buffers
    pub buffers: Vec<Buffer>,
    /// Index of the current buffer in `buffers`
    pub current_buf: usize,
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
    /// Pending two-key or replace action in normal mode (gg, dd, r)
    pub pending_normal: PendingNormal,
    /// Last search pattern for n/N repeat
    pub last_search_pattern: Option<String>,
}

impl Editor {
    /// Create a new editor with an empty buffer
    pub fn new() -> Self {
        Self::from_buffer(Buffer::new())
    }

    /// Create a new editor with a file loaded
    pub fn with_file(path: &str) -> Result<Self, std::io::Error> {
        Buffer::from_file(path).map(Self::from_buffer)
    }

    /// Build an editor from a buffer with default state (cursor, mode, viewport, etc.)
    fn from_buffer(buffer: Buffer) -> Self {
        Self {
            buffers: vec![buffer],
            current_buf: 0,
            cursor: Cursor::default(),
            mode: Mode::default(),
            viewport_offset: 0,
            command_buffer: String::new(),
            status_message: None,
            pending_normal: PendingNormal::None,
            last_search_pattern: None,
        }
    }

    /// Reference to the current buffer
    pub fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.current_buf]
    }

    /// Mutable reference to the current buffer
    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current_buf]
    }

    /// Open a file into a new buffer and switch to it
    pub fn open_file_into_new_buffer(&mut self, path: &str) -> Result<(), std::io::Error> {
        let buffer = Buffer::from_file(path)?;
        self.buffers.push(buffer);
        self.current_buf = self.buffers.len() - 1;
        self.cursor = Cursor::default();
        self.viewport_offset = 0;
        Ok(())
    }

    /// Switch to next buffer (wrap around)
    pub fn next_buf(&mut self) {
        if self.buffers.len() <= 1 {
            return;
        }
        self.current_buf = (self.current_buf + 1) % self.buffers.len();
        self.clamp_cursor_to_buffer();
        self.viewport_offset = 0;
    }

    /// Switch to previous buffer (wrap around)
    pub fn prev_buf(&mut self) {
        if self.buffers.len() <= 1 {
            return;
        }
        self.current_buf = self.current_buf.checked_sub(1).unwrap_or(self.buffers.len() - 1);
        self.clamp_cursor_to_buffer();
        self.viewport_offset = 0;
    }

    /// Clamp cursor to valid range for current buffer
    fn clamp_cursor_to_buffer(&mut self) {
        let buf = self.current_buffer();
        let line_count = buf.line_count();
        if line_count == 0 {
            self.cursor.line = 0;
            self.cursor.col = 0;
        } else {
            self.cursor.line = self.cursor.line.min(line_count - 1);
            self.clamp_cursor_col();
        }
    }

    /// Clear any pending two-key or replace action (e.g. when entering normal from another mode)
    pub fn clear_pending_normal(&mut self) {
        self.pending_normal = PendingNormal::None;
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        let max_col = self.max_col_for_line(self.cursor.line);
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
        if self.cursor.line < self.current_buffer().line_count().saturating_sub(1) {
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
        self.cursor.col = self.max_col_for_line(self.cursor.line);
    }

    /// Move cursor to next word
    pub fn move_word_forward(&mut self) {
        if let Some(chars) = self.current_line_chars() {
            let mut col = self.cursor.col;

            // Skip current word (non-whitespace)
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }
            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }

            if col >= chars.len() && self.cursor.line < self.current_buffer().line_count() - 1 {
                // Move to next line
                self.cursor.line += 1;
                self.cursor.col = 0;
                self.adjust_viewport();
            } else {
                self.cursor.col = col.min(self.max_col_for_line(self.cursor.line));
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

        if let Some(chars) = self.current_line_chars() {
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

    /// Move cursor to first non-blank character on current line (vim ^)
    pub fn move_to_first_non_blank(&mut self) {
        if let Some(chars) = self.current_line_chars() {
            let mut col = 0;
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }
            self.cursor.col = col.min(self.max_col_for_line(self.cursor.line));
        }
    }

    /// Move cursor to last line of buffer (vim G)
    pub fn move_to_last_line(&mut self) {
        let line_count = self.current_buffer().line_count();
        if line_count > 0 {
            self.cursor.line = line_count.saturating_sub(1);
            self.clamp_cursor_col();
            self.adjust_viewport();
        }
    }

    /// Move cursor to first line of buffer (vim gg)
    pub fn move_to_first_line(&mut self) {
        self.cursor.line = 0;
        self.clamp_cursor_col();
        self.adjust_viewport();
    }

    /// Move cursor to end of current word or next word (vim e)
    pub fn move_to_end_of_word(&mut self) {
        if let Some(chars) = self.current_line_chars() {
            let mut col = self.cursor.col;

            // Skip whitespace to start of next word
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }
            // Skip to one past end of word
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }
            let end_col = col.saturating_sub(1);

            if col >= chars.len() && self.cursor.line < self.current_buffer().line_count().saturating_sub(1) {
                // Past end of line; go to next line and find end of first word
                self.cursor.line += 1;
                self.adjust_viewport();
                if let Some(next_line) = self.current_buffer().line(self.cursor.line) {
                    let next_chars: Vec<char> = next_line.chars().collect();
                    let mut c = 0;
                    while c < next_chars.len() && next_chars[c].is_whitespace() {
                        c += 1;
                    }
                    while c < next_chars.len() && !next_chars[c].is_whitespace() {
                        c += 1;
                    }
                    self.cursor.col = c.saturating_sub(1).min(self.max_col_for_line(self.cursor.line));
                } else {
                    self.cursor.col = 0;
                }
            } else {
                self.cursor.col = end_col.min(self.max_col_for_line(self.cursor.line));
            }
        }
    }

    /// True if line is empty or only whitespace (vim "blank" for paragraph motion)
    fn is_line_blank(&self, line_idx: usize) -> bool {
        let len = self.current_buffer().line_len(line_idx);
        if len == 0 {
            return true;
        }
        if let Some(line) = self.current_buffer().line(line_idx) {
            line.chars().all(|c| c.is_whitespace())
        } else {
            true
        }
    }

    /// Move cursor to previous blank line / start of paragraph (vim {)
    pub fn move_paragraph_prev(&mut self) {
        let mut line = self.cursor.line;
        while line > 0 && !self.is_line_blank(line) {
            line -= 1;
        }
        self.cursor.line = line;
        self.cursor.col = 0;
        self.clamp_cursor_col();
        self.adjust_viewport();
    }

    /// Move cursor to next blank line / start of next paragraph (vim })
    pub fn move_paragraph_next(&mut self) {
        let line_count = self.current_buffer().line_count();
        let mut line = self.cursor.line + 1;
        while line < line_count && !self.is_line_blank(line) {
            line += 1;
        }
        self.cursor.line = line.min(line_count.saturating_sub(1));
        self.cursor.col = 0;
        self.clamp_cursor_col();
        self.adjust_viewport();
    }

    /// Maximum valid column for a line in the current mode (Insert: end of line; Normal: last char)
    fn max_col_for_line(&self, line: usize) -> usize {
        let line_len = self.current_buffer().line_len(line);
        if self.mode == Mode::Insert {
            line_len
        } else {
            line_len.saturating_sub(1).max(0)
        }
    }

    /// Current line as Vec<char> for motion logic (may include newline)
    fn current_line_chars(&self) -> Option<Vec<char>> {
        self.current_buffer()
            .line(self.cursor.line)
            .map(|line| line.chars().collect())
    }

    /// Clamp cursor column to valid range for current line
    fn clamp_cursor_col(&mut self) {
        let max_col = self.max_col_for_line(self.cursor.line);
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
        self.cursor.col = self.current_buffer().line_len(self.cursor.line);
    }

    /// Enter insert mode at start of line
    pub fn enter_insert_mode_start(&mut self) {
        self.mode = Mode::Insert;
        self.cursor.col = 0;
    }

    /// Open a new line below current line and enter insert mode (vim o)
    pub fn open_line_below(&mut self) {
        let line = self.cursor.line;
        let line_len = self.current_buffer().line_len(line);
        self.current_buffer_mut().insert_newline(line, line_len);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.adjust_viewport();
        self.mode = Mode::Insert;
    }

    /// Open a new line above current line and enter insert mode (vim O)
    pub fn open_line_above(&mut self) {
        let line = self.cursor.line;
        self.current_buffer_mut().insert_newline(line, 0);
        self.cursor.col = 0;
        self.adjust_viewport();
        self.mode = Mode::Insert;
    }

    /// Enter normal mode
    pub fn enter_normal_mode(&mut self) {
        self.clear_pending_normal();
        self.mode = Mode::Normal;
        // Move cursor back one if we're past the end
        self.clamp_cursor_col();
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.mode = Mode::Command;
        self.command_buffer.clear();
    }

    /// Enter search mode (vim /)
    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.command_buffer.clear();
    }

    /// Run forward search from current cursor; move to match and save pattern. Returns true if found.
    pub fn search_forward(&mut self) -> bool {
        if self.command_buffer.is_empty() {
            self.set_status("No pattern");
            return false;
        }
        if let Some((line, col)) = self.current_buffer().find_forward(
            self.cursor.line,
            self.cursor.col,
            self.command_buffer.as_str(),
            true,
        ) {
            self.cursor.line = line;
            self.cursor.col = col;
            self.clamp_cursor_col();
            self.adjust_viewport();
            self.last_search_pattern = Some(self.command_buffer.clone());
            true
        } else {
            self.set_status("Pattern not found");
            false
        }
    }

    /// Run backward search from current cursor; move to match and save pattern. Returns true if found.
    #[allow(dead_code)]
    pub fn search_backward(&mut self) -> bool {
        if self.command_buffer.is_empty() {
            self.set_status("No pattern");
            return false;
        }
        if let Some((line, col)) = self.current_buffer().find_backward(
            self.cursor.line,
            self.cursor.col,
            self.command_buffer.as_str(),
            true,
        ) {
            self.cursor.line = line;
            self.cursor.col = col;
            self.clamp_cursor_col();
            self.adjust_viewport();
            self.last_search_pattern = Some(self.command_buffer.clone());
            true
        } else {
            self.set_status("Pattern not found");
            false
        }
    }

    /// Repeat last search forward (vim n)
    pub fn repeat_search_forward(&mut self) -> bool {
        let pattern = match self.last_search_pattern.as_deref() {
            Some(p) if !p.is_empty() => p,
            _ => {
                self.set_status("No previous search");
                return false;
            }
        };
        if let Some((line, col)) =
            self.current_buffer().find_forward(self.cursor.line, self.cursor.col, pattern, true)
        {
            self.cursor.line = line;
            self.cursor.col = col;
            self.clamp_cursor_col();
            self.adjust_viewport();
            true
        } else {
            self.set_status("Pattern not found");
            false
        }
    }

    /// Repeat last search backward (vim N)
    pub fn repeat_search_backward(&mut self) -> bool {
        let pattern = match self.last_search_pattern.as_deref() {
            Some(p) if !p.is_empty() => p,
            _ => {
                self.set_status("No previous search");
                return false;
            }
        };
        if let Some((line, col)) =
            self.current_buffer().find_backward(self.cursor.line, self.cursor.col, pattern, true)
        {
            self.cursor.line = line;
            self.cursor.col = col;
            self.clamp_cursor_col();
            self.adjust_viewport();
            true
        } else {
            self.set_status("Pattern not found");
            false
        }
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, ch: char) {
        let (line, col) = (self.cursor.line, self.cursor.col);
        self.current_buffer_mut().insert_char(line, col, ch);
        self.cursor.col += 1;
    }

    /// Insert a newline at cursor position
    pub fn insert_newline(&mut self) {
        let (line, col) = (self.cursor.line, self.cursor.col);
        self.current_buffer_mut().insert_newline(line, col);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.adjust_viewport();
    }

    /// Delete character at cursor (like 'x' in vim)
    pub fn delete_char_at_cursor(&mut self) {
        let (line, col) = (self.cursor.line, self.cursor.col);
        self.current_buffer_mut().delete_char(line, col);
        self.clamp_cursor_col();
    }

    /// Replace character at cursor with ch; stay in normal mode (vim r)
    pub fn replace_char_at_cursor(&mut self, ch: char) {
        let (line, col) = (self.cursor.line, self.cursor.col);
        if col < self.current_buffer().line_len(line) {
            self.current_buffer_mut().delete_char(line, col);
            self.current_buffer_mut().insert_char(line, col, ch);
        }
        self.clamp_cursor_col();
    }

    /// Delete from cursor to end of line (vim D)
    pub fn delete_to_end_of_line(&mut self) {
        while self.cursor.col < self.current_buffer().line_len(self.cursor.line) {
            let (line, col) = (self.cursor.line, self.cursor.col);
            self.current_buffer_mut().delete_char(line, col);
        }
        self.clamp_cursor_col();
    }

    /// Join current line with next (vim J); cursor on the space between
    pub fn join_lines(&mut self) {
        let line_count = self.current_buffer().line_count();
        if self.cursor.line + 1 >= line_count {
            return;
        }
        let line = self.cursor.line;
        let line_len = self.current_buffer().line_len(line);
        self.current_buffer_mut().insert_char(line, line_len, ' ');
        self.current_buffer_mut().delete_char(line, line_len + 1);
        self.cursor.col = line_len;
        self.clamp_cursor_col();
    }

    /// Delete current line (vim dd); cursor to start of next line or previous if last
    pub fn delete_current_line(&mut self) {
        let line_count = self.current_buffer().line_count();
        if line_count == 0 {
            return;
        }
        let line = self.cursor.line;
        let was_last_line = line == line_count - 1;
        while self.current_buffer().line_len(line) > 0 {
            self.current_buffer_mut().delete_char(line, 0);
        }
        if line < line_count - 1 {
            self.current_buffer_mut().delete_char(line, 0);
        }
        if was_last_line && line > 0 {
            self.cursor.line = line - 1;
        }
        self.cursor.col = 0;
        self.clamp_cursor_col();
        self.adjust_viewport();
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        let (line, col) = (self.cursor.line, self.cursor.col);
        if let Some((new_line, new_col)) =
            self.current_buffer_mut().delete_char_before(line, col)
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
        self.current_buffer_mut().save()?;
        if let Some(name) = self.current_buffer().filename() {
            self.set_status(&format!("\"{}\" written", name));
        } else {
            self.set_status("File saved");
        }
        Ok(())
    }

    /// Execute a command from the command buffer
    pub fn execute_command(&mut self) -> Option<EditorCommand> {
        let cmd = self.command_buffer.trim().to_string();
        let result = match cmd.as_str() {
            "q" | "quit" => Some(EditorCommand::Quit),
            "q!" | "quit!" => Some(EditorCommand::ForceQuit),
            "bn" | "bnext" => {
                self.next_buf();
                None
            }
            "bp" | "bprev" | "bprevious" => {
                self.prev_buf();
                None
            }
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
                    let path = filename.trim().to_string();
                    match self.current_buffer_mut().save_as(&path) {
                        Ok(_) => self.set_status(&format!("\"{}\" written", path)),
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
