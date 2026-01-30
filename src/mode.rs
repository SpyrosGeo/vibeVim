/// The editing modes of the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal mode - for navigation and commands
    Normal,
    /// Insert mode - for typing text
    Insert,
    /// Command mode - for entering commands (after pressing :)
    Command,
    /// Search mode - for searching in buffer
    Search,
}

impl Mode {
    /// Get a display name for the mode
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Search => "SEARCH",
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}
