use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter, Error as IoError};
use std::path::PathBuf;

/// A text buffer backed by a Rope data structure for efficient editing.
pub struct Buffer {
    /// The rope containing the text content
    text: Rope,
    /// The file path associated with this buffer, if any
    pub file_path: Option<PathBuf>,
    /// Whether the buffer has been modified since last save
    pub modified: bool,
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            file_path: None,
            modified: false,
        }
    }

    /// Returns a normalized path (canonical when the path exists, else the path as given).
    pub fn normalize_path(path: &str) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
    }

    /// Load a buffer from a file
    pub fn from_file(path: &str) -> Result<Self, IoError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let text = Rope::from_reader(reader)?;
        let file_path = Some(Self::normalize_path(path));

        Ok(Self {
            text,
            file_path,
            modified: false,
        })
    }

    /// Save the buffer to its associated file
    pub fn save(&mut self) -> Result<(), IoError> {
        if let Some(ref path) = self.file_path {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            self.text.write_to(writer)?;
            self.modified = false;
            Ok(())
        } else {
            Err(IoError::new(
                std::io::ErrorKind::NotFound,
                "No file path associated with buffer",
            ))
        }
    }

    /// Save the buffer to a specific file path
    pub fn save_as(&mut self, path: &str) -> Result<(), IoError> {
        self.file_path = Some(PathBuf::from(path));
        self.save()
    }

    /// Get the total number of lines in the buffer
    pub fn line_count(&self) -> usize {
        self.text.len_lines()
    }

    /// Get the content of a specific line (0-indexed)
    pub fn line(&self, line_idx: usize) -> Option<ropey::RopeSlice<'_>> {
        if line_idx < self.line_count() {
            Some(self.text.line(line_idx))
        } else {
            None
        }
    }

    /// Get the length of a specific line (excluding newline)
    pub fn line_len(&self, line_idx: usize) -> usize {
        if let Some(line) = self.line(line_idx) {
            let len = line.len_chars();
            // Subtract 1 for the newline character if present (not on last line)
            if len > 0 && line_idx < self.line_count() - 1 {
                len.saturating_sub(1)
            } else {
                len
            }
        } else {
            0
        }
    }

    /// Insert a character at the given line and column position
    pub fn insert_char(&mut self, line: usize, col: usize, ch: char) {
        let line_start = self.text.line_to_char(line);
        let char_idx = line_start + col;
        self.text.insert_char(char_idx, ch);
        self.modified = true;
    }

    /// Delete a character at the given line and column position
    pub fn delete_char(&mut self, line: usize, col: usize) {
        if col < self.line_len(line) || (line < self.line_count() - 1 && col == self.line_len(line))
        {
            let line_start = self.text.line_to_char(line);
            let char_idx = line_start + col;
            if char_idx < self.text.len_chars() {
                self.text.remove(char_idx..char_idx + 1);
                self.modified = true;
            }
        }
    }

    /// Delete the character before the given position (backspace)
    pub fn delete_char_before(&mut self, line: usize, col: usize) -> Option<(usize, usize)> {
        if col > 0 {
            // Delete character in the same line
            self.delete_char(line, col - 1);
            Some((line, col - 1))
        } else if line > 0 {
            // Join with previous line
            let prev_line_len = self.line_len(line - 1);
            self.delete_char(line - 1, prev_line_len);
            Some((line - 1, prev_line_len))
        } else {
            None
        }
    }

    /// Insert a newline at the given position
    pub fn insert_newline(&mut self, line: usize, col: usize) {
        self.insert_char(line, col, '\n');
    }

    /// Find pattern in a single line at or after start_col; returns (line_idx, col) if found.
    fn find_in_line(
        &self,
        line_idx: usize,
        start_col: usize,
        pattern_chars: &[char],
    ) -> Option<(usize, usize)> {
        if pattern_chars.is_empty() {
            return None;
        }
        let line_len = self.line_len(line_idx);
        let line_chars: Vec<char> = self
            .line(line_idx)
            .map(|l| l.chars().take(line_len).collect())?;
        let max_start = line_len.saturating_sub(pattern_chars.len());
        for col in start_col..=max_start {
            if col + pattern_chars.len() <= line_chars.len()
                && line_chars[col..].starts_with(pattern_chars)
            {
                return Some((line_idx, col));
            }
        }
        None
    }

    /// Find the next occurrence of pattern forward from (start_line, start_col).
    /// Returns (line, col) of the first character of the match, or None if not found.
    /// Search starts after the cursor on the current line (vim-style /).
    pub fn find_forward(
        &self,
        start_line: usize,
        start_col: usize,
        pattern: &str,
        wrap: bool,
    ) -> Option<(usize, usize)> {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        if pattern_chars.is_empty() {
            return None;
        }
        let line_count = self.line_count();
        if line_count == 0 {
            return None;
        }

        // Pass 1: from (start_line, start_col+1) to end of buffer
        if start_line < line_count {
            if let Some((line, col)) =
                self.find_in_line(start_line, start_col + 1, &pattern_chars)
            {
                return Some((line, col));
            }
        }
        for line_idx in (start_line + 1)..line_count {
            if let Some((line, col)) = self.find_in_line(line_idx, 0, &pattern_chars) {
                return Some((line, col));
            }
        }

        // Pass 2 (wrap): from (0, 0) to (start_line, start_col)
        if wrap {
            for line_idx in 0..start_line {
                if let Some(m) = self.find_in_line(line_idx, 0, &pattern_chars) {
                    return Some(m);
                }
            }
            if start_line < line_count {
                if let Some((line, col)) =
                    self.find_in_line(start_line, 0, &pattern_chars)
                {
                    if col <= start_col {
                        return Some((line, col));
                    }
                }
            }
        }

        None
    }

    /// Find the last occurrence of pattern before (start_line, start_col) (previous match).
    pub fn find_backward(
        &self,
        start_line: usize,
        start_col: usize,
        pattern: &str,
        wrap: bool,
    ) -> Option<(usize, usize)> {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        if pattern_chars.is_empty() {
            return None;
        }
        let line_count = self.line_count();
        if line_count == 0 {
            return None;
        }

        // Pass 1: current line from start_col-1 down to 0, then lines start_line-1 down to 0
        if start_line < line_count && start_col > 0 {
            let line_len = self.line_len(start_line);
            let line_chars: Vec<char> = self
                .line(start_line)
                .map(|l| l.chars().take(line_len).collect())?;
            let max_start = line_len.saturating_sub(pattern_chars.len());
            for col in (0..start_col.min(max_start + 1)).rev() {
                if col + pattern_chars.len() <= line_chars.len()
                    && line_chars[col..].starts_with(&pattern_chars[..])
                {
                    return Some((start_line, col));
                }
            }
        }
        for line_idx in (0..start_line).rev() {
            if let Some((line, col)) =
                self.find_in_line_backward(line_idx, self.line_len(line_idx), &pattern_chars)
            {
                return Some((line, col));
            }
        }

        // Pass 2 (wrap): from end of buffer down to (start_line, start_col)
        if wrap {
            for line_idx in (start_line + 1)..line_count {
                if let Some((line, col)) =
                    self.find_in_line_backward(line_idx, self.line_len(line_idx), &pattern_chars)
                {
                    return Some((line, col));
                }
            }
            if start_line < line_count {
                let line_len = self.line_len(start_line);
                let line_chars: Vec<char> = self
                    .line(start_line)
                    .map(|l| l.chars().take(line_len).collect())?;
                let max_start = line_len.saturating_sub(pattern_chars.len());
                for col in (start_col..=max_start).rev() {
                    if col + pattern_chars.len() <= line_chars.len()
                        && line_chars[col..].starts_with(&pattern_chars[..])
                    {
                        return Some((start_line, col));
                    }
                }
            }
        }

        None
    }

    /// Find last occurrence of pattern in line up to end_col; returns (line_idx, col).
    fn find_in_line_backward(
        &self,
        line_idx: usize,
        end_col: usize,
        pattern_chars: &[char],
    ) -> Option<(usize, usize)> {
        if pattern_chars.is_empty() {
            return None;
        }
        let line_len = self.line_len(line_idx);
        let line_chars: Vec<char> = self
            .line(line_idx)
            .map(|l| l.chars().take(line_len).collect())?;
        let max_start = (end_col).saturating_sub(pattern_chars.len()).min(line_len.saturating_sub(pattern_chars.len()));
        for col in (0..=max_start).rev() {
            if col + pattern_chars.len() <= line_chars.len()
                && line_chars[col..].starts_with(pattern_chars)
            {
                return Some((line_idx, col));
            }
        }
        None
    }

    /// Get the filename (just the name, not the full path)
    pub fn filename(&self) -> Option<String> {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    /// Check if buffer is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.text.len_chars() == 0
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
