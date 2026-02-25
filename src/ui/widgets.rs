use ratatui::{
    prelude::Stylize,
    widgets::{Block, Borders, Widget},
};

use super::Theme;

pub struct EditorView {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub show_line_numbers: bool,
    pub scroll_offset: usize,
    pub theme: Theme,
    pub cursor_blink_on: bool,
}

impl EditorView {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            show_line_numbers: true,
            scroll_offset: 0,
            theme: Theme::monokai_pro(),
            cursor_blink_on: true,
        }
    }
}

impl Default for EditorView {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for EditorView {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        // Create a bordered block
        let block = Block::default()
            .bg(self.theme.background)
            .fg(self.theme.foreground)
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(self.theme.border));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let line_count = self.lines.len();
        let line_number_width = if self.show_line_numbers && line_count > 0 {
            (line_count.to_string().len() + 3).max(5) as u16
        } else {
            2
        };

        // Clear the editor area
        let clear_style = ratatui::style::Style::default()
            .bg(self.theme.background)
            .fg(self.theme.foreground);

        for y in 0..inner.height {
            for x in 0..inner.width {
                buf[(inner.x + x, inner.y + y)]
                    .set_char(' ')
                    .set_style(clear_style);
            }
        }

        // Render visible lines
        let visible_lines = inner.height as usize;

        for y in 0..visible_lines {
            let line_idx = self.scroll_offset + y;

            if line_idx >= self.lines.len() {
                break;
            }

            let line_text = self.lines.get(line_idx).cloned().unwrap_or_default();
            let is_current_line = line_idx == self.cursor_line;

            // Render line number with separator
            if self.show_line_numbers {
                let line_num_str = format!(
                    "{:>width$} │",
                    line_idx + 1,
                    width = (line_number_width as usize - 2)
                );

                for (x, c) in line_num_str.chars().enumerate() {
                    let pos_x = inner.x + x as u16;
                    let pos_y = inner.y + y as u16;
                    if pos_x < inner.x + line_number_width {
                        let style = if is_current_line {
                            ratatui::style::Style::default()
                                .bg(self.theme.cursor_line)
                                .fg(self.theme.line_number_current)
                        } else {
                            ratatui::style::Style::default()
                                .bg(self.theme.background)
                                .fg(self.theme.line_number)
                        };
                        buf[(pos_x, pos_y)].set_char(c).set_style(style);
                    }
                }
            }

            // Render line content
            let text_start = inner.x + line_number_width;

            // Horizontal scroll - keep cursor visible
            let max_visible = (inner.width.saturating_sub(line_number_width + 1)) as usize;
            let line_len = line_text.len();
            let display_col = if line_len > max_visible {
                if self.cursor_col > max_visible * 2 / 3 {
                    (self.cursor_col.saturating_sub(max_visible / 3))
                        .min(line_len.saturating_sub(max_visible))
                } else {
                    0
                }
            } else {
                0
            };

            let visible_text: String = line_text
                .chars()
                .skip(display_col)
                .take(max_visible)
                .collect();
            let pos_y = inner.y + y as u16;
            let cursor_rel_col = self.cursor_col.saturating_sub(display_col);

            for (x, c) in visible_text.chars().enumerate() {
                let col = text_start as usize + x;
                if col < (inner.x + inner.width - 1) as usize {
                    let abs_col = display_col + x;
                    let is_cursor = is_current_line && abs_col == self.cursor_col;

                    let style = if is_cursor && self.cursor_blink_on {
                        ratatui::style::Style::default()
                            .bg(self.theme.cursor)
                            .fg(self.theme.background)
                    } else {
                        ratatui::style::Style::default()
                            .bg(if is_current_line {
                                self.theme.cursor_line
                            } else {
                                self.theme.background
                            })
                            .fg(self.theme.foreground)
                    };

                    buf[(col as u16, pos_y)].set_char(c).set_style(style);
                }
            }

            // Render cursor on empty line or at end of line
            if is_current_line {
                let cursor_pos = text_start + cursor_rel_col as u16;
                if cursor_pos < inner.x + inner.width - 1 {
                    let existing_char = if cursor_rel_col < line_text.len() {
                        line_text.chars().nth(cursor_rel_col)
                    } else {
                        None
                    };

                    if self.cursor_blink_on {
                        let cursor_char = existing_char.unwrap_or(' ');
                        let style = ratatui::style::Style::default()
                            .bg(self.theme.cursor)
                            .fg(self.theme.background);

                        buf[(cursor_pos, pos_y)]
                            .set_char(cursor_char)
                            .set_style(style);
                    } else if existing_char.is_none() {
                        buf[(cursor_pos, pos_y)].set_char(' ').set_style(
                            ratatui::style::Style::default()
                                .bg(self.theme.cursor_line)
                                .fg(self.theme.foreground),
                        );
                    }
                }
            }

            // Draw vertical border on right
            if line_idx < self.lines.len() {
                let right_x = inner.x + inner.width - 1;
                buf[(right_x, pos_y)]
                    .set_char('│')
                    .set_style(ratatui::style::Style::default().fg(self.theme.border));
            }
        }

