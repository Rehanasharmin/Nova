use ratatui::style::Color;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    #[allow(dead_code)]
    pub selection: Color,
    pub cursor: Color,
    pub cursor_line: Color,
    pub line_number: Color,
    pub line_number_current: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub help_bar_bg: Color,
    pub help_bar_fg: Color,
    pub border: Color,
    pub title_bg: Color,
    pub title_fg: Color,
    pub accent: Color,
    pub scrollbar: Color,
}

impl Theme {
    pub fn monokai_pro() -> Self {
        Self {
            name: "monokai_pro".to_string(),
            background: Color::Rgb(39, 40, 34),
            foreground: Color::Rgb(249, 238, 230),
            selection: Color::Rgb(117, 113, 97),
            cursor: Color::Rgb(249, 238, 230),
            cursor_line: Color::Rgb(50, 52, 46),
            line_number: Color::Rgb(100, 100, 100),
            line_number_current: Color::Rgb(255, 200, 100),
            status_bar_bg: Color::Rgb(30, 30, 25),
            status_bar_fg: Color::Rgb(200, 200, 190),
            help_bar_bg: Color::Rgb(30, 30, 25),
            help_bar_fg: Color::Rgb(150, 150, 140),
            border: Color::Rgb(60, 58, 53),
            title_bg: Color::Rgb(35, 35, 30),
            title_fg: Color::Rgb(255, 200, 100),
            accent: Color::Rgb(255, 200, 100),
            scrollbar: Color::Rgb(80, 75, 70),
        }
    }

    pub fn nord_frost() -> Self {
        Self {
            name: "nord_frost".to_string(),
            background: Color::Rgb(46, 52, 64),
            foreground: Color::Rgb(216, 222, 233),
            selection: Color::Rgb(59, 66, 82),
            cursor: Color::Rgb(136, 192, 208),
            cursor_line: Color::Rgb(59, 66, 82),
            line_number: Color::Rgb(76, 86, 106),
            line_number_current: Color::Rgb(136, 192, 208),
            status_bar_bg: Color::Rgb(40, 46, 56),
            status_bar_fg: Color::Rgb(216, 222, 233),
            help_bar_bg: Color::Rgb(40, 46, 56),
            help_bar_fg: Color::Rgb(136, 192, 208),
            border: Color::Rgb(67, 79, 94),
            title_bg: Color::Rgb(40, 46, 56),
            title_fg: Color::Rgb(136, 192, 208),
            accent: Color::Rgb(136, 192, 208),
            scrollbar: Color::Rgb(80, 95, 110),
        }
    }

    pub fn dracula_vibrant() -> Self {
        Self {
            name: "dracula_vibrant".to_string(),
            background: Color::Rgb(40, 42, 54),
            foreground: Color::Rgb(248, 248, 242),
            selection: Color::Rgb(69, 71, 90),
            cursor: Color::Rgb(255, 121, 198),
            cursor_line: Color::Rgb(60, 62, 80),
            line_number: Color::Rgb(90, 90, 110),
            line_number_current: Color::Rgb(255, 121, 198),
            status_bar_bg: Color::Rgb(30, 30, 45),
            status_bar_fg: Color::Rgb(200, 200, 195),
            help_bar_bg: Color::Rgb(30, 30, 45),
            help_bar_fg: Color::Rgb(139, 233, 253),
            border: Color::Rgb(80, 82, 100),
            title_bg: Color::Rgb(35, 37, 50),
            title_fg: Color::Rgb(255, 121, 198),
            accent: Color::Rgb(189, 147, 249),
            scrollbar: Color::Rgb(100, 100, 120),
        }
    }

    pub fn gruvbox_soft() -> Self {
        Self {
            name: "gruvbox_soft".to_string(),
            background: Color::Rgb(40, 40, 40),
            foreground: Color::Rgb(235, 219, 178),
            selection: Color::Rgb(80, 73, 69),
            cursor: Color::Rgb(254, 128, 25),
            cursor_line: Color::Rgb(55, 53, 50),
            line_number: Color::Rgb(100, 90, 80),
            line_number_current: Color::Rgb(254, 128, 25),
            status_bar_bg: Color::Rgb(35, 35, 35),
            status_bar_fg: Color::Rgb(200, 185, 165),
            help_bar_bg: Color::Rgb(35, 35, 35),
            help_bar_fg: Color::Rgb(160, 145, 125),
            border: Color::Rgb(70, 65, 60),
            title_bg: Color::Rgb(30, 30, 30),
            title_fg: Color::Rgb(254, 128, 25),
            accent: Color::Rgb(184, 187, 38),
            scrollbar: Color::Rgb(90, 85, 80),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "one_dark".to_string(),
            background: Color::Rgb(40, 44, 52),
            foreground: Color::Rgb(220, 223, 228),
            selection: Color::Rgb(57, 62, 70),
            cursor: Color::Rgb(97, 175, 239),
            cursor_line: Color::Rgb(50, 54, 62),
            line_number: Color::Rgb(90, 95, 105),
            line_number_current: Color::Rgb(97, 175, 239),
            status_bar_bg: Color::Rgb(33, 37, 43),
            status_bar_fg: Color::Rgb(190, 195, 200),
            help_bar_bg: Color::Rgb(33, 37, 43),
            help_bar_fg: Color::Rgb(97, 175, 239),
            border: Color::Rgb(60, 65, 75),
            title_bg: Color::Rgb(30, 34, 40),
            title_fg: Color::Rgb(97, 175, 239),
            accent: Color::Rgb(97, 175, 239),
            scrollbar: Color::Rgb(80, 85, 95),
        }
    }

    pub fn get_theme(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "monokai_pro" | "monokai" => Self::monokai_pro(),
            "nord_frost" | "nord" => Self::nord_frost(),
            "dracula_vibrant" | "dracula" => Self::dracula_vibrant(),
            "gruvbox_soft" | "gruvbox" => Self::gruvbox_soft(),
            "one_dark" => Self::one_dark(),
            _ => Self::monokai_pro(),
        }
    }

    pub fn all_themes() -> Vec<String> {
        vec![
            "monokai_pro".to_string(),
            "nord_frost".to_string(),
            "dracula_vibrant".to_string(),
            "gruvbox_soft".to_string(),
            "one_dark".to_string(),
        ]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::monokai_pro()
    }
}
