use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Margin, Rect},
    style::Style,
    widgets::Paragraph,
    Terminal,
};

use crate::buffer::Buffer;
use crate::config::Settings;
use crate::ui::{widgets::TitleBar, EditorView, HelpBar, StatusBar, Theme};

mod buffer;
mod config;
mod ui;

#[derive(Clone, Debug)]
enum EditOp {
    Insert {
        pos: usize,
        text: String,
    },
    Delete {
        pos: usize,
        text: String,
    },
    #[allow(dead_code)]
    Replace {
        pos: usize,
        #[allow(dead_code)]
        old_len: usize,
        #[allow(dead_code)]
        old_text: String,
        new_text: String,
    },
}

struct UndoHistory {
    ops: Vec<EditOp>,
    pos: usize,
}

impl UndoHistory {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            pos: 0,
        }
    }

    fn push(&mut self, op: EditOp) {
        self.ops.truncate(self.pos);
        self.ops.push(op);
        self.pos += 1;
        if self.ops.len() > 1000 {
            self.ops.remove(0);
            self.pos -= 1;
        }
    }

    fn undo(&mut self, buffer: &mut Buffer) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        match &self.ops[self.pos] {
            EditOp::Insert { pos, text } => {
                buffer.delete(*pos, text.len());
                true
            }
            EditOp::Delete { pos, text } => {
                buffer.insert(*pos, text);
                true
            }
            EditOp::Replace {
                pos,
                old_len: _,
                old_text,
                new_text,
            } => {
                buffer.delete(*pos, new_text.len());
                buffer.insert(*pos, old_text);
                true
            }
        }
    }

    fn redo(&mut self, buffer: &mut Buffer) -> bool {
        if self.pos >= self.ops.len() {
            return false;
        }
        match &self.ops[self.pos] {
            EditOp::Insert { pos, text } => {
                buffer.insert(*pos, text);
                self.pos += 1;
                true
            }
            EditOp::Delete { pos, text } => {
                buffer.delete(*pos, text.len());
                self.pos += 1;
                true
            }
            EditOp::Replace {
                pos,
                old_len: _,
                old_text: _,
                new_text,
            } => {
                buffer.insert(*pos, new_text);
                self.pos += 1;
                true
            }
        }
    }

    fn clear(&mut self) {
        self.ops.clear();
        self.pos = 0;
    }
}

#[derive(Clone, PartialEq)]
enum EditorMode {
    Normal,
    Search {
        query: String,
        case_sensitive: bool,
        backward: bool,
    },
    Replace {
        search: String,
        replace: String,
        case_sensitive: bool,
        all: bool,
        confirmed: bool,
    },
    GoToLine,
    Confirm {
        title: String,
        message: String,
        options: Vec<String>,
        selected: usize,
    },
    Input {
        title: String,
        input: String,
        history: Vec<String>,
    },
    Help,
}

static TIPS: &[&str] = &[
    "Press Ctrl+F to search for text in the file",
    "Press Ctrl+\\ to find and replace text",
    "Press Ctrl+G to jump to a specific line number",
    "Use Ctrl+Z to undo and Ctrl+Y to redo changes",
    "Press Ctrl+T to cycle through different themes",
    "Press Ctrl+B to toggle line numbers on/off",
    "Enable soft tabs in config for spaces instead of tabs",
    "Auto-indent is on by default - it preserves code structure",
    "Press Ctrl+W to toggle word wrap for long lines",
    "Use Ctrl+O to open a file, Ctrl+S to save",
];

#[derive(Clone)]
enum PendingAction {
    SaveAndQuit,
    QuitWithoutSave,
    SaveAs(String),
    ReplaceAll(String, String),
}

