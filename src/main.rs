mod app;
mod buffer;
mod editor;
mod input;
mod mode;
mod ui;

use std::io::{self, stdout};
use std::panic;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

fn main() -> io::Result<()> {
    // Set up panic handler to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal
        let _ = restore_terminal();
        // Call the original panic handler
        original_hook(panic_info);
    }));

    // Initialize terminal
    let terminal = setup_terminal()?;
    let mut terminal = terminal;

    // Create the application
    let args: Vec<String> = std::env::args().collect();
    let mut app = if args.len() > 1 {
        match App::with_file(&args[1]) {
            Ok(app) => app,
            Err(_) => {
                // If file doesn't exist, create new buffer with that filename
                let mut app = App::new();
                app.editor.buffer.file_path = Some(std::path::PathBuf::from(&args[1]));
                app.editor.set_status(&format!("New file: {}", &args[1]));
                app
            }
        }
    } else {
        App::new()
    };

    // Run the application
    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal()?;

    result
}

/// Set up the terminal for TUI rendering
fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
