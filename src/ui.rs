use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::editor::Editor;
use crate::mode::Mode;

/// The width reserved for line numbers
const LINE_NUMBER_WIDTH: u16 = 6;

/// Render the editor UI
pub fn render(frame: &mut Frame, editor: &mut Editor) {
    let size = frame.area();

    // Create the main layout: text area + status bar + command line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Text area
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Command line
        ])
        .split(size);

    // Render the text area (line numbers + content)
    render_text_area(frame, editor, chunks[0]);

    // Render the status bar
    render_status_bar(frame, editor, chunks[1]);

    // Render the command line
    render_command_line(frame, editor, chunks[2]);

    // Position the cursor
    position_cursor(frame, editor, chunks[0]);
}

/// Render the main text editing area with line numbers
fn render_text_area(frame: &mut Frame, editor: &mut Editor, area: Rect) {
    // Split into line numbers and text content
    let text_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(LINE_NUMBER_WIDTH),
            Constraint::Min(1),
        ])
        .split(area);

    let line_numbers_area = text_chunks[0];
    let content_area = text_chunks[1];

    // Calculate visible lines
    let visible_height = content_area.height as usize;
    editor.adjust_viewport_with_height(visible_height);

    let start_line = editor.viewport_offset;
    let end_line = (start_line + visible_height).min(editor.buffer.line_count());

    // Render line numbers
    let mut line_number_lines = Vec::new();
    for line_idx in start_line..end_line {
        let num_str = format!("{:>4} ", line_idx + 1);
        let style = if line_idx == editor.cursor.line {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        line_number_lines.push(Line::from(Span::styled(num_str, style)));
    }

    // Fill remaining lines with tildes (like vim)
    for _ in end_line..start_line + visible_height {
        line_number_lines.push(Line::from(Span::styled(
            "    ~ ",
            Style::default().fg(Color::Blue),
        )));
    }

    let line_numbers = Paragraph::new(line_number_lines);
    frame.render_widget(line_numbers, line_numbers_area);

    // Render text content
    let mut content_lines = Vec::new();
    for line_idx in start_line..end_line {
        if let Some(line) = editor.buffer.line(line_idx) {
            let line_str: String = line.chars().filter(|c| *c != '\n').collect();
            content_lines.push(Line::from(line_str));
        }
    }

    // Fill remaining lines
    for _ in end_line..start_line + visible_height {
        content_lines.push(Line::from(""));
    }

    let content = Paragraph::new(content_lines);
    frame.render_widget(content, content_area);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, editor: &Editor, area: Rect) {
    let mode_style = match editor.mode {
        Mode::Normal => Style::default().bg(Color::Blue).fg(Color::White),
        Mode::Insert => Style::default().bg(Color::Green).fg(Color::Black),
        Mode::Command => Style::default().bg(Color::Yellow).fg(Color::Black),
        Mode::Search => Style::default().bg(Color::Magenta).fg(Color::White),
    };

    let filename = editor
        .buffer
        .filename()
        .unwrap_or_else(|| "[No Name]".to_string());

    let modified = if editor.buffer.modified { "[+]" } else { "" };

    let position = format!(
        "{}:{} ",
        editor.cursor.line + 1,
        editor.cursor.col + 1
    );

    // Calculate available space
    let mode_text = format!(" {} ", editor.mode.as_str());
    let file_text = format!(" {}{} ", filename, modified);
    let left_len = mode_text.len() + file_text.len();
    let right_len = position.len();
    let padding = area.width as usize - left_len - right_len;

    let status_line = Line::from(vec![
        Span::styled(mode_text, mode_style.add_modifier(Modifier::BOLD)),
        Span::styled(file_text, Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::styled(
            " ".repeat(padding.max(0)),
            Style::default().bg(Color::DarkGray),
        ),
        Span::styled(
            position,
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
    ]);

    let status_bar = Paragraph::new(status_line);
    frame.render_widget(status_bar, area);
}

/// Render the command line
fn render_command_line(frame: &mut Frame, editor: &Editor, area: Rect) {
    let content = match editor.mode {
        Mode::Command => format!(":{}", editor.command_buffer),
        Mode::Search => format!("/{}", editor.command_buffer),
        _ => editor
            .status_message
            .clone()
            .unwrap_or_default(),
    };

    let command_line = Paragraph::new(content);
    frame.render_widget(command_line, area);
}

/// Position the cursor in the frame
fn position_cursor(frame: &mut Frame, editor: &Editor, text_area: Rect) {
    // In command or search mode, cursor is in the command line
    if editor.mode == Mode::Command || editor.mode == Mode::Search {
        let prefix_len = 1; // ':' or '/'
        let x = prefix_len + editor.command_buffer.len() as u16;
        let y = frame.area().height - 1; // Last line
        frame.set_cursor_position((x, y));
        return;
    }

    // Calculate cursor position in text area
    let content_x = LINE_NUMBER_WIDTH;
    let visible_line = editor.cursor.line.saturating_sub(editor.viewport_offset);

    let x = content_x + editor.cursor.col as u16;
    let y = text_area.y + visible_line as u16;

    // Only show cursor if within visible area
    if y < text_area.y + text_area.height {
        frame.set_cursor_position((x, y));
    }
}