struct Editor {
    buffer: Buffer,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    settings: Settings,
    theme: Theme,
    show_help: bool,
    show_line_numbers: bool,
    word_wrap: bool,
    should_quit: bool,
    undo: UndoHistory,
    mode: EditorMode,
    pending_action: Option<PendingAction>,
    quit_after_save: bool,
    cursor_blink_on: bool,
    last_cursor_time: std::time::Instant,
    screen_width: usize,
    screen_height: usize,
    current_tip: String,
}

impl Editor {
    fn new(initial_file: Option<String>, width: usize, height: usize) -> Self {
        let settings = Settings::load();
        let theme = Theme::get_theme(&settings.theme);

        let buffer = if let Some(file_path) = initial_file {
            let path = std::path::PathBuf::from(&file_path);
            if path.exists() {
                Buffer::from_file(path).unwrap_or_else(Buffer::new)
            } else {
                Buffer::for_new_file(path)
            }
        } else {
            Buffer::new()
        };

        Self {
            buffer,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            settings,
            theme,
            show_help: true,
            show_line_numbers: true,
            word_wrap: false,
            should_quit: false,
            undo: UndoHistory::new(),
            mode: EditorMode::Normal,
            pending_action: None,
            quit_after_save: false,
            cursor_blink_on: true,
            last_cursor_time: std::time::Instant::now(),
            screen_width: width,
            screen_height: height,
            current_tip: String::new(),
        }
    }

