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

    /// Load a buffer from a file
    pub fn from_file(path: &str) -> Result<Self, IoError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let text = Rope::from_reader(reader)?;

        Ok(Self {
            text,
            file_path: Some(PathBuf::from(path)),
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
