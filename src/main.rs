mod app;
mod buffer;
mod dir;
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
        let path_arg = &args[1];
        let (path_opt, path_error) = if path_arg == "." {
            match std::env::current_dir() {
                Ok(p) => (Some(p), None),
                Err(e) => (None, Some(e)),
            }
        } else {
            (Some(std::path::PathBuf::from(path_arg)), None)
        };

        if let Some(path) = path_opt {
            if path.is_dir() {
                match App::with_directory(&path) {
                    Ok(app) => app,
                    Err(e) => {
                        let mut app = App::new();
                        app.editor.set_status(&format!("Cannot open directory: {}", e));
                        app
                    }
                }
            } else {
                match App::with_file(path_arg) {
                    Ok(app) => app,
                    Err(_) => {
                        let mut app = App::new();
                        app.editor.current_buffer_mut().file_path =
                            Some(std::path::PathBuf::from(path_arg));
                        app.editor.set_status(&format!("New file: {}", path_arg));
                        app
                    }
                }
            }
        } else {
            let mut app = App::new();
            app.editor
                .set_status(&format!("Cannot resolve path: {}", path_error.unwrap()));
            app
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