        // Render scrollbar
        if self.lines.len() > visible_lines {
            let scrollbar_height = inner.height as f64;
            let total_lines = self.lines.len();
            let scroll_ratio = (self.scroll_offset as f64 / total_lines as f64) * scrollbar_height;
            let thumb_size =
                ((visible_lines as f64 / total_lines as f64) * scrollbar_height).max(1.0) as u16;
            let thumb_pos = scroll_ratio as u16;

            for y in 0..inner.height {
                let pos_y = inner.y + y as u16;
                let thumb_start = thumb_pos as u16;
                let thumb_end = thumb_start + thumb_size;
                let style = if y >= thumb_start && y < thumb_end {
                    ratatui::style::Style::default().fg(self.theme.accent)
                } else {
                    ratatui::style::Style::default().fg(self.theme.scrollbar)
                };
                buf[(inner.x + inner.width - 1, pos_y)]
                    .set_char('█')
                    .set_style(style);
            }
        }
    }
}

pub struct TitleBar {
    pub file_name: String,
    pub theme: Theme,
}

impl TitleBar {
    pub fn new() -> Self {
        Self {
            file_name: "Nova".to_string(),
            theme: Theme::monokai_pro(),
        }
    }
}

impl Default for TitleBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TitleBar {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let style = ratatui::style::Style::default()
            .bg(self.theme.title_bg)
            .fg(self.theme.title_fg);

        let title = format!(" {} ", self.file_name);

        // Draw left border
        buf[(area.x, area.y)].set_char('│').set_style(style);

        // Draw title
        for (x, c) in title.chars().enumerate() {
            if x < area.width as usize - 2 {
                buf[(area.x + 1 + x as u16, area.y)]
                    .set_char(c)
                    .set_style(style);
            }
        }

        // Fill rest with spaces
        for x in (title.len() + 1)..(area.width as usize) {
            buf[(area.x + x as u16, area.y)]
                .set_char(' ')
                .set_style(style);
        }

        // Draw right border
        buf[(area.x + area.width - 1, area.y)]
            .set_char('│')
            .set_style(style);
    }
}

