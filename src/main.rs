use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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

#[derive(Clone)]
enum EditOp {
    Insert {
        line: usize,
        col: usize,
        text: String,
    },
    Delete {
        line: usize,
        col: usize,
        text: String,
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
            EditOp::Insert { line, col, text } => {
                if *line < buffer.lines.len() {
                    let ls = &mut buffer.lines[*line];
                    if *col <= ls.len() {
                        ls.drain(*col..(*col + text.len()).min(ls.len()));
                    }
                }
                buffer.is_modified = true;
                true
            }
            EditOp::Delete { line, col, text } => {
                if *line < buffer.lines.len() {
                    buffer.lines[*line].insert_str(*col, text);
                }
                buffer.is_modified = true;
                true
            }
        }
    }
    fn redo(&mut self, buffer: &mut Buffer) -> bool {
        if self.pos >= self.ops.len() {
            return false;
        }
        match &self.ops[self.pos] {
            EditOp::Insert { line, col, text } => {
                if *line < buffer.lines.len() {
                    buffer.lines[*line].insert_str(*col, text);
                }
                buffer.is_modified = true;
                self.pos += 1;
                true
            }
            EditOp::Delete { line, col, text } => {
                if *line < buffer.lines.len() {
                    let ls = &mut buffer.lines[*line];
                    if *col < ls.len() {
                        ls.drain(*col..(*col + text.len()).min(ls.len()));
                    }
                }
                buffer.is_modified = true;
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

enum EditorMode {
    Normal,
    Search {
        query: String,
        results: Vec<(usize, usize)>,
        result_idx: usize,
    },
    Confirm {
        title: String,
        message: String,
        options: Vec<String>,
        selected: usize,
    },
    Input {
        title: String,
        input: String,
    },
}

#[derive(Clone)]
enum PendingAction {
    SaveAndQuit,
    QuitWithoutSave,
    SaveAs(String),
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
    should_quit: bool,
    undo: UndoHistory,
    mode: EditorMode,
    pending_action: Option<PendingAction>,
    quit_after_save: bool,
    cursor_blink_on: bool,
    last_cursor_time: std::time::Instant,
}

impl Editor {
    fn new(initial_file: Option<String>) -> Self {
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
            should_quit: false,
            undo: UndoHistory::new(),
            mode: EditorMode::Normal,
            pending_action: None,
            quit_after_save: false,
            cursor_blink_on: true,
            last_cursor_time: std::time::Instant::now(),
        }
    }

    fn update_scroll(&mut self) {
        let v = 20usize;
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
        if self.cursor_line >= self.scroll_offset + v {
            self.scroll_offset = self.cursor_line.saturating_sub(v - 1);
        }
        if self.scroll_offset + v > self.buffer.lines.len() {
            self.scroll_offset = self.buffer.lines.len().saturating_sub(v);
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
        self.cursor_line = self
            .cursor_line
            .min(self.buffer.lines.len().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.buffer.line_len(self.cursor_line));
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
                results,
                result_idx,
            } => {
                let (new_query, new_results, new_result_idx, should_exit) =
                    self.handle_search_owned(key, query, results, result_idx);
                if should_exit {
                    self.mode = EditorMode::Normal;
                } else {
                    self.mode = EditorMode::Search {
                        query: new_query,
                        results: new_results,
                        result_idx: new_result_idx,
                    };
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
                // If action is None and Enter was pressed on "Yes", we need Input mode for save-as
                if action.is_none() && key.code == KeyCode::Enter && is_yes_selected {
                    self.quit_after_save = true;
                    self.mode = EditorMode::Input {
                        title: "Save As".into(),
                        input: "untitled.txt".into(),
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
            EditorMode::Input { title, input } => {
                let (new_title, new_input, action) = self.handle_input_owned(key, title, input);
                if let Some(act) = action {
                    self.pending_action = Some(act);
                }
                if key.code != KeyCode::Enter && key.code != KeyCode::Esc {
                    self.mode = EditorMode::Input {
                        title: new_title,
                        input: new_input,
                    };
                } else {
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
            }
        }
    }

    fn handle_normal(&mut self, k: &event::KeyEvent) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        match (k.code, k.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                if self.buffer.path.is_none() {
                    self.quit_after_save = true;
                    self.mode = EditorMode::Input {
                        title: "Save As".into(),
                        input: "untitled.txt".into(),
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
                    };
                } else {
                    let _ = self.buffer.save();
                }
            }
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                self.open_file();
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                let _ = self.undo.undo(&mut self.buffer);
                self.clamp_cursor();
                self.update_scroll();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                let _ = self.undo.redo(&mut self.buffer);
                self.clamp_cursor();
                self.update_scroll();
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.show_line_numbers = !self.show_line_numbers;
            }
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.show_help = !self.show_help;
            }
            (KeyCode::Char('T'), KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
                let ts = Theme::all_themes();
                let c = ts.iter().position(|x| *x == self.theme.name).unwrap_or(0);
                self.theme = Theme::get_theme(&ts[(c + 1) % ts.len()]);
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::Search {
                    query: String::new(),
                    results: Vec::new(),
                    result_idx: 0,
                };
            }
            (KeyCode::Up, _) => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                }
            }
            (KeyCode::Down, _) => {
                if self.cursor_line < self.buffer.lines.len() - 1 {
                    self.cursor_line += 1;
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
                } else if self.cursor_line < self.buffer.lines.len() - 1 {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            (KeyCode::Home, _) => {
                self.cursor_col = 0;
            }
            (KeyCode::End, _) => {
                self.cursor_col = self.buffer.line_len(self.cursor_line);
            }
            (KeyCode::PageUp, _) => {
                self.cursor_line = self.cursor_line.saturating_sub(10);
            }
            (KeyCode::PageDown, _) => {
                self.cursor_line =
                    (self.cursor_line + 10).min(self.buffer.lines.len().saturating_sub(1));
            }
            (KeyCode::Enter, _) => {
                self.buffer
                    .insert_newline(self.cursor_line, self.cursor_col);
                self.undo.push(EditOp::Insert {
                    line: self.cursor_line,
                    col: self.cursor_col,
                    text: "\n".into(),
                });
                self.cursor_line += 1;
                self.cursor_col = 0;
            }
            (KeyCode::Backspace, _) => {
                if self.cursor_col > 0 {
                    let d = self.buffer.lines[self.cursor_line].remove(self.cursor_col - 1);
                    self.undo.push(EditOp::Delete {
                        line: self.cursor_line,
                        col: self.cursor_col - 1,
                        text: d.to_string(),
                    });
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    let c = self.buffer.lines.remove(self.cursor_line);
                    self.cursor_line -= 1;
                    self.cursor_col = self.buffer.line_len(self.cursor_line);
                    self.buffer.lines[self.cursor_line].push_str(&c);
                    self.undo.push(EditOp::Delete {
                        line: self.cursor_line,
                        col: self.cursor_col,
                        text: "\n".into(),
                    });
                }
            }
            (KeyCode::Tab, _) => {
                let spaces = " ".repeat(self.settings.tab_size);
                if let Some(line) = self.buffer.lines.get_mut(self.cursor_line) {
                    line.insert_str(self.cursor_col, &spaces);
                }
                self.undo.push(EditOp::Insert {
                    line: self.cursor_line,
                    col: self.cursor_col,
                    text: spaces.clone(),
                });
                self.cursor_col += spaces.len();
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                if self.buffer.lines.len() > 1 {
                    let deleted = self.buffer.lines.remove(self.cursor_line);
                    if self.cursor_line >= self.buffer.lines.len() {
                        self.cursor_line = self.buffer.lines.len() - 1;
                    }
                    self.cursor_col = self.cursor_col.min(self.buffer.line_len(self.cursor_line));
                    self.undo.push(EditOp::Delete {
                        line: self.cursor_line,
                        col: 0,
                        text: deleted,
                    });
                }
            }
            (KeyCode::Char(c), m) if m.is_empty() || m == KeyModifiers::SHIFT => {
                if !c.is_control() {
                    if let Some(line) = self.buffer.lines.get_mut(self.cursor_line) {
                        line.insert(self.cursor_col, c);
                    }
                    self.undo.push(EditOp::Insert {
                        line: self.cursor_line,
                        col: self.cursor_col,
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
        mut results: Vec<(usize, usize)>,
        mut result_idx: usize,
    ) -> (String, Vec<(usize, usize)>, usize, bool) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut should_exit = false;
        match k.code {
            KeyCode::Esc => {
                should_exit = true;
            }
            KeyCode::Enter => {
                if !results.is_empty() {
                    let (ln, co) = results[result_idx];
                    self.cursor_line = ln;
                    self.cursor_col = co;
                    self.clamp_cursor();
                    self.update_scroll();
                }
                should_exit = true;
            }
            KeyCode::Backspace => {
                query.pop();
                results.clear();
                result_idx = 0;
                if !query.is_empty() {
                    for (ln, l) in self.buffer.lines.iter().enumerate() {
                        let mut s = 0;
                        while let Some(p) = l[s..].find(&query) {
                            results.push((ln, s + p));
                            s += p + 1;
                        }
                    }
                }
            }
            KeyCode::Char(c) if k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT => {
                if !c.is_control() {
                    query.push(c);
                    results.clear();
                    result_idx = 0;
                    if !query.is_empty() {
                        for (ln, l) in self.buffer.lines.iter().enumerate() {
                            let mut s = 0;
                            while let Some(p) = l[s..].find(&query) {
                                results.push((ln, s + p));
                                s += p + 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        (query, results, result_idx, should_exit)
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
            KeyCode::Enter => {
                match options[selected].as_str() {
                    "Yes" => {
                        if self.buffer.path.is_some() {
                            action = Some(PendingAction::SaveAndQuit);
                        } else {
                            self.quit_after_save = true;
                            // Return Input mode instead of Confirm
                            return (title, message, options, selected, action);
                        }
                    }
                    "No" => {
                        action = Some(PendingAction::QuitWithoutSave);
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => { /* will return to Normal */ }
            _ => {}
        }
        (title, message, options, selected, action)
    }

    fn handle_input_owned(
        &mut self,
        k: &event::KeyEvent,
        title: String,
        mut input: String,
    ) -> (String, String, Option<PendingAction>) {
        self.cursor_blink_on = true;
        self.last_cursor_time = std::time::Instant::now();

        let mut action = None;
        match k.code {
            KeyCode::Enter => {
                action = Some(PendingAction::SaveAs(input.clone()));
            }
            KeyCode::Esc => { /* will return to Normal */ }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) if !c.is_control() => {
                input.push(c);
            }
            _ => {}
        }
        (title, input, action)
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
                    if [
                        "txt", "rs", "js", "ts", "py", "go", "md", "json", "toml", "yaml", "c",
                        "h", "cpp",
                    ]
                    .contains(&ext_str.as_str())
                    {
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
        f.render_widget(
            TitleBar {
                file_name: format!(" Nova - {} ", self.buffer.file_name()),
                theme: self.theme.clone(),
            },
            ta,
        );

        let sa = Rect::new(a.x, a.y + th + eh, a.width, sh);
        let (search_mode, status_text) = match &self.mode {
            EditorMode::Search { query, .. } => (true, query.clone()),
            EditorMode::Confirm { title, message, .. } => {
                (true, format!("{} - {}", title, message))
            }
            EditorMode::Input { title, input, .. } => (true, format!("{}: {}", title, input)),
            _ => (false, "".into()),
        };
        f.render_widget(
            StatusBar {
                file_name: self.buffer.file_name(),
                modified: self.buffer.is_modified,
                line: self.cursor_line + 1,
                col: self.cursor_col + 1,
                language: self.buffer.language.clone(),
                theme: self.theme.clone(),
                search_mode,
                search_text: status_text,
            },
            sa,
        );

        if self.show_help {
            let ha = Rect::new(a.x, a.y + th + eh + sh, a.width, hh);
            f.render_widget(
                HelpBar {
                    shortcuts: vec![
                        ("Ctrl+O", "Open"),
                        ("Ctrl+S", "Save"),
                        ("Ctrl+F", "Find"),
                        ("Ctrl+Z", "Undo"),
                        ("Ctrl+Y", "Redo"),
                        ("Ctrl+T", "Theme"),
                        ("Ctrl+B", "Lines"),
                        ("Ctrl+Q", "Quit"),
                    ],
                    visible: true,
                    theme: self.theme.clone(),
                },
                ha,
            );
        }

        let ea = Rect::new(a.x, a.y + th, a.width, eh);
        f.render_widget(
            EditorView {
                lines: self.buffer.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
                show_line_numbers: self.show_line_numbers,
                scroll_offset: self.scroll_offset,
                theme: self.theme.clone(),
                cursor_blink_on: self.cursor_blink_on,
            },
            ea,
        );

        if let EditorMode::Input { title, input } = &self.mode {
            let dw = 40u16;
            let dh = 5u16;
            let dx = (a.width.saturating_sub(dw)) / 2;
            let dy = (a.height.saturating_sub(dh)) / 2;
            let dr = Rect::new(a.x + dx, a.y + dy, dw, dh);
            let bp = ratatui::widgets::Block::default()
                .title(title.clone())
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Double)
                .style(
                    Style::default()
                        .bg(self.theme.background)
                        .fg(self.theme.foreground),
                );
            f.render_widget(bp, dr);
            let tr = dr.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            f.render_widget(
                Paragraph::new(input.clone()).style(
                    Style::default()
                        .bg(self.theme.background)
                        .fg(self.theme.foreground),
                ),
                tr,
            );
        }
    }
}

fn run(initial_file: Option<String>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut o = stdout();
    o.execute(EnterAlternateScreen)?;
    let b = CrosstermBackend::new(o);
    let mut t = Terminal::new(b)?;
    let mut e = Editor::new(initial_file);
    loop {
        t.draw(|f| e.render(f))?;
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    e.handle_key(&k);
                }
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
        eprintln!("{}", x);
        std::process::exit(1);
    }
    Ok(())
}
