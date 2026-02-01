use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use ratatui_explorer::Input as ExplorerInput;

use crate::app::App;
use crate::editor::{Editor, EditorCommand, PendingNormal};
use crate::mode::Mode;

/// The result of handling an input event
pub enum InputResult {
    /// Continue running
    Continue,
    /// Exit the editor
    Exit,
}

/// Handle a key event; dispatches to file explorer or editor based on focus.
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> InputResult {
    // Space then E (in normal mode): toggle sidebar visibility or open current directory
    if app.pending_space_e {
        app.pending_space_e = false;
        if matches!(key.code, KeyCode::Char('e') | KeyCode::Char('E'))
            && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
            && app.editor.mode == Mode::Normal
        {
            app.toggle_sidebar_or_open_current_dir();
            return InputResult::Continue;
        }
    }

    // Ctrl+w: start window-switch sequence
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.pending_ctrl_w = true;
        return InputResult::Continue;
    }

    // Second key after Ctrl+w: w toggles focus
    if app.pending_ctrl_w {
        app.pending_ctrl_w = false;
        if key.code == KeyCode::Char('w') && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) {
            if app.directory_state.is_some() {
                app.focus_on_explorer = !app.focus_on_explorer;
            }
            return InputResult::Continue;
        }
    }

    if app.focus_on_explorer {
        // R or F5: refresh the file list
        if matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5)) {
            if let Some(ref mut dir) = app.directory_state {
                match dir.refresh() {
                    Ok(()) => app.editor.set_status("Explorer refreshed"),
                    Err(e) => app.editor.set_status(&format!("{}", e)),
                }
            }
            return InputResult::Continue;
        }
        let is_enter = matches!(key.code, KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right);
        if is_enter {
            if let Some(ref dir) = app.directory_state {
                let current = dir.file_explorer().current();
                let path = current.path();
                if path.is_file() {
                    match path.to_str() {
                        Some(s) => {
                            let path_str = s.to_string();
                            match app.editor.open_file_into_new_buffer(&path_str) {
                                Ok(()) => {
                                    app.focus_on_explorer = false;
                                    app.editor.set_status(&format!("Opened {}", path_str));
                                }
                                Err(e) => {
                                    app.editor.set_status(&format!("{}", e));
                                }
                            }
                            return InputResult::Continue;
                        }
                        None => {
                            app.editor.set_status("Path is not valid UTF-8");
                            return InputResult::Continue;
                        }
                    }
                }
            }
        }
        if let Some(ref mut dir) = app.directory_state {
            let event = Event::Key(key);
            let input = ExplorerInput::from(&event);
            if let Err(e) = dir.file_explorer_mut().handle(input) {
                app.editor.set_status(&format!("{}", e));
            }
        }
        return InputResult::Continue;
    }

    // In normal mode with editor focus, Space starts the "Space then E" shortcut
    if !app.focus_on_explorer
        && app.editor.mode == Mode::Normal
        && key.code == KeyCode::Char(' ')
    {
        app.pending_space_e = true;
        return InputResult::Continue;
    }

    handle_editor(app, key)
}

/// Handle key event for the editor (when focus is on the editor pane).
fn handle_editor(app: &mut App, key: KeyEvent) -> InputResult {
    let editor = &mut app.editor;
    match editor.mode {
        Mode::Normal => handle_normal_mode(editor, key),
        Mode::Insert => handle_insert_mode(editor, key),
        Mode::Command => handle_command_mode(app, key),
        Mode::Search => handle_search_mode(editor, key),
    }
}

