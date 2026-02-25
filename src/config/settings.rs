use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub tab_size: usize,
    pub use_spaces: bool,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
    pub word_wrap: bool,
    pub auto_save: bool,
    pub theme: String,
    pub show_tabs: bool,
    pub show_status_bar: bool,
    pub show_help: bool,
    pub mouse_support: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_spaces: true,
            show_line_numbers: true,
            highlight_current_line: true,
            word_wrap: false,
            auto_save: false,
            theme: "monokai_pro".to_string(),
            show_tabs: true,
            show_status_bar: true,
            show_help: true,
            mouse_support: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(settings) = toml::from_str(&contents) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    #[allow(dead_code)]
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let toml = toml::to_string_pretty(self).unwrap();
            std::fs::write(path, toml)?;
        }
        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("nova").join("config.toml"))
    }
}
