use std::path::PathBuf;

#[derive(Clone)]
pub struct GapBuffer {
    before: Vec<u8>,
    after: Vec<u8>,
    line_offsets: Vec<usize>,
}

impl GapBuffer {
    pub fn new() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            line_offsets: vec![0],
        }
    }

    fn build_cache(&mut self) {
        self.line_offsets.clear();
        self.line_offsets.push(0);

        for (i, &byte) in self.before.iter().enumerate() {
            if byte == b'\n' {
                self.line_offsets.push(i + 1);
            }
        }

        let after_start = self.before.len();
        for (i, &byte) in self.after.iter().enumerate() {
            if byte == b'\n' {
                self.line_offsets.push(after_start + i + 1);
            }
        }

        self.line_offsets.push(self.before.len() + self.after.len());
    }

    #[allow(dead_code)]
    pub fn from_string(s: &str) -> Self {
        let mut buf = Self::new();
        buf.before = s.as_bytes().to_vec();
        buf.build_cache();
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
        self.build_cache();
    }

    pub fn delete(&mut self, pos: usize, len: usize) {
        self.move_gap(pos);
        let del_len = len.min(self.after.len());
        self.after.drain(..del_len);
        self.build_cache();
    }

    pub fn get_line(&self, line_num: usize) -> String {
        if self.line_offsets.len() <= 1 {
            return String::new();
        }

        if line_num >= self.line_offsets.len() - 1 {
            return String::new();
        }

        let start = self.line_offsets[line_num];
        let mut end = self.line_offsets[line_num + 1];

        if end == 0 {
            return String::new();
        }

        if start < self.before.len() {
            let before_end = self.before.len();
            if end <= before_end {
                let actual_end = if end > 0 && self.before[end.saturating_sub(1)] == b'\n' {
                    end - 1
                } else {
                    end
                };
                if actual_end > start {
                    return String::from_utf8_lossy(&self.before[start..actual_end]).to_string();
                }
                return String::new();
            } else {
                let before_part_end = before_end.min(end);
                let actual_before_end = if before_part_end > start
                    && before_part_end <= self.before.len()
                    && self.before[before_part_end.saturating_sub(1)] == b'\n'
                {
                    before_part_end - 1
                } else {
                    before_part_end
                };
                let before_part = &self.before[start..actual_before_end];
                let after_end = (end - before_end).min(self.after.len());
                let actual_after_end =
                    if after_end > 0 && self.after[after_end.saturating_sub(1)] == b'\n' {
                        after_end - 1
                    } else {
                        after_end
                    };
                let after_part = &self.after[..actual_after_end];
                let mut result = Vec::with_capacity(before_part.len() + after_part.len());
                result.extend_from_slice(before_part);
                result.extend_from_slice(after_part);
                return String::from_utf8_lossy(&result).to_string();
            }
        } else {
            let after_start = start - self.before.len();
            let after_end = (end - self.before.len()).min(self.after.len());
            if after_start < self.after.len() {
                let actual_end = if after_end > after_start
                    && self.after[after_end.saturating_sub(1)] == b'\n'
                {
                    after_end - 1
                } else {
                    after_end
                };
                if actual_end > after_start {
                    return String::from_utf8_lossy(&self.after[after_start..actual_end])
                        .to_string();
                }
            }
        }

        String::new()
    }

    pub fn num_lines(&self) -> usize {
        self.line_offsets.len().max(1) - 1
    }

    pub fn get_line_offsets(&self) -> Vec<usize> {
        self.line_offsets.clone()
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
        let offsets = text.get_line_offsets();
        let buf = Self {
            text,
            path: None,
            is_modified: false,
            language: "plaintext".to_string(),
            line_offsets: offsets,
        };
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
        let offsets = text.get_line_offsets();
        let buf = Self {
            text,
            path: Some(path),
            is_modified: false,
            language: "plaintext".to_string(),
            line_offsets: offsets,
        };
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
            line_offsets: Vec::new(),
        };
        buf.line_offsets = buf.text.get_line_offsets();
        buf
    }

    pub fn insert(&mut self, pos: usize, text: &str) {
        self.text.insert(pos, text);
        self.line_offsets = self.text.get_line_offsets();
        self.is_modified = true;
    }

    pub fn delete(&mut self, pos: usize, len: usize) {
        self.text.delete(pos, len);
        self.line_offsets = self.text.get_line_offsets();
        self.is_modified = true;
    }

    pub fn get_line(&self, line: usize) -> String {
        self.text.get_line(line)
    }

    pub fn num_lines(&self) -> usize {
        self.text.num_lines()
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.line_offsets.len() {
            return 0;
        }
        let start = self.line_offsets[line];
        let end = if line + 1 < self.line_offsets.len() {
            self.line_offsets[line + 1]
        } else {
            self.text.len()
        };
        end.saturating_sub(start)
    }

    pub fn total_len(&self) -> usize {
        self.text.len()
    }

    pub fn insert_newline(&mut self, line: usize, col: usize) {
        let pos = self.get_cursor_pos(line, col);
        self.text.insert(pos, "\n");
        self.line_offsets = self.text.get_line_offsets();
        self.is_modified = true;
    }

    pub fn get_cursor_pos(&self, line: usize, col: usize) -> usize {
        if line >= self.line_offsets.len() {
            return self.text.len();
        }
        let offset = self.line_offsets[line];
        let line_len = if line + 1 < self.line_offsets.len() {
            self.line_offsets[line + 1].saturating_sub(offset)
        } else {
            0
        };
        let col = col.min(line_len);
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
        self.line_offsets = self.text.get_line_offsets();
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
