use crate::tui::app::{AppState, AppStep};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
};

// use crate::tui::highlight::highlight_code;
use ratatui::widgets::{BorderType, Tabs};

pub fn draw_ui(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Title + Tabs
                Constraint::Min(5),    // Main Content
                Constraint::Length(3), // Status
                Constraint::Length(6), // Logs
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_header_tabs(f, state, chunks[0]);

    match state.active_tab {
        0 => {
            // Files Tab
            match state.step {
                AppStep::FileSelection => draw_selection_list(f, state, chunks[1]),
                _ => draw_progress_and_list(f, state, chunks[1]),
            }
        }
        1 => {
            // Settings Tab
            draw_config_form(f, state, chunks[1]);
        }
        _ => {}
    }

    draw_status(f, state, chunks[2]);
    draw_logs(f, state, chunks[3]);

    if state.show_help {
        draw_help_overlay(f);
    }
}

fn draw_header_tabs(f: &mut Frame, state: &mut AppState, area: Rect) {
    let titles = vec![" [1] Files ", " [2] Settings "];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Abyss Scanner"),
        )
        .select(state.active_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        );
    f.render_widget(tabs, area);
}

fn draw_config_form(f: &mut Frame, state: &mut AppState, area: Rect) {
    let depth_str = match state.config.max_depth {
        Some(d) => format!("{}", d),
        None => "Unlimited".to_string(),
    };

    let items = vec![
        ListItem::new(format!("Max Depth: {}", depth_str)),
        ListItem::new(format!("No Tokens: {}", state.config.no_tokens)),
        ListItem::new(format!("Clipboard: {}", state.config.clipboard_copy)),
        ListItem::new(format!("Output Format: {:?}", state.config.output_format)),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .title("Configuration (Nav: Arrows or h/j/k/l, Space: Toggle)"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 30, 30))
                .fg(Color::Rgb(255, 215, 0)) // Gold
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state.config_list_state);
}