/// Handle key events in normal mode
fn handle_normal_mode(editor: &mut Editor, key: KeyEvent) -> InputResult {
    // Clear any previous status message on new input
    editor.clear_status();

    // Handle or cancel pending two-key / replace action
    match editor.pending_normal {
        PendingNormal::SecondG if key.code != KeyCode::Char('g') => {
            editor.clear_pending_normal();
        }
        PendingNormal::SecondD if key.code != KeyCode::Char('d') => {
            editor.clear_pending_normal();
        }
        PendingNormal::ReplaceChar => {
            if let KeyCode::Char(c) = key.code {
                editor.replace_char_at_cursor(c);
            }
            editor.clear_pending_normal();
            if matches!(key.code, KeyCode::Char(_)) {
                return InputResult::Continue;
            }
        }
        _ => {}
    }

    match key.code {
        // Movement keys
        KeyCode::Char('h') | KeyCode::Left => editor.move_left(),
        KeyCode::Char('j') | KeyCode::Down => editor.move_down(),
        KeyCode::Char('k') | KeyCode::Up => editor.move_up(),
        KeyCode::Char('l') | KeyCode::Right => editor.move_right(),

        // Word movement
        KeyCode::Char('w') => editor.move_word_forward(),
        KeyCode::Char('b') => editor.move_word_backward(),
        KeyCode::Char('e') => editor.move_to_end_of_word(),
        KeyCode::Char('W') => editor.move_word_forward(),
        KeyCode::Char('B') => editor.move_word_backward(),
        KeyCode::Char('E') => editor.move_to_end_of_word(),

        // Line movement
        KeyCode::Char('0') => editor.move_to_line_start(),
        KeyCode::Char('$') => editor.move_to_line_end(),
        KeyCode::Char('^') => editor.move_to_first_non_blank(),
        KeyCode::Char('G') => editor.move_to_last_line(),
        KeyCode::Char('{') => editor.move_paragraph_prev(),
        KeyCode::Char('}') => editor.move_paragraph_next(),
        KeyCode::Char('g') => {
            if editor.pending_normal == PendingNormal::SecondG {
                editor.move_to_first_line();
                editor.clear_pending_normal();
            } else {
                editor.pending_normal = PendingNormal::SecondG;
            }
        }

        // Enter insert mode
        KeyCode::Char('i') => editor.enter_insert_mode(),
        KeyCode::Char('a') => editor.enter_insert_mode_append(),
        KeyCode::Char('A') => editor.enter_insert_mode_end(),
        KeyCode::Char('I') => editor.enter_insert_mode_start(),
        KeyCode::Char('o') => editor.open_line_below(),
        KeyCode::Char('O') => editor.open_line_above(),

        // Delete character
        KeyCode::Char('x') => editor.delete_char_at_cursor(),
        KeyCode::Char('D') => editor.delete_to_end_of_line(),
        KeyCode::Char('J') => editor.join_lines(),
        KeyCode::Char('d') => {
            if editor.pending_normal == PendingNormal::SecondD {
                editor.delete_current_line();
                editor.clear_pending_normal();
            } else {
                editor.pending_normal = PendingNormal::SecondD;
            }
        }
        KeyCode::Char('r') => editor.pending_normal = PendingNormal::ReplaceChar,

        // Enter command mode
        KeyCode::Char(':') => editor.enter_command_mode(),
        KeyCode::Char('/') => editor.enter_search_mode(),

        // Search repeat
        KeyCode::Char('n') => {
            editor.repeat_search_forward();
        }
        KeyCode::Char('N') => {
            editor.repeat_search_backward();
        }

        // Ctrl+C will set the mode to normal_mode 
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return return_to_normal_mode(editor);
        }

        _ => {}
    }

    InputResult::Continue
}

/// Handle key events in insert mode
fn handle_insert_mode(editor: &mut Editor, key: KeyEvent) -> InputResult {
    match key.code {
        // Exit insert mode
        KeyCode::Esc => editor.enter_normal_mode(),

        // Backspace
        KeyCode::Backspace => editor.backspace(),

        // Enter/Return
        KeyCode::Enter => editor.insert_newline(),

        // Regular character input
        KeyCode::Char(c) => {
            // Handle Ctrl+C in insert mode too
            if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
               return return_to_normal_mode(editor);
            }
            editor.insert_char(c);
        }

        // Arrow keys work in insert mode too
        KeyCode::Left => editor.move_left(),
        KeyCode::Right => editor.move_right(),
        KeyCode::Up => editor.move_up(),
        KeyCode::Down => editor.move_down(),

        // Tab inserts spaces (4 spaces)
        KeyCode::Tab => {
            for _ in 0..4 {
                editor.insert_char(' ');
            }
        }

        _ => {}
    }

    InputResult::Continue
}

/// Handle key events in search mode (vim /)
fn handle_search_mode(editor: &mut Editor, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => return return_to_normal_mode(editor),
        KeyCode::Enter => {
            editor.search_forward();
            editor.command_buffer.clear();
            editor.enter_normal_mode();
        }
        KeyCode::Backspace => {
            if editor.command_buffer.is_empty() {
                return return_to_normal_mode(editor);
            }
            editor.command_buffer.pop();
        }
        KeyCode::Char(c) => editor.command_buffer.push(c),
        _ => {}
    }
    InputResult::Continue
}

/// Handle key events in command mode
fn handle_command_mode(app: &mut App, key: KeyEvent) -> InputResult {
    match key.code {
        // Cancel command
        KeyCode::Esc => {
           return return_to_normal_mode(&mut app.editor);
        }

        // Execute command
        KeyCode::Enter => {
            let (is_toggle_sidebar, cmd_result) = {
                let editor = &mut app.editor;
                let cmd = editor.command_buffer.trim();
                let is_toggle = cmd == "e." || cmd == "Explore" || cmd == "Lexplore";
                if is_toggle {
                    editor.command_buffer.clear();
                    editor.mode = Mode::Normal;
                    (true, None)
                } else {
                    let result = editor.execute_command();
                    (false, result)
                }
            };
            if is_toggle_sidebar {
                app.toggle_sidebar_or_open_current_dir();
                return InputResult::Continue;
            }
            if let Some(cmd_result) = cmd_result {
                let editor = &mut app.editor;
                match cmd_result {
                    EditorCommand::Quit => {
                        if editor.current_buffer().modified {
                            editor.set_status("No write since last change (add ! to override)");
                            return InputResult::Continue;
                        }
                        return InputResult::Exit;
                    }
                    EditorCommand::ForceQuit => {
                        return InputResult::Exit;
                    }
                }
            }
        }

        // Backspace in command buffer
        KeyCode::Backspace => {
            let editor = &mut app.editor;
            if editor.command_buffer.is_empty() {
                editor.mode = Mode::Normal;
            } else {
                editor.command_buffer.pop();
            }
        }

        // Add character to command buffer
        KeyCode::Char(c) => {
            app.editor.command_buffer.push(c);
        }

        _ => {}
    }

    InputResult::Continue
}
fn return_to_normal_mode(editor: &mut Editor) ->InputResult {
    editor.command_buffer.clear();
    editor.enter_normal_mode();
    return InputResult::Continue;
}
