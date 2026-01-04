pub mod app;
pub mod highlight;
pub mod tree;
pub mod ui;

use crate::config::AbyssConfig;
use crate::runner::{ScanEvent, discover_files, process_files};
use crate::tui::app::{AppState, AppStep};
use crate::tui::tree::build_tree;
use crate::tui::ui::draw_ui;
use std::path::PathBuf;

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::thread;
use std::{io, time::Duration};

#[derive(Debug)]
enum TuiAction {
    Quit,
    Rescan(Box<AbyssConfig>),
}

pub fn start_tui(mut config: AbyssConfig) -> Result<()> {
    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Main Loop (supports Rescan)
    let result = loop {
        // Setup State
        let mut app_state = AppState::new(config.clone());

        // Start Discovery Thread
        let (tx, rx): (Sender<ScanEvent>, Receiver<ScanEvent>) = unbounded();
        // Channel for Discovery Result
        let (result_tx, result_rx) = unbounded::<(Vec<PathBuf>, PathBuf)>();

        let config_clone = config.clone();
        let tx_clone = tx.clone();

        thread::spawn(
            move || match discover_files(&config_clone, Some(tx_clone.clone())) {
                Ok((paths, root)) => {
                    let _ = result_tx.send((paths, root));
                }
                Err(e) => {
                    let _ = tx_clone.send(ScanEvent::Error(e.to_string()));
                }
            },
        );

        // Run App
        match run_app(
            &mut terminal,
            &mut app_state,
            rx,
            result_rx,
            config.clone(),
            tx,
        ) {
            Ok(TuiAction::Quit) => break Ok(()),
            Ok(TuiAction::Rescan(new_config)) => {
                config = *new_config;
                continue; // Restart loop with new config
            }
            Err(e) => break Err(anyhow::anyhow!(e)),
        }
    };

    // 3. Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    rx: Receiver<ScanEvent>,
    result_rx: Receiver<(Vec<PathBuf>, PathBuf)>,
    _config: AbyssConfig,
    tx: Sender<ScanEvent>, // Needed to pass to process_files thread later
) -> io::Result<TuiAction> {
    // Store discovery result locally until confirmed
    let mut pending_discovery: Option<(Vec<PathBuf>, PathBuf)> = None;

    loop {
        state.on_tick();
        terminal.draw(|f| draw_ui(f, state))?;

        #[allow(clippy::collapsible_if)]
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Global keys
                match key.code {
                    KeyCode::Char('q') => return Ok(TuiAction::Quit),
                    KeyCode::Char('h') => state.show_help = !state.show_help,
                    KeyCode::Tab => state.next_tab(),
                    _ => {}
                }

                if state.active_tab == 1 {
                    // Settings Tab Input
                    match key.code {
                        KeyCode::Down => state.next_config_item(),
                        KeyCode::Up => state.previous_config_item(),
                        KeyCode::Enter | KeyCode::Char(' ') => state.toggle_config_bool(),
                        KeyCode::Right => state.increase_config_value(),
                        KeyCode::Left => state.decrease_config_value(),
                        KeyCode::Char('r') => {
                            // Trigger Rescan
                            return Ok(TuiAction::Rescan(Box::new(state.config.clone())));
                        }
                        _ => {}
                    }
                } else if state.active_tab == 0 {
                    // Files Tab Input
                    match key.code {
                        KeyCode::Down => state.next_file(),
                        KeyCode::Up => state.previous_file(),
                        KeyCode::Esc => state.unselect(),
                        KeyCode::Char(' ') => {
                            state.toggle_selection();
                        }
                        KeyCode::Right => {
                            state.toggle_expand();
                        }
                        KeyCode::Left => {
                            state.toggle_expand();
                        }
                        KeyCode::Enter => {
                            #[allow(clippy::collapsible_if)]
                            if state.step == AppStep::FileSelection {
                                if let Some((_, root)) = &pending_discovery {
                                    // Transition to Processing
                                    state.step = AppStep::Processing;
                                    state.scanned_files.clear();
                                    state.status_message = "PROCESSING...".to_string();

                                    // Filter selected paths from tree
                                    let selected_paths: Vec<PathBuf> =
                                        if let Some(tree) = &state.file_tree {
                                            tree.collect_selected_paths()
                                        } else {
                                            Vec::new()
                                        };

                                    state.total_files = selected_paths.len();

                                    // Spawn processing thread
                                    let root_clone = root.clone();
                                    let config_clone = state.config.clone(); // Use the edited config!
                                    let tx_clone = tx.clone();
                                    thread::spawn(move || {
                                        if let Err(e) = process_files(
                                            selected_paths,
                                            root_clone,
                                            config_clone,
                                            Some(tx_clone.clone()),
                                        ) {
                                            let _ = tx_clone.send(ScanEvent::Error(e.to_string()));
                                        }
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Check for Discovery Result
        if let Ok((paths, root)) = result_rx.try_recv() {
            state.step = AppStep::FileSelection;
            state.status_message =
                "SELECT FILES (Arrows: Expand/Collapse/Nav, Space: Toggle, Enter: Process)"
                    .to_string();

            state.discovered_paths = paths.clone();
            state.file_tree = Some(build_tree(&root, paths));
            state.total_files = state.discovered_paths.len();
            pending_discovery = Some((state.discovered_paths.clone(), root));
        }

        // Handle Events
        while let Ok(event) = rx.try_recv() {
            match event {
                ScanEvent::StartScanning => {
                    // Only update status if we are in initial scanning phase
                    if state.step == AppStep::Scanning {
                        state.status_message = "SCANNING ACTIVE".to_string();
                        state.add_log("Scan initiated.".to_string());
                        state.start_time = Some(std::time::Instant::now());
                    }
                }
                ScanEvent::FilesFound(n) => {
                    if state.step == AppStep::Scanning {
                        state.add_log(format!("Discovered {} files.", n));
                    }
                }
                ScanEvent::FileProcessed(path) => {
                    state.processed_count += 1;
                    state.scanned_files.push(path);
                }
                ScanEvent::TokenCountUpdate(n) => {
                    state.total_tokens = n;
                }
                ScanEvent::Complete(msg) => {
                    state.is_complete = true;
                    // state.step = AppStep::Done; // Maybe keep it as Processing/Done?
                    // Previous code set app.step = Done.
                    state.step = AppStep::Done;
                    state.status_message = "TASK COMPLETE".to_string();
                    state.add_log(msg);
                }
                ScanEvent::Error(e) => {
                    state.add_log(format!("ERROR: {}", e));
                    state.status_message = "SYSTEM ERROR".to_string();
                }
            }
        }
    }
}
