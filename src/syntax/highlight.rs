pub struct Highlighter {
    pub language: String,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            language: "plaintext".to_string(),
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        self.language = lang.to_string();
    }

    pub fn get_comment_prefix(&self) -> Option<&'static str> {
        match self.language.as_str() {
            "python" | "ruby" | "shell" | "bash" | "yaml" => Some("#"),
            "rust" | "javascript" | "typescript" | "go" | "java" | "c" | "cpp" | "css" | "json" => Some("//"),
            "html" => Some("<!--"),
            _ => Some("#"),
        }
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}
