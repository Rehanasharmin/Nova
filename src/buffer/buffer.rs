use std::path::PathBuf;

#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub path: Option<PathBuf>,
    pub is_modified: bool,
    pub language: String,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            path: None,
            is_modified: false,
            language: "plaintext".to_string(),
        }
    }

    pub fn from_file(path: PathBuf) -> Option<Self> {
        let content = std::fs::read_to_string(&path).ok()?;
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        let language = detect_language(&path);
        Some(Self {
            lines,
            path: Some(path),
            is_modified: false,
            language,
        })
    }

    pub fn for_new_file(path: PathBuf) -> Self {
        let language = detect_language(&path);
        Self {
            lines: vec![String::new()],
            path: Some(path),
            is_modified: false,
            language,
        }
    }

    pub fn insert_newline(&mut self, line: usize, col: usize) {
        if line < self.lines.len() {
            let old_line = self.lines[line].clone();
            self.lines[line] = old_line[..col.min(old_line.len())].to_string();
            let new_line = if col < old_line.len() {
                old_line[col..].to_string()
            } else {
                String::new()
            };
            self.lines.insert(line + 1, new_line);
            self.is_modified = true;
        }
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| l.len()).unwrap_or(0)
    }

    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn get_line(&self, line: usize) -> String {
        self.lines.get(line).cloned().unwrap_or_default()
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
            self.is_modified = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        let content = self.lines.join("\n");
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
        "js" | "mjs" => "javascript",
        "ts" | "mts" => "typescript",
        "py" => "python",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "sh" | "bash" | "zsh" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "md" => "markdown",
        _ => "plaintext",
    }
    .to_string()
}
