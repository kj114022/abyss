use crate::config::AbyssConfig;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AppStep {
    Scanning,
    FileSelection,
    Processing,
    Done,
}

pub struct AppState {
    pub scanned_files: Vec<PathBuf>,
    pub list_state: ListState,
    pub processed_count: usize,
    pub total_files: usize,
    pub total_tokens: usize,
    pub status_message: String,
    pub is_complete: bool,
    pub logs: Vec<String>,
    pub pulse: f64,
    pub tick_count: usize,
    pub start_time: Option<Instant>,
    pub show_help: bool,

    // Selection Mode
    pub step: AppStep,

    // Discovery
    pub discovered_paths: Vec<PathBuf>,
    pub file_tree: Option<crate::tui::tree::FileNode>,
    pub tree_index: usize,
    pub selection_list_state: ListState,
    pub preview_path: Option<PathBuf>,
    pub preview_content: String,
    pub preview_highlighted: Vec<ratatui::text::Line<'static>>,

    // Config Mode
    pub active_tab: usize,
    pub config: AbyssConfig,
    pub config_list_state: ListState,

    // Search
    pub search_query: String,
    pub is_searching: bool,
}

impl AppState {
    pub fn new(config: AbyssConfig) -> Self {
        Self {
            scanned_files: Vec::new(),
            list_state: ListState::default(),
            processed_count: 0,
            total_files: 0,
            total_tokens: 0,
            status_message: "ENTERING THE ABYSS...".to_string(),
            is_complete: false,
            logs: Vec::new(),
            pulse: 0.0,
            tick_count: 0,
            start_time: None,
            show_help: false,

            step: AppStep::Scanning,
            discovered_paths: Vec::new(),
            file_tree: None,
            tree_index: 0,
            selection_list_state: ListState::default(),
            preview_path: None,
            preview_content: String::new(),
            preview_highlighted: Vec::new(),

            active_tab: 0, // 0 = Files, 1 = Settings
            config,
            config_list_state: ListState::default(),

            search_query: String::new(),
            is_searching: false,
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count += 1;
        let t = self.tick_count as f64 * 0.1;
        self.pulse = (t.sin() + 1.0) / 2.0;
    }

    pub fn add_log(&mut self, log: String) {
        if self.logs.len() > 10 {
            self.logs.remove(0);
        }
        self.logs.push(log);
    }

    pub fn progress_percent(&self) -> u16 {
        if self.total_files == 0 {
            return 0;
        }
        ((self.processed_count as f64 / self.total_files as f64) * 100.0).min(100.0) as u16
    }

    pub fn eta(&self) -> String {
        match self.eta_seconds() {
            Some(s) => {
                if s > 60 {
                    format!("{}m {}s", s / 60, s % 60)
                } else {
                    format!("{}s", s)
                }
            }
            None => "CALCULATING...".to_string(),
        }
    }

    pub fn eta_seconds(&self) -> Option<u64> {
        let start = self.start_time?;
        if self.processed_count == 0 {
            return None;
        }
        let elapsed = start.elapsed().as_secs_f64();
        let rate = self.processed_count as f64 / elapsed;
        if rate == 0.0 {
            return None;
        }
        let remaining = self.total_files.saturating_sub(self.processed_count) as f64;
        Some((remaining / rate) as u64)
    }

    pub fn next_file(&mut self) {
        match self.step {
            AppStep::FileSelection => {
                if let Some(tree) = &self.file_tree {
                    let visible_count = tree.visible_count();
                    if visible_count == 0 {
                        return;
                    }
                    if self.tree_index < visible_count - 1 {
                        self.tree_index += 1;
                    } else {
                        self.tree_index = 0;
                    }
                }
            }
            _ => {
                if self.scanned_files.is_empty() {
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= self.scanned_files.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
        }
    }

    pub fn previous_file(&mut self) {
        match self.step {
            AppStep::FileSelection => {
                if let Some(tree) = &self.file_tree {
                    let visible_count = tree.visible_count();
                    if visible_count == 0 {
                        return;
                    }
                    if self.tree_index > 0 {
                        self.tree_index -= 1;
                    } else {
                        self.tree_index = visible_count - 1;
                    }
                }
            }
            _ => {
                if self.scanned_files.is_empty() {
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.scanned_files.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
        }
    }

    pub fn toggle_selection(&mut self) {
        #[allow(clippy::collapsible_if)]
        if self.step == AppStep::FileSelection {
            if let Some(tree) = &mut self.file_tree {
                tree.toggle_select_at_index(self.tree_index);
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        #[allow(clippy::collapsible_if)]
        if self.step == AppStep::FileSelection {
            if let Some(tree) = &mut self.file_tree {
                tree.toggle_expand_at_index(self.tree_index);
            }
        }
    }

    pub fn unselect(&mut self) {
        match self.step {
            AppStep::FileSelection => self.tree_index = 0,
            _ => self.list_state.select(None),
        }
        self.update_preview();
    }

    pub fn update_preview(&mut self) {
        // Determine current selected path
        let mut path_to_load = None;
        #[allow(clippy::collapsible_if)]
        if self.step == AppStep::FileSelection {
            if let Some(tree) = &self.file_tree {
                // Find the node at tree_index in the visible flattened list
                let visible = tree.flatten();
                #[allow(clippy::collapsible_if)]
                if let Some(node) = visible.get(self.tree_index) {
                    if !node.is_dir {
                        path_to_load = Some(node.path.clone());
                    }
                }
            }
        }

        // If path changed, load it
        if path_to_load != self.preview_path {
            self.preview_path = path_to_load.clone();
            if let Some(p) = path_to_load {
                // Load file content (first 100 lines)
                if let Ok(content) = std::fs::read_to_string(&p) {
                    self.preview_content = content.lines().take(100).collect::<Vec<_>>().join("\n");
                } else {
                    self.preview_content = "[Binary or unsociable file]".to_string();
                }
                let ext = p.extension().unwrap_or_default().to_string_lossy();
                self.preview_highlighted = crate::tui::highlight::highlight_code(&self.preview_content, &ext);
            } else {
                self.preview_content.clear();
                self.preview_highlighted.clear();
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % 2;
    }

    pub fn next_config_item(&mut self) {
        let i = match self.config_list_state.selected() {
            Some(i) => {
                if i >= 3 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.config_list_state.select(Some(i));
    }

    pub fn previous_config_item(&mut self) {
        let i = match self.config_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    3
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.config_list_state.select(Some(i));
    }

    pub fn toggle_config_bool(&mut self) {
        if let Some(i) = self.config_list_state.selected() {
            match i {
                0 => {} // Max Depth
                1 => self.config.no_tokens = !self.config.no_tokens,
                2 => self.config.clipboard_copy = !self.config.clipboard_copy,
                3 => self.cycle_output_format(),
                _ => {}
            }
        }
    }

    fn cycle_output_format(&mut self) {
        use crate::config::OutputFormat;
        self.config.output_format = match self.config.output_format {
            OutputFormat::Xml => OutputFormat::Json,
            OutputFormat::Json => OutputFormat::Markdown,
            OutputFormat::Markdown => OutputFormat::Plain,
            OutputFormat::Plain => OutputFormat::Xml,
        };
    }

    pub fn increase_config_value(&mut self) {
        if self.config_list_state.selected() == Some(0) {
            // Max Depth
            if let Some(d) = self.config.max_depth {
                self.config.max_depth = Some(d + 1);
            } else {
                self.config.max_depth = Some(1);
            }
        }
    }

    pub fn decrease_config_value(&mut self) {
        if self.config_list_state.selected() == Some(0) {
            // Max Depth
            if let Some(d) = self.config.max_depth {
                if d > 0 {
                    self.config.max_depth = Some(d - 1);
                } else {
                    self.config.max_depth = None;
                }
            } else {
                self.config.max_depth = Some(5);
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AbyssConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AbyssConfig;

    #[test]
    fn test_progress_percent() {
        let mut state = AppState::new(AbyssConfig::default());
        state.total_files = 10;
        state.processed_count = 5;
        assert_eq!(state.progress_percent(), 50);

        state.processed_count = 10;
        assert_eq!(state.progress_percent(), 100);

        state.total_files = 0;
        assert_eq!(state.progress_percent(), 0);
    }

    #[test]
    fn test_eta_is_none_initially() {
        let state = AppState::new(AbyssConfig::default());
        assert!(state.eta_seconds().is_none());
    }
}