pub struct StatusBar {
    pub file_name: String,
    pub modified: bool,
    pub line: usize,
    pub col: usize,
    pub language: String,
    pub theme: Theme,
    pub search_mode: bool,
    pub search_text: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            file_name: String::from("[No Name]"),
            modified: false,
            line: 1,
            col: 1,
            language: "plaintext".to_string(),
            theme: Theme::monokai_pro(),
            search_mode: false,
            search_text: String::new(),
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for StatusBar {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let width = area.width as usize;

        let style = ratatui::style::Style::default()
            .bg(self.theme.status_bar_bg)
            .fg(self.theme.status_bar_fg);

        let accent_style = ratatui::style::Style::default()
            .bg(self.theme.status_bar_bg)
            .fg(self.theme.accent);

        // Clear
        for x in 0..area.width {
            buf[(area.x + x, area.y)].set_char(' ').set_style(style);
        }

        let (left, right) = if self.search_mode {
            (
                format!(" Search: {}", self.search_text),
                "ESC cancel | ENTER go".to_string(),
            )
        } else {
            let file_icon = if self.modified { "●" } else { "○" };
            let file_info = if self.file_name.is_empty() || self.file_name == "[No Name]" {
                "untitled".to_string()
            } else {
                self.file_name.clone()
            };
            (
                format!(" {} {} ", file_icon, file_info),
                format!(
                    " Ln {:>width$} Col {:>width2$} │ {:^10} ",
                    self.line,
                    self.col,
                    self.language.to_uppercase(),
                    width = 4,
                    width2 = 3
                ),
            )
        };

        // Left side
        for (x, c) in left.chars().enumerate() {
            if x < width {
                let s = if self.search_mode {
                    accent_style
                } else {
                    style
                };
                buf[(area.x + x as u16, area.y)].set_char(c).set_style(s);
            }
        }

        // Right side
        let right_start = width.saturating_sub(right.len());
        for (x, c) in right.chars().enumerate() {
            let pos = right_start + x;
            if pos < width {
                buf[(area.x + pos as u16, area.y)]
                    .set_char(c)
                    .set_style(style);
            }
        }

        // Borders
        buf[(area.x, area.y)].set_char('│').set_style(style);
        buf[(area.x + area.width - 1, area.y)]
            .set_char('│')
            .set_style(style);
    }
}

pub struct HelpBar {
    pub shortcuts: Vec<(&'static str, &'static str)>,
    pub visible: bool,
    pub theme: Theme,
}

impl HelpBar {
    pub fn new() -> Self {
        Self {
            shortcuts: vec![],
            visible: true,
            theme: Theme::monokai_pro(),
        }
    }
}

impl Default for HelpBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HelpBar {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        if !self.visible {
            return;
        }

        let style = ratatui::style::Style::default()
            .bg(self.theme.help_bar_bg)
            .fg(self.theme.help_bar_fg);

        let accent_style = ratatui::style::Style::default()
            .bg(self.theme.help_bar_bg)
            .fg(self.theme.accent);

        // Clear
        for x in 0..area.width {
            buf[(area.x + x, area.y)].set_char(' ').set_style(style);
        }

        // Build shortcut text with key in accent color
        let mut x_pos = 1;

        for (key, desc) in &self.shortcuts {
            // Key in accent
            for c in key.chars() {
                if x_pos < area.width as usize - 1 {
                    buf[(area.x + x_pos as u16, area.y)]
                        .set_char(c)
                        .set_style(accent_style);
                    x_pos += 1;
                }
            }

            // Separator
            if x_pos < area.width as usize - 1 {
                buf[(area.x + x_pos as u16, area.y)]
                    .set_char(':')
                    .set_style(style);
                x_pos += 1;
            }

            // Description in normal
            for c in desc.chars() {
                if x_pos < area.width as usize - 1 {
                    buf[(area.x + x_pos as u16, area.y)]
                        .set_char(c)
                        .set_style(style);
                    x_pos += 1;
                }
            }

            // Separator between groups
            if x_pos < area.width as usize - 3 {
                buf[(area.x + x_pos as u16, area.y)]
                    .set_char(' ')
                    .set_style(style);
                x_pos += 1;
            }
        }

        // Borders
        buf[(area.x, area.y)].set_char('│').set_style(style);
        buf[(area.x + area.width - 1, area.y)]
            .set_char('│')
            .set_style(style);
    }
}