    fn get_random_tip() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as usize;
        TIPS[seed % TIPS.len()].to_string()
    }

    fn generate_tip(&mut self) {
        self.current_tip = Self::get_random_tip();
    }

    fn update_scroll(&mut self) {
        let view_height = self.screen_height.saturating_sub(3);
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
        if self.cursor_line >= self.scroll_offset + view_height {
            self.scroll_offset = self.cursor_line.saturating_sub(view_height - 1);
        }
        let max_scroll = self.buffer.num_lines().saturating_sub(view_height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn update_cursor_blink(&mut self) {
        let elapsed = self.last_cursor_time.elapsed().as_millis();
        if elapsed > 500 {
            self.cursor_blink_on = !self.cursor_blink_on;
            self.last_cursor_time = std::time::Instant::now();
        }
    }

    fn clamp_cursor(&mut self) {
        let num_lines = self.buffer.num_lines().saturating_sub(1);
        self.cursor_line = self.cursor_line.min(num_lines);
        self.cursor_col = self.cursor_col.min(self.buffer.line_len(self.cursor_line));
    }

    fn get_indent(&self, line: usize) -> String {
        let line_content = self.buffer.get_line(line);
        let mut indent = String::new();
        for ch in line_content.chars() {
            if ch == ' ' {
                indent.push(' ');
            } else if ch == '\t' {
                indent.push('\t');
            } else {
                break;
            }
        }
        indent
    }

    fn handle_key(&mut self, key: &event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        let mode = std::mem::replace(&mut self.mode, EditorMode::Normal);

        match mode {
            EditorMode::Normal => {
                self.handle_normal(key);
            }
            EditorMode::Search {
                query,
                case_sensitive,
                backward,
            } => {
                let (new_query, new_case, new_backward, should_exit) =
                    self.handle_search_owned(key, query, case_sensitive, backward);
                if should_exit {
                    self.mode = EditorMode::Normal;
                } else {
                    self.mode = EditorMode::Search {
                        query: new_query,
                        case_sensitive: new_case,
                        backward: new_backward,
                    };
                }
            }
            EditorMode::Replace {
                search,
                replace,
                case_sensitive,
                all,
                confirmed,
            } => {
                let (
                    new_search,
                    new_replace,
                    new_case,
                    new_all,
                    new_confirmed,
                    action,
                    should_exit,
                ) = self.handle_replace_owned(key, search, replace, case_sensitive, all, confirmed);
                if let Some(act) = action {
                    self.pending_action = Some(act);
                }
                if should_exit {
                    self.mode = EditorMode::Normal;
                } else {
                    self.mode = EditorMode::Replace {
                        search: new_search,
                        replace: new_replace,
                        case_sensitive: new_case,
                        all: new_all,
                        confirmed: new_confirmed,
                    };
                }
            }
            EditorMode::GoToLine => {
                let (line_num, should_exit) = self.handle_goto_owned(key);
                if should_exit {
                    self.mode = EditorMode::Normal;
                } else if let Some(num) = line_num {
                    self.goto_line(num);
                    self.mode = EditorMode::Normal;
                }
            }
            EditorMode::Confirm {
                title,
                message,
                options,
                selected,
            } => {
                let (new_title, new_message, new_options, new_selected, action) =
                    self.handle_confirm_owned(key, title, message, options, selected);
                let is_yes_selected =
                    new_options.get(new_selected).map(|s| s.as_str()) == Some("Yes");
                if let Some(act) = &action {
                    self.pending_action = Some(act.clone());
                }
                if action.is_none() && key.code == KeyCode::Enter && is_yes_selected {
                    self.quit_after_save = true;
                    self.mode = EditorMode::Input {
                        title: "Save As".into(),
                        input: "untitled.txt".into(),
                        history: Vec::new(),
                    };
                } else if key.code == KeyCode::Enter {
                    self.mode = EditorMode::Normal;
                } else if key.code == KeyCode::Esc {
                    self.mode = EditorMode::Normal;
                } else {
                    self.mode = EditorMode::Confirm {
                        title: new_title,
                        message: new_message,
                        options: new_options,
                        selected: new_selected,
                    };
                }
            }
            EditorMode::Input {
                title,
                input,
                history,
            } => {
                let (new_title, new_input, new_history, action) =
                    self.handle_input_owned(key, title, input, history);
                if let Some(act) = action {
                    self.pending_action = Some(act);
                }
                if key.code != KeyCode::Enter && key.code != KeyCode::Esc {
                    self.mode = EditorMode::Input {
                        title: new_title,
                        input: new_input,
                        history: new_history,
                    };
                } else {
                    self.mode = EditorMode::Normal;
                }
            }
            EditorMode::Help => {
                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('h') && key.modifiers == KeyModifiers::CONTROL)
                {
                    self.mode = EditorMode::Normal;
                }
            }
        }

        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::SaveAndQuit => {
                    let _ = self.buffer.save();
                    self.should_quit = true;
                }
                PendingAction::QuitWithoutSave => {
                    self.buffer.is_modified = false;
                    self.should_quit = true;
                }
                PendingAction::SaveAs(filename) => {
                    let path = std::path::PathBuf::from(filename);
                    let _ = self.buffer.save_as(path);
                    if self.quit_after_save {
                        self.should_quit = true;
                        self.quit_after_save = false;
                    }
                }
                PendingAction::ReplaceAll(search, replace) => {
                    let _count = self.buffer.replace(&search, &replace);
                    self.undo.clear();
                }
            }
        }
    }

    fn goto_line(&mut self, line_num: usize) {
        let num_lines = self.buffer.num_lines();
        if line_num > 0 && line_num <= num_lines {
            self.cursor_line = line_num - 1;
            self.cursor_col = 0;
            self.clamp_cursor();
            self.update_scroll();
        }
    }

    fn handle_normal(&mut self, k: &event::KeyEvent) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        match (k.code, k.modifiers) {
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.generate_tip();
                self.mode = EditorMode::Help;
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                if self.buffer.path.is_none() {
                    self.quit_after_save = true;
                    self.mode = EditorMode::Input {
                        title: "Save As".into(),
                        input: "untitled.txt".into(),
                        history: Vec::new(),
                    };
                } else if self.buffer.is_modified {
                    self.mode = EditorMode::Confirm {
                        title: "Quit".into(),
                        message: "Save changes?".into(),
                        options: vec!["Yes".into(), "No".into(), "Cancel".into()],
                        selected: 0,
                    };
                } else {
                    self.should_quit = true;
                }
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.buffer.path.is_none() {
                    self.mode = EditorMode::Input {
                        title: "Save As".into(),
                        input: "untitled.txt".into(),
                        history: Vec::new(),
                    };
                } else {
                    let _ = self.buffer.save();
                }
            }
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                self.open_file();
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                if self.undo.undo(&mut self.buffer) {
                    let (line, col) = self.buffer.get_line_col(0);
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                self.clamp_cursor();
                self.update_scroll();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                if self.undo.redo(&mut self.buffer) {
                    let (line, col) = self.buffer.get_line_col(0);
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                self.clamp_cursor();
                self.update_scroll();
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.show_line_numbers = !self.show_line_numbers;
            }
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.show_help = !self.show_help;
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.word_wrap = !self.word_wrap;
            }
            (KeyCode::Char('T'), KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
                let ts = Theme::all_themes();
                let c = ts.iter().position(|x| *x == self.theme.name).unwrap_or(0);
                self.theme = Theme::get_theme(&ts[(c + 1) % ts.len()]);
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::Search {
                    query: String::new(),
                    case_sensitive: false,
                    backward: false,
                };
            }
            (KeyCode::Char('\\'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::Replace {
                    search: String::new(),
                    replace: String::new(),
                    case_sensitive: false,
                    all: false,
                    confirmed: false,
                };
            }
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::GoToLine;
            }
            (KeyCode::Up, _) => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    let indent = self.get_indent(self.cursor_line);
                    if self.cursor_col < indent.len() && !indent.is_empty() {
                        self.cursor_col = indent.len();
                    }
                }
            }
            (KeyCode::Down, _) => {
                if self.cursor_line < self.buffer.num_lines() - 1 {
                    self.cursor_line += 1;
                    let indent = self.get_indent(self.cursor_line);
                    if self.cursor_col < indent.len() && !indent.is_empty() {
                        self.cursor_col = indent.len();
                    }
                }
            }
            (KeyCode::Left, _) => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.buffer.line_len(self.cursor_line);
                }
            }
            (KeyCode::Right, _) => {
                let line_len = self.buffer.line_len(self.cursor_line);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line < self.buffer.num_lines() - 1 {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            (KeyCode::Home, _) => {
                let indent = self.get_indent(self.cursor_line);
                if self.cursor_col > indent.len() {
                    self.cursor_col = indent.len();
                } else {
                    self.cursor_col = 0;
                }
            }
            (KeyCode::End, _) => {
                self.cursor_col = self.buffer.line_len(self.cursor_line);
            }
            (KeyCode::PageUp, _) => {
                self.cursor_line = self.cursor_line.saturating_sub(self.screen_height - 2);
            }
            (KeyCode::PageDown, _) => {
                let max_line = self.buffer.num_lines() - 1;
                self.cursor_line = (self.cursor_line + self.screen_height - 2).min(max_line);
            }
            (KeyCode::Enter, _) => {
                let indent = self.get_indent(self.cursor_line);
                self.buffer
                    .insert_newline(self.cursor_line, self.cursor_col);
                self.undo.push(EditOp::Insert {
                    pos: self.buffer.get_cursor_pos(self.cursor_line, 0),
                    text: "\n".to_string(),
                });
                self.cursor_line += 1;
                self.cursor_col = 0;
                if self.settings.auto_indent && !indent.is_empty() {
                    self.buffer
                        .insert(self.buffer.get_cursor_pos(self.cursor_line, 0), &indent);
                    self.cursor_col = indent.len();
                }
            }
            (KeyCode::Backspace, _) => {
                if self.cursor_col > 0 {
                    let pos = self
                        .buffer
                        .get_cursor_pos(self.cursor_line, self.cursor_col - 1);
                    let ch = self
                        .buffer
                        .get_line(self.cursor_line)
                        .chars()
                        .nth(self.cursor_col - 1)
                        .unwrap_or(' ');
                    self.buffer.delete(pos, 1);
                    self.undo.push(EditOp::Delete {
                        pos,
                        text: ch.to_string(),
                    });
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    let prev_line_len = self.buffer.line_len(self.cursor_line - 1);
                    self.buffer.delete(
                        self.buffer
                            .get_cursor_pos(self.cursor_line, 0)
                            .saturating_sub(1),
                        1,
                    );
                    self.cursor_line -= 1;
                    self.cursor_col = prev_line_len;
                }
            }
            (KeyCode::Tab, _) => {
                if self.settings.use_spaces {
                    let spaces = " ".repeat(self.settings.tab_size);
                    let pos = self
                        .buffer
                        .get_cursor_pos(self.cursor_line, self.cursor_col);
                    self.buffer.insert(pos, &spaces);
                    self.undo.push(EditOp::Insert {
                        pos,
                        text: spaces.clone(),
                    });
                    self.cursor_col += spaces.len();
                } else {
                    let pos = self
                        .buffer
                        .get_cursor_pos(self.cursor_line, self.cursor_col);
                    self.buffer.insert(pos, "\t");
                    self.undo.push(EditOp::Insert {
                        pos,
                        text: "\t".to_string(),
                    });
                    self.cursor_col += 1;
                }
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                if self.buffer.num_lines() > 1 {
                    let start_pos = self.buffer.get_cursor_pos(self.cursor_line, 0);
                    let line_len = self.buffer.line_len(self.cursor_line);
                    let deleted = self.buffer.get_line(self.cursor_line);
                    self.buffer.delete(start_pos, line_len + 1);
                    if self.cursor_line >= self.buffer.num_lines() - 1 {
                        self.cursor_line = self.buffer.num_lines() - 1;
                    }
                    self.cursor_col = self.cursor_col.min(self.buffer.line_len(self.cursor_line));
                    self.undo.push(EditOp::Delete {
                        pos: start_pos,
                        text: deleted,
                    });
                }
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                let start_pos = self.buffer.get_cursor_pos(self.cursor_line, 0);
                if self.cursor_col > 0 {
                    let deleted: String = self
                        .buffer
                        .get_line(self.cursor_line)
                        .chars()
                        .take(self.cursor_col)
                        .collect();
                    self.buffer.delete(start_pos, deleted.len());
                    self.undo.push(EditOp::Delete {
                        pos: start_pos,
                        text: deleted,
                    });
                    self.cursor_col = 0;
                }
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                let pos = self
                    .buffer
                    .get_cursor_pos(self.cursor_line, self.cursor_col);
                if pos < self.buffer.total_len() - 1 {
                    let ch = self.buffer.text.get_range(pos, pos + 1);
                    self.buffer.delete(pos, 1);
                    self.undo.push(EditOp::Delete { pos, text: ch });
                }
            }
            (KeyCode::Char(c), m) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if !c.is_control() {
                    let pos = self
                        .buffer
                        .get_cursor_pos(self.cursor_line, self.cursor_col);
                    self.buffer.insert(pos, &c.to_string());
                    self.undo.push(EditOp::Insert {
                        pos,
                        text: c.to_string(),
                    });
                    self.cursor_col += 1;
                }
            }
            _ => {}
        }
        self.clamp_cursor();
        self.update_scroll();
    }

    fn handle_search_owned(
        &mut self,
        k: &event::KeyEvent,
        mut query: String,
        mut case_sensitive: bool,
        mut backward: bool,
    ) -> (String, bool, bool, bool) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut should_exit = false;
        match k.code {
            KeyCode::Esc => {
                should_exit = true;
            }
            KeyCode::Enter => {
                if !query.is_empty() {
                    if let Some((line, col)) =
                        self.buffer.find(&query, self.cursor_line, self.cursor_col)
                    {
                        self.cursor_line = line;
                        self.cursor_col = col;
                        self.clamp_cursor();
                        self.update_scroll();
                    }
                }
                should_exit = true;
            }
            KeyCode::Backspace => {
                query.pop();
            }
            KeyCode::Char('c') if k.modifiers == KeyModifiers::CONTROL => {
                case_sensitive = !case_sensitive;
            }
            KeyCode::Char('r') if k.modifiers == KeyModifiers::CONTROL => {
                backward = !backward;
            }
            KeyCode::Char(c) if k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT => {
                if !c.is_control() {
                    query.push(c);
                    if !query.is_empty() {
                        if let Some((line, col)) =
                            self.buffer.find(&query, self.cursor_line, self.cursor_col)
                        {
                            self.cursor_line = line;
                            self.cursor_col = col;
                            self.clamp_cursor();
                            self.update_scroll();
                        }
                    }
                }
            }
            _ => {}
        }
        (query, case_sensitive, backward, should_exit)
    }

    fn handle_replace_owned(
        &mut self,
        k: &event::KeyEvent,
        mut search: String,
        mut replace: String,
        case_sensitive: bool,
        all: bool,
        confirmed: bool,
    ) -> (
        String,
        String,
        bool,
        bool,
        bool,
        Option<PendingAction>,
        bool,
    ) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut action = None;
        let mut should_exit = false;
        let mut new_confirmed = confirmed;

        match k.code {
            KeyCode::Esc => {
                should_exit = true;
            }
            KeyCode::Enter => {
                if confirmed {
                    if all {
                        action = Some(PendingAction::ReplaceAll(search.clone(), replace.clone()));
                    } else {
                        let _count = self.buffer.replace(&search, &replace);
                        self.undo.clear();
                    }
                    should_exit = true;
                } else {
                    new_confirmed = true;
                }
            }
            KeyCode::Tab => {
                if search.is_empty() {
                    search = self.buffer.get_line(self.cursor_line);
                } else {
                    replace = "".to_string();
                }
            }
            KeyCode::Backspace => {
                if replace.is_empty() && !search.is_empty() {
                    search.pop();
                } else {
                    replace.pop();
                }
            }
            KeyCode::Char('a') if k.modifiers == KeyModifiers::CONTROL => {
                return (
                    search,
                    replace,
                    case_sensitive,
                    true,
                    confirmed,
                    action,
                    should_exit,
                );
            }
            KeyCode::Char(c) if k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT => {
                if !c.is_control() {
                    if replace.is_empty() && !search.is_empty() && !confirmed {
                        replace.push(c);
                    } else if confirmed {
                        replace.push(c);
                    } else {
                        search.push(c);
                    }
                }
            }
            _ => {}
        }
        (
            search,
            replace,
            case_sensitive,
            all,
            new_confirmed,
            action,
            should_exit,
        )
    }

    fn handle_goto_owned(&mut self, k: &event::KeyEvent) -> (Option<usize>, bool) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        match k.code {
            KeyCode::Esc => (None, true),
            KeyCode::Enter => (None, true),
            _ => (None, false),
        }
    }

    fn handle_confirm_owned(
        &mut self,
        k: &event::KeyEvent,
        title: String,
        message: String,
        options: Vec<String>,
        mut selected: usize,
    ) -> (String, String, Vec<String>, usize, Option<PendingAction>) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut action = None;
        match k.code {
            KeyCode::Up => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            KeyCode::Down => {
                if selected < options.len() - 1 {
                    selected += 1;
                }
            }
            KeyCode::Enter => match options[selected].as_str() {
                "Yes" => {
                    if self.buffer.path.is_some() {
                        action = Some(PendingAction::SaveAndQuit);
                    } else {
                        self.quit_after_save = true;
                        return (title, message, options, selected, action);
                    }
                }
                "No" => {
                    action = Some(PendingAction::QuitWithoutSave);
                }
                _ => {}
            },
            KeyCode::Esc => {}
            _ => {}
        }
        (title, message, options, selected, action)
    }

    fn handle_input_owned(
        &mut self,
        k: &event::KeyEvent,
        title: String,
        mut input: String,
        mut history: Vec<String>,
    ) -> (String, String, Vec<String>, Option<PendingAction>) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut action = None;
        match k.code {
            KeyCode::Enter => {
                action = Some(PendingAction::SaveAs(input.clone()));
                if !input.is_empty() {
                    history.push(input.clone());
                }
            }
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) if !c.is_control() => {
                input.push(c);
            }
            KeyCode::Tab => {
                input.push('\t');
            }
            _ => {}
        }
        (title, input, history, action)
    }

    fn open_file(&mut self) {
        if let Ok(ent) = std::fs::read_dir(".") {
            for e in ent
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .take(10)
            {
                if let Some(ext) = e.path().extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    let known_exts = [
                        "txt", "rs", "js", "ts", "py", "go", "md", "json", "toml", "yaml", "c",
                        "h", "cpp", "hpp", "sh", "bash", "zsh", "html", "css", "xml",
                    ];
                    if known_exts.contains(&ext_str.as_str()) {
                        if let Some(b) = Buffer::from_file(e.path()) {
                            self.buffer = b;
                            self.cursor_line = 0;
                            self.cursor_col = 0;
                            self.scroll_offset = 0;
                            self.undo.clear();
                            break;
                        }
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let a = f.area();
        let th = 1u16;
        let hh = if self.show_help { 1u16 } else { 0u16 };
        let sh = 1u16;
        let eh = a.height.saturating_sub(th + hh + sh);

        let ta = Rect::new(a.x, a.y, a.width, th);
        let modified_indicator = if self.buffer.is_modified {
            " [Modified]"
        } else {
            ""
        };
        f.render_widget(
            TitleBar {
                file_name: format!(" Nova - {}{} ", self.buffer.file_name(), modified_indicator),
                theme: self.theme.clone(),
            },
            ta,
        );

        let sa = Rect::new(a.x, a.y + th + eh, a.width, sh);
        let status_text = match &self.mode {
            EditorMode::Search { query, .. } => format!("Search: {}", query),
            EditorMode::Replace {
                search,
                replace,
                confirmed,
                ..
            } => {
                if *confirmed {
                    format!(
                        "Replace '{}' with '{}'? [Enter=Yes, A=all, C=cancel]",
                        search, replace
                    )
                } else {
                    format!("Replace: {} -> {}", search, replace)
                }
            }
            EditorMode::GoToLine => "Go to line:".to_string(),
            EditorMode::Confirm { title, message, .. } => format!("{} - {}", title, message),
            EditorMode::Input { title, input, .. } => format!("{}: {}", title, input),
            _ => format!("Ln {}, Col {}", self.cursor_line + 1, self.cursor_col + 1),
        };
        f.render_widget(
            StatusBar {
                file_name: self.buffer.file_name(),
                modified: self.buffer.is_modified,
                line: self.cursor_line + 1,
                col: self.cursor_col + 1,
                language: self.buffer.language.clone(),
                theme: self.theme.clone(),
                search_mode: !matches!(self.mode, EditorMode::Normal),
                search_text: status_text,
            },
            sa,
        );

        if self.show_help {
            let ha = Rect::new(a.x, a.y + th + eh + sh, a.width, hh);
            f.render_widget(
                HelpBar {
                    shortcuts: vec![
                        ("Ctrl+H", "Help"),
                        ("Ctrl+O", "Open"),
                        ("Ctrl+S", "Save"),
                        ("Ctrl+F", "Find"),
                    ],
                    visible: true,
                    theme: self.theme.clone(),
                    tip: self.current_tip.clone(),
                },
                ha,
            );
        }

        if self.mode == EditorMode::Help {
            self.render_help(f, a);
            return;
        }

        let ea = Rect::new(a.x, a.y + th, a.width, eh);
        f.render_widget(
            EditorView {
                buffer: self.buffer.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
                show_line_numbers: self.show_line_numbers,
                scroll_offset: self.scroll_offset,
                theme: self.theme.clone(),
                cursor_blink_on: self.cursor_blink_on,
                word_wrap: self.word_wrap,
                width: self.screen_width as u16,
            },
            ea,
        );

        if let EditorMode::Input { title, input, .. } = &self.mode {
            self.render_input_dialog(f, a, title, input);
        } else if let EditorMode::GoToLine = &self.mode {
            self.render_input_dialog(f, a, "Go to Line", "");
        }
    }

    fn render_help(&self, f: &mut ratatui::Frame, area: Rect) {
        let dw = 60u16;
        let dh = 20u16;
        let dx = (area.width.saturating_sub(dw)) / 2;
        let dy = (area.height.saturating_sub(dh)) / 2;
        let dr = Rect::new(area.x + dx, area.y + dy, dw, dh);

        let bp = ratatui::widgets::Block::default()
            .title(" Help - Press Ctrl+H or ESC to close ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Double)
            .style(
                Style::default()
                    .bg(self.theme.background)
                    .fg(self.theme.foreground),
            );
        f.render_widget(bp, dr);

        let content = "Key          Action              Key          Action\n\
             ------------------------------------------------\n\
             Ctrl+O       Open file           Ctrl+Z       Undo\n\
             Ctrl+S       Save file           Ctrl+Y       Redo\n\
             Ctrl+F       Find text           Ctrl+T       Change theme\n\
             Ctrl+G       Go to line          Ctrl+B       Toggle lines\n\
             Ctrl+\\       Replace             Ctrl+W       Toggle wrap\n\
             Ctrl+Q       Quit                Ctrl+H       Help";

        let tr = dr.inner(Margin::new(1, 1));
        f.render_widget(
            Paragraph::new(content)
                .style(
                    Style::default()
                        .bg(self.theme.background)
                        .fg(self.theme.foreground),
                )
                .wrap(ratatui::widgets::Wrap { trim: true }),
            tr,
        );
    }

    fn render_input_dialog(&self, f: &mut ratatui::Frame, area: Rect, title: &str, input: &str) {
        let dw = 30u16;
        let dh = 3u16;
        let dx = (area.width.saturating_sub(dw)) / 2;
        let dy = (area.height.saturating_sub(dh)) / 2;
        let dr = Rect::new(area.x + dx, area.y + dy, dw, dh);

        let bp = ratatui::widgets::Block::default()
            .title(format!(" {} ", title))
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Double)
            .style(
                Style::default()
                    .bg(self.theme.background)
                    .fg(self.theme.foreground),
            );
        f.render_widget(bp, dr);

        let tr = dr.inner(Margin::new(1, 1));
        f.render_widget(
            Paragraph::new(input.to_string()).style(
                Style::default()
                    .bg(self.theme.background)
                    .fg(self.theme.foreground),
            ),
            tr,
        );
    }
}

fn run(initial_file: Option<String>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut o = stdout();
    o.execute(EnterAlternateScreen)?;
    let b = CrosstermBackend::new(o);
    let mut t = Terminal::new(b)?;

    let (width, height) = size().unwrap_or((80, 24));

    let mut e = Editor::new(initial_file, width as usize, height as usize);

    loop {
        t.draw(|f| e.render(f))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read() {
                Ok(Event::Key(k)) => {
                    if k.kind == KeyEventKind::Press {
                        e.handle_key(&k);
                    }
                }
                Ok(Event::Resize(w, h)) => {
                    e.screen_width = w as usize;
                    e.screen_height = h as usize;
                }
                _ => {}
            }
        }

        e.update_cursor_blink();
        if e.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    t.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut initial_file: Option<String> = None;
    for arg in &args[1..] {
        if !arg.starts_with('-') {
            initial_file = Some(arg.clone());
            break;
        }
    }

    if let Err(x) = run(initial_file) {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen).ok();
        eprintln!("Error: {}", x);
        std::process::exit(1);
    }
    Ok(())
}
