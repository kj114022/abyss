use lazy_static::lazy_static;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub fn highlight_code(code: &str, extension: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension(extension)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension("txt"))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    // Use a specific theme, fallback to defaults if missing
    let theme = THEME_SET
        .themes
        .get("base16-ocean.dark")
        .or_else(|| THEME_SET.themes.values().next()) // Fallback to any available
        .expect("No themes available"); // Should never happen with load_defaults

    let mut h = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();

    for line_str in code.lines() {
        // HighlightLines works best with newlines included, but code.lines() strips them.
        // We'll highlight the line content.
        let ranges: Vec<(SyntectStyle, &str)> =
            h.highlight_line(line_str, &SYNTAX_SET).unwrap_or_default();

        let spans: Vec<Span> = ranges
            .into_iter()
            .map(|(style, text)| {
                let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                Span::styled(text.to_string(), ratatui::style::Style::default().fg(fg))
            })
            .collect();

        lines.push(Line::from(spans));
    }

    lines
}