fn draw_selection_list(f: &mut Frame, state: &mut AppState, area: Rect) {
    // Split into Tree and Preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    let tree_area = chunks[0];
    let preview_area = chunks[1];

    // Split tree area for Search Bar if needed
    let (search_rect, list_rect) = if state.is_searching || !state.search_query.is_empty() {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref()) // List first, Search at bottom? Or Top?
            // Let's put Search at top for "active filtering" feel
            .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
            .split(tree_area);
        (Some(v_chunks[0]), v_chunks[1])
    } else {
        (None, tree_area)
    };

    if let Some(area) = search_rect {
        let query_text = if state.search_query.is_empty() {
            Span::styled("Type to filter...", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw(&state.search_query)
        };

        let border_style = if state.is_searching {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let search_block = Block::default()
            .borders(Borders::ALL)
            .title("Filter")
            .border_style(border_style);

        let input = Paragraph::new(query_text).block(search_block);
        f.render_widget(input, area);
    }

    let tree = match &state.file_tree {
        Some(t) => t,
        None => return,
    };

    // Flatten tree to get visible nodes
    let visible_nodes = tree.flatten();

    // Determine offset for scrolling
    // Simple scrolling logic: Ensure tree_index is visible
    let list_height = list_rect.height as usize;
    if list_height == 0 {
        return;
    }

    let offset = if state.tree_index >= list_height {
        state.tree_index - list_height + 1
    } else {
        0
    };

    let items: Vec<ListItem> = visible_nodes
        .iter()
        .skip(offset)
        .take(list_height)
        .enumerate()
        .map(|(idx, node)| {
            let actual_idx = offset + idx;
            let is_selected_row = actual_idx == state.tree_index;

            // Indentation
            let indent = " ".repeat(node.depth * 2);

            // Icon
            let icon = if node.is_dir {
                if node.is_expanded { "v " } else { "> " }
            } else {
                "  "
            };

            // Checkbox
            let checkbox = if node.is_selected { "[x] " } else { "[ ] " };

            // Name
            let name = &node.name;

            let content = format!("{}{}{}{}", indent, icon, checkbox, name);

            let style = if is_selected_row {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if node.is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .title("File Tree"),
        )
        .highlight_style(Style::default());

    f.render_widget(list, list_rect);

    // Preview Pane
    draw_preview_pane(f, state, preview_area);
}

fn draw_preview_pane(f: &mut Frame, state: &mut AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .title("Preview");

    if let Some(path) = &state.preview_path {
        // let ext = path.extension().unwrap_or_default().to_string_lossy();
        // let highlighted = highlight_code(&state.preview_content, &ext);

        let p = Paragraph::new(state.preview_highlighted.clone())
            .block(block.title(format!(
                "Preview: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )))
            .wrap(Wrap { trim: false }); // No wrap for code usually, but TUI limitation
        f.render_widget(p, area);
    } else {
        let p = Paragraph::new("Select a file to preview")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    }
}

fn draw_progress_and_list(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    // 1. Progress Column
    let progress_area = chunks[0];
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Gauge
                Constraint::Min(2),    // Stats
            ]
            .as_ref(),
        )
        .split(progress_area);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(state.progress_percent())
        .label(format!("{}%", state.progress_percent()));
    f.render_widget(gauge, inner_chunks[0]);

    let stats_text = vec![
        Line::from(vec![
            Span::raw("Processed: "),
            Span::styled(
                format!("{}/{}", state.processed_count, state.total_files),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Total Tokens: "),
            Span::styled(
                format!("{}", state.total_tokens),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("ETA: "),
            Span::styled(
                match state.eta_seconds() {
                    Some(s) => format!("{}s", s),
                    None => "Calculating...".to_string(),
                },
                Style::default().fg(Color::Blue),
            ),
        ]),
    ];
    let stats =
        Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title("Stats"));
    f.render_widget(stats, inner_chunks[1]);

    // 2. Scanned Files List
    let items: Vec<ListItem> = state
        .scanned_files
        .iter()
        .rev() // Show newest at top if we want, or match scroll
        .take(20) // Show last 20
        .map(|p| ListItem::new(p.file_name().unwrap_or_default().to_string_lossy()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Processing Stream"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, chunks[1], &mut state.list_state);
}

fn draw_status(f: &mut Frame, state: &mut AppState, area: Rect) {
    let status_color = if state.status_message.contains("ERROR") {
        Color::Red
    } else if state.is_complete {
        Color::Green
    } else {
        Color::Cyan
    };

    // Simple pulse effect on color (visual only)
    let style = Style::default().fg(status_color);

    let status = Paragraph::new(Line::from(vec![Span::styled(&state.status_message, style)]))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, area);
}

fn draw_logs(f: &mut Frame, state: &mut AppState, area: Rect) {
    let logs: Vec<ListItem> = state
        .logs
        .iter()
        .rev()
        .map(|s| ListItem::new(Span::raw(s)))
        .collect();

    let list = List::new(logs).block(Block::default().borders(Borders::ALL).title("Logs"));
    f.render_widget(list, area);
}

fn draw_help_overlay(f: &mut Frame) {
    let block = Block::default().borders(Borders::ALL).title("Help");
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area); // Clear background

    let text = vec![
        Line::from("Abyss TUI Help"),
        Line::from(""),
        Line::from("Scanning Mode:"),
        Line::from("  Monitors progress of automatic scan."),
        Line::from(""),
        Line::from("Selection Mode:"),
        Line::from("  Space: Toggle file selection"),
        Line::from("  Enter: Confirm and start processing"),
        Line::from("  Arrows or j/k: Navigate list"),
        Line::from("  Left/Right or h/l: Collapse/Expand"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  q: Quit"),
        Line::from("  ?: Toggle Help"),
    ];

    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1]);

    layout[1]
}
