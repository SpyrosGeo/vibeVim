use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::editor::{Editor, EditorCommand};
use crate::mode::Mode;

/// The result of handling an input event
pub enum InputResult {
    /// Continue running
    Continue,
    /// Exit the editor
    Exit,
}

/// Handle a key event based on the current mode
pub fn handle_key_event(editor: &mut Editor, key: KeyEvent) -> InputResult {
    match editor.mode {
        Mode::Normal => handle_normal_mode(editor, key),
        Mode::Insert => handle_insert_mode(editor, key),
        Mode::Command => handle_command_mode(editor, key),
    }
}

/// Handle key events in normal mode
fn handle_normal_mode(editor: &mut Editor, key: KeyEvent) -> InputResult {
    // Clear any previous status message on new input
    editor.clear_status();

    match key.code {
        // Movement keys
        KeyCode::Char('h') | KeyCode::Left => editor.move_left(),
        KeyCode::Char('j') | KeyCode::Down => editor.move_down(),
        KeyCode::Char('k') | KeyCode::Up => editor.move_up(),
        KeyCode::Char('l') | KeyCode::Right => editor.move_right(),

        // Word movement
        KeyCode::Char('w') => editor.move_word_forward(),
        KeyCode::Char('b') => editor.move_word_backward(),

        // Line movement
        KeyCode::Char('0') => editor.move_to_line_start(),
        KeyCode::Char('$') => editor.move_to_line_end(),

        // Enter insert mode
        KeyCode::Char('i') => editor.enter_insert_mode(),
        KeyCode::Char('a') => editor.enter_insert_mode_append(),
        KeyCode::Char('A') => editor.enter_insert_mode_end(),
        KeyCode::Char('I') => editor.enter_insert_mode_start(),

        // Delete character
        KeyCode::Char('x') => editor.delete_char_at_cursor(),

        // Enter command mode
        KeyCode::Char(':') => editor.enter_command_mode(),

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

/// Handle key events in command mode
fn handle_command_mode(editor: &mut Editor, key: KeyEvent) -> InputResult {
    match key.code {
        // Cancel command
        KeyCode::Esc => {
           return return_to_normal_mode(editor);
        }

        // Execute command
        KeyCode::Enter => {
            if let Some(cmd) = editor.execute_command() {
                match cmd {
                    EditorCommand::Quit => {
                        if editor.buffer.modified {
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
            if editor.command_buffer.is_empty() {
                editor.mode = Mode::Normal;
            } else {
                editor.command_buffer.pop();
            }
        }

        // Add character to command buffer
        KeyCode::Char(c) => {
            editor.command_buffer.push(c);
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
