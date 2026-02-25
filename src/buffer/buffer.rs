use std::path::PathBuf;

#[derive(Clone)]
pub struct GapBuffer {
    before: Vec<u8>,
    after: Vec<u8>,
}

impl GapBuffer {
    pub fn new() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_string(s: &str) -> Self {
        let mut buf = Self::new();
        buf.before = s.as_bytes().to_vec();
        buf
    }

    pub fn len(&self) -> usize {
        self.before.len() + self.after.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_range(&self, start: usize, end: usize) -> String {
        let total = self.len();
        let start = start.min(total);
        let end = end.min(total);

        if start >= end {
            return String::new();
        }

        let before_end = self.before.len();

        if start < before_end && end <= before_end {
            String::from_utf8_lossy(&self.before[start..end]).to_string()
        } else if start >= before_end {
            let after_start = start - before_end;
            let after_end = (end - before_end).min(self.after.len());
            String::from_utf8_lossy(&self.after[after_start..after_end]).to_string()
        } else {
            let mut result = String::from_utf8_lossy(&self.before[start..]).to_string();
            let after_end = (end - before_end).min(self.after.len());
            result.push_str(&String::from_utf8_lossy(&self.after[..after_end]));
            result
        }
    }

    pub fn move_gap(&mut self, pos: usize) {
        let pos = pos.min(self.len());
        let gap_pos = self.before.len();

        if pos < gap_pos {
            let mut tmp = self.before[pos..].to_vec();
            self.after.append(&mut tmp);
            self.before.truncate(pos);
        } else if pos > gap_pos {
            let mut tmp = self.after[..pos - gap_pos].to_vec();
            tmp.append(&mut self.before);
            self.before = tmp;
            self.after.clear();
        }
    }

    pub fn insert(&mut self, pos: usize, text: &str) {
        self.move_gap(pos);
        self.before.extend_from_slice(text.as_bytes());
    }

    pub fn delete(&mut self, pos: usize, len: usize) {
        self.move_gap(pos);
        let del_len = len.min(self.after.len());
        self.after.drain(..del_len);
    }

    pub fn get_line(&self, line_num: usize) -> String {
        let mut line_start = 0;
        let mut current_line = 0;

        for (i, &byte) in self.before.iter().enumerate() {
            if byte == b'\n' {
                if current_line == line_num {
                    return String::from_utf8_lossy(&self.before[line_start..i]).to_string();
                }
                current_line += 1;
                line_start = i + 1;
            }
        }

        let before_end = self.before.len();
        let mut after_start = if self.before.last() == Some(&b'\n') {
            0
        } else {
            before_end
        };

        for (i, &byte) in self.after.iter().enumerate() {
            if byte == b'\n' {
                if current_line == line_num && after_start <= i {
                    return String::from_utf8_lossy(&self.after[after_start..i]).to_string();
                }
                current_line += 1;
                after_start = i + 1;
            }
        }

        if current_line == line_num {
            if after_start < self.after.len() {
                String::from_utf8_lossy(&self.after[after_start..]).to_string()
            } else if line_start < before_end {
                String::from_utf8_lossy(&self.before[line_start..]).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    pub fn num_lines(&self) -> usize {
        let mut count = 1;
        for &byte in &self.before {
            if byte == b'\n' {
                count += 1;
            }
        }
        for &byte in &self.after {
            if byte == b'\n' {
                count += 1;
            }
        }
        count
    }

    #[allow(dead_code)]
    pub fn line_start_offset(&self, line_num: usize) -> usize {
        let mut offset = 0;
        let mut current = 0;

        for (i, &byte) in self.before.iter().enumerate() {
            if current == line_num {
                return offset;
            }
            if byte == b'\n' {
                current += 1;
                offset = i + 1;
            }
        }

        let before_len = self.before.len();
        if current == line_num {
            return offset;
        }

        for (i, &byte) in self.after.iter().enumerate() {
            if current == line_num {
                return before_len + i;
            }
            if byte == b'\n' {
                current += 1;
                let _offset = before_len + i + 1;
            }
        }

        self.len()
    }

    pub fn to_string(&self) -> String {
        let mut result = String::from_utf8_lossy(&self.before).to_string();
        result.push_str(&String::from_utf8_lossy(&self.after));
        result
    }

    #[allow(dead_code)]
    pub fn to_lines(&self) -> Vec<String> {
        self.to_string().lines().map(|s| s.to_string()).collect()
    }
}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Buffer {
    pub text: GapBuffer,
    pub path: Option<PathBuf>,
    pub is_modified: bool,
    pub language: String,
    pub line_offsets: Vec<usize>,
}

impl Buffer {
    pub fn new() -> Self {
        let mut text = GapBuffer::new();
        text.insert(0, "\n");
        let mut buf = Self {
            text,
            path: None,
            is_modified: false,
            language: "plaintext".to_string(),
            line_offsets: vec![0],
        };
        buf.update_line_offsets();
        buf
    }

    pub fn from_file(path: PathBuf) -> Option<Self> {
        let content = std::fs::read_to_string(&path).ok()?;
        let content = if content.ends_with('\n') {
            content
        } else {
            format!("{}\n", content)
        };

        let text = GapBuffer::from_string(&content);
        let mut buf = Self {
            text,
            path: Some(path.clone()),
            is_modified: false,
            language: detect_language(&path),
            line_offsets: vec![0],
        };
        buf.update_line_offsets();
        Some(buf)
    }

    pub fn for_new_file(path: PathBuf) -> Self {
        let mut text = GapBuffer::new();
        text.insert(0, "\n");
        let mut buf = Self {
            text,
            path: Some(path),
            is_modified: false,
            language: "plaintext".to_string(),
            line_offsets: vec![0],
        };
        buf.update_line_offsets();
        buf
    }

    fn update_line_offsets(&mut self) {
        self.line_offsets.clear();
        self.line_offsets.push(0);

        let bytes = self.text.to_string();
        for (i, byte) in bytes.bytes().enumerate() {
            if byte == b'\n' {
                self.line_offsets.push(i + 1);
            }
        }
    }

    pub fn insert(&mut self, pos: usize, text: &str) {
        self.text.insert(pos, text);
        self.update_line_offsets();
        self.is_modified = true;
    }

    pub fn delete(&mut self, pos: usize, len: usize) {
        self.text.delete(pos, len);
        self.update_line_offsets();
        self.is_modified = true;
    }

    pub fn get_line(&self, line: usize) -> String {
        self.text.get_line(line)
    }

    pub fn num_lines(&self) -> usize {
        self.text.num_lines()
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.get_line(line).len()
    }

    pub fn total_len(&self) -> usize {
        self.text.len()
    }

    pub fn insert_newline(&mut self, line: usize, col: usize) {
        let pos = self.get_cursor_pos(line, col);
        self.text.insert(pos, "\n");
        self.update_line_offsets();
        self.is_modified = true;
    }

    pub fn get_cursor_pos(&self, line: usize, col: usize) -> usize {
        if line >= self.line_offsets.len() {
            return self.text.len();
        }
        let offset = self.line_offsets[line];
        let line_content = self.get_line(line);
        let col = col.min(line_content.len());
        offset + col
    }

    pub fn get_line_col(&self, pos: usize) -> (usize, usize) {
        let text_len = self.text.len();
        let pos = pos.min(text_len);

        let mut line = 0;
        for (i, &offset) in self.line_offsets.iter().enumerate() {
            if i + 1 < self.line_offsets.len() {
                if pos < self.line_offsets[i + 1] {
                    return (line, pos - offset);
                }
            }
            line += 1;
        }

        if let Some(&last_offset) = self.line_offsets.last() {
            (line.saturating_sub(1), pos.saturating_sub(last_offset))
        } else {
            (0, pos)
        }
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            let content = self.text.to_string();
            let content = content.trim_end_matches('\n');
            std::fs::write(path, content)?;
            self.is_modified = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        let content = self.text.to_string();
        let content = content.trim_end_matches('\n');
        std::fs::write(&path, content)?;
        self.path = Some(path);
        self.language = detect_language(&self.path.as_ref().unwrap());
        self.is_modified = false;
        Ok(())
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
            .to_string()
    }

    pub fn find(&self, query: &str, from_line: usize, from_col: usize) -> Option<(usize, usize)> {
        if query.is_empty() {
            return None;
        }

        let text = self.text.to_string();
        let mut search_start = 0;

        for (i, &offset) in self.line_offsets.iter().enumerate() {
            if i == from_line {
                search_start = offset + from_col.min(self.get_line(i).len());
                break;
            }
        }

        if let Some(pos) = text[search_start..].find(query) {
            return Some(self.get_line_col(search_start + pos));
        }

        if search_start > 0 {
            if let Some(pos) = text[..search_start].find(query) {
                return Some(self.get_line_col(pos));
            }
        }

        None
    }

    pub fn replace(&mut self, old: &str, new: &str) -> usize {
        let text = self.text.to_string();
        let count = text.matches(old).count();
        let new_text = text.replace(old, new);

        self.text = GapBuffer::from_string(&new_text);
        if !new_text.ends_with('\n') {
            self.text.insert(self.text.len(), "\n");
        }
        self.update_line_offsets();
        self.is_modified = true;
        count
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

fn detect_language(path: &PathBuf) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => "rust",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "py" | "pyw" => "python",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "sh" | "bash" | "zsh" | "fish" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "md" | "markdown" => "markdown",
        "sql" => "sql",
        "lua" => "lua",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        _ => "plaintext",
    }
    .to_string()
}
