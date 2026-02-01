//! Directory / file explorer state for the sidebar.
//! Uses `ratatui-explorer` for the file list (single-dir view with enter/leave dirs).
//! Hidden files are shown (ratatui-explorer does not filter dotfiles).

use std::io;
use std::path::Path;

use ratatui::style::{Color, Style};
use ratatui_explorer::{FileExplorer, Theme};

/// State for the directory sidebar when opening a directory (e.g. `vibeVim .`).
/// Wraps ratatui-explorer's FileExplorer; supports j/k navigation and enter dir / parent.
pub struct DirectoryState {
    /// File explorer widget state (cwd, file list, selection).
    pub file_explorer: FileExplorer,
}

impl DirectoryState {
    /// Create directory state at the given path. The path must be a directory.
    /// Uses a theme with no inner block (outer block is in ui) but visible selection and item styles.
    pub fn new(path: &Path) -> io::Result<Self> {
        let theme = Theme::new()
            .with_item_style(Style::default().fg(Color::White))
            .with_dir_style(Style::default().fg(Color::LightBlue))
            .with_highlight_item_style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .with_highlight_dir_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray));
        let mut file_explorer = FileExplorer::with_theme(theme)?;
        file_explorer.set_cwd(path)?;
        Ok(Self { file_explorer })
    }

    /// Reference to the file explorer for rendering and input.
    pub fn file_explorer(&self) -> &FileExplorer {
        &self.file_explorer
    }

    /// Mutable reference for handling input and navigation.
    pub fn file_explorer_mut(&mut self) -> &mut FileExplorer {
        &mut self.file_explorer
    }

    /// Re-read the current directory (e.g. after external file changes).
    pub fn refresh(&mut self) -> io::Result<()> {
        let cwd = self.file_explorer.cwd().clone();
        self.file_explorer.set_cwd(cwd)
    }
}
