use std::sync::{
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};

use crate::app::{input::Action, save, ClickArea};
use crate::ui::view::{
    ConfirmDialogButtonRole, ConfirmDialogButtonViewModel, ConfirmDialogViewModel,
};

use super::{confirm_dialog, AppContext, InGameScreen, Screen, ScreenTransition};

const LOADING_FRAMES: &[&str] = &["[=  ]", "[== ]", "[===]", "[ ==]", "[  =]"];
const SPINNER_TICK_DELAY: u8 = 5;

#[derive(Clone, Debug)]
pub struct DeleteConfirmState {
    pub path: std::path::PathBuf,
    pub city_name: String,
}

pub struct LoadCityState {
    pub saves: Vec<save::SaveEntry>,
    pub saves_snapshot: Arc<[save::SaveEntry]>,
    pub selected: usize,
    pub row_areas: Vec<ClickArea>,
    pub dialog_items: Vec<ClickArea>,
    pub is_loading: bool,
    pub confirm_delete: Option<DeleteConfirmState>,
    confirm_selected: usize,
    spinner_frame: usize,
    spinner_ticks: u8,
    saves_rx: Option<Receiver<save::SaveDiscoveryUpdate>>,
}

pub struct LoadCityScreen {
    pub state: LoadCityState,
}

impl LoadCityScreen {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            save::discover_saves(tx);
        });

        Self {
            state: LoadCityState {
                saves: Vec::new(),
                saves_snapshot: Arc::from(Vec::<save::SaveEntry>::new()),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: true,
                confirm_delete: None,
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: Some(rx),
            },
        }
    }

    fn sync_snapshot(&mut self) {
        save::sort_save_entries(&mut self.state.saves);
        self.state.saves_snapshot = Arc::from(self.state.saves.clone());
        self.state.selected = self
            .state
            .selected
            .min(self.state.saves_snapshot.len().saturating_sub(1));
    }

    fn finish_loading(&mut self) {
        self.state.is_loading = false;
        self.state.saves_rx = None;
        self.state.spinner_frame = 0;
        self.state.spinner_ticks = 0;
    }

    fn poll_loading(&mut self) {
        let mut changed = false;
        let mut finished = false;

        if let Some(rx) = &self.state.saves_rx {
            loop {
                match rx.try_recv() {
                    Ok(save::SaveDiscoveryUpdate::Entry(entry)) => {
                        self.state.saves.push(entry);
                        changed = true;
                    }
                    Ok(save::SaveDiscoveryUpdate::Finished) => {
                        finished = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }

        if changed {
            self.sync_snapshot();
        }

        if finished {
            self.finish_loading();
        } else if self.state.is_loading {
            self.state.spinner_ticks = self.state.spinner_ticks.saturating_add(1);
            if self.state.spinner_ticks >= SPINNER_TICK_DELAY {
                self.state.spinner_ticks = 0;
                self.state.spinner_frame = (self.state.spinner_frame + 1) % LOADING_FRAMES.len();
            }
        }
    }

    fn open_delete_prompt(&mut self) {
        let Some(entry) = self.state.saves_snapshot.get(self.state.selected) else {
            return;
        };

        self.state.confirm_delete = Some(DeleteConfirmState {
            path: entry.path.clone(),
            city_name: entry.city_name.clone(),
        });
        self.state.confirm_selected = 0;
    }

    fn cycle_delete_prompt_selection(&mut self, delta: i32) {
        if let Some(dialog) = self.confirm_dialog_view_model() {
            confirm_dialog::cycle_selection(
                &mut self.state.confirm_selected,
                dialog.button_count(),
                delta,
            );
        }
    }

    fn confirm_delete(&mut self) {
        let Some(dialog) = self.confirm_dialog_view_model() else {
            return;
        };
        let Some(confirm) = self.state.confirm_delete.take() else {
            return;
        };

        match dialog.selected_role() {
            Some(ConfirmDialogButtonRole::Accept) => {
                if save::delete_city(&confirm.path).is_ok() {
                    self.state.saves.retain(|entry| entry.path != confirm.path);
                    self.sync_snapshot();
                }
            }
            Some(ConfirmDialogButtonRole::Cancel)
            | Some(ConfirmDialogButtonRole::Alternate)
            | None => {}
        }

        self.state.confirm_selected = 0;
        self.state.dialog_items.clear();
    }

    fn confirm_dialog_view_model(&self) -> Option<ConfirmDialogViewModel> {
        let confirm = self.state.confirm_delete.as_ref()?;
        let buttons = vec![
            ConfirmDialogButtonViewModel {
                label: "Delete".to_string(),
                role: ConfirmDialogButtonRole::Accept,
            },
            ConfirmDialogButtonViewModel {
                label: "Cancel".to_string(),
                role: ConfirmDialogButtonRole::Cancel,
            },
        ];
        Some(ConfirmDialogViewModel {
            title: "Delete City".to_string(),
            message: format!("Delete saved city \"{}\"?", confirm.city_name),
            selected: self
                .state
                .confirm_selected
                .min(buttons.len().saturating_sub(1)),
            buttons,
        })
    }
}

impl Screen for LoadCityScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_event(
        &mut self,
        _event: &crossterm::event::Event,
        _context: AppContext,
    ) -> Option<ScreenTransition> {
        self.poll_loading();
        None
    }

    fn on_tick(&mut self, _context: AppContext) {
        self.poll_loading();
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        self.poll_loading();
        if self.state.confirm_delete.is_some() {
            return match action {
                Action::Quit => None,
                Action::MenuBack
                | Action::CharInput('n')
                | Action::CharInput('N')
                | Action::CharInput('c')
                | Action::CharInput('C') => {
                    if let Some(dialog) = self.confirm_dialog_view_model() {
                        if let Some(index) = dialog.index_for_role(ConfirmDialogButtonRole::Cancel)
                        {
                            self.state.confirm_selected = index;
                        }
                    }
                    self.confirm_delete();
                    None
                }
                Action::MenuSelect => {
                    self.confirm_delete();
                    None
                }
                Action::CharInput('y') | Action::CharInput('Y') => {
                    if let Some(dialog) = self.confirm_dialog_view_model() {
                        if let Some(index) = dialog.index_for_role(ConfirmDialogButtonRole::Accept)
                        {
                            self.state.confirm_selected = index;
                        }
                    }
                    self.confirm_delete();
                    None
                }
                Action::MoveCursor(dx, dy) => {
                    if dx < 0 || dy < 0 {
                        self.cycle_delete_prompt_selection(-1);
                    } else if dx > 0 || dy > 0 {
                        self.cycle_delete_prompt_selection(1);
                    }
                    None
                }
                Action::MouseClick { col, row } => {
                    if let Some(index) = self
                        .state
                        .dialog_items
                        .iter()
                        .position(|area| area.contains(col, row))
                    {
                        self.state.confirm_selected = index;
                        self.confirm_delete();
                    }
                    None
                }
                _ => None,
            };
        }

        let count = self.state.saves_snapshot.len();
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(_, dy) if count > 0 => {
                if count > 0 {
                    self.state.selected = if dy > 0 {
                        (self.state.selected + 1) % count
                    } else {
                        self.state
                            .selected
                            .checked_sub(1)
                            .unwrap_or(count.saturating_sub(1))
                    };
                }
                None
            }
            Action::MouseClick { col, row } if count > 0 => {
                for (idx, area) in self.state.row_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.selected = idx;
                        return None;
                    }
                }
                None
            }
            Action::MenuSelect if count > 0 => {
                if let Some(entry) = self.state.saves_snapshot.get(self.state.selected) {
                    match save::load_city(&entry.path) {
                        Ok((map, sim)) => {
                            if let Some(tx) = context.cmd_tx {
                                let _ = tx.send(crate::core::engine::EngineCommand::ReplaceState {
                                    map,
                                    sim,
                                });
                            }
                            Some(ScreenTransition::Replace(Box::new(InGameScreen::new())))
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Action::CharInput('d') | Action::CharInput('D') if count > 0 => {
                self.open_delete_prompt();
                None
            }
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::LoadCity(crate::ui::view::LoadCityViewModel {
            saves: self.state.saves_snapshot.clone(),
            selected: self.state.selected,
            is_loading: self.state.is_loading,
            loading_indicator: LOADING_FRAMES[self.state.spinner_frame],
            confirm_dialog: self.confirm_dialog_view_model(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{map::Map, sim::SimState};
    use crossterm::event::{Event, MouseEvent, MouseEventKind};
    use std::sync::{Arc, RwLock};
    use std::time::SystemTime;

    fn sample_entry() -> save::SaveEntry {
        save::SaveEntry {
            path: "test.json".into(),
            city_name: "Test City".to_string(),
            year: 1955,
            month: 8,
            population: 6789,
            treasury: 12_345,
            modified_at: Some(SystemTime::now()),
        }
    }

    #[test]
    fn on_tick_finishes_loading_when_results_arrive() {
        let (tx, rx) = mpsc::channel();
        tx.send(save::SaveDiscoveryUpdate::Entry(sample_entry()))
            .expect("test channel should accept entry");
        tx.send(save::SaveDiscoveryUpdate::Finished)
            .expect("test channel should accept finish");

        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: Vec::new(),
                saves_snapshot: Arc::from(Vec::<save::SaveEntry>::new()),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: true,
                confirm_delete: None,
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: Some(rx),
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        screen.on_tick(AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,

        });

        assert!(!screen.state.is_loading);
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }

    #[test]
    fn on_event_finishes_loading_when_ticks_are_starved() {
        let (tx, rx) = mpsc::channel();
        tx.send(save::SaveDiscoveryUpdate::Entry(sample_entry()))
            .expect("test channel should accept entry");
        tx.send(save::SaveDiscoveryUpdate::Finished)
            .expect("test channel should accept finish");

        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: Vec::new(),
                saves_snapshot: Arc::from(Vec::<save::SaveEntry>::new()),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: true,
                confirm_delete: None,
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: Some(rx),
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::empty(),
        });

        let transition = screen.on_event(
            &event,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(!screen.state.is_loading);
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }

    #[test]
    fn on_event_accumulates_partial_results_before_finish() {
        let (tx, rx) = mpsc::channel();
        tx.send(save::SaveDiscoveryUpdate::Entry(sample_entry()))
            .expect("test channel should accept entry");

        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: Vec::new(),
                saves_snapshot: Arc::from(Vec::<save::SaveEntry>::new()),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: true,
                confirm_delete: None,
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: Some(rx),
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::empty(),
        });

        let transition = screen.on_event(
            &event,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(screen.state.is_loading);
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }

    struct TestDir {
        path: std::path::PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "tuicity2000-load-city-tests-{}-{}-{}",
                label,
                std::process::id(),
                nonce
            ));
            std::fs::create_dir_all(&path).expect("temp dir should be creatable");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn delete_action_opens_confirm_prompt_for_selected_city() {
        let entry = sample_entry();
        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: vec![entry.clone()],
                saves_snapshot: Arc::from(vec![entry.clone()]),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: false,
                confirm_delete: None,
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: None,
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        let transition = screen.on_action(
            Action::CharInput('d'),
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        let confirm = screen
            .state
            .confirm_delete
            .as_ref()
            .expect("delete prompt should open");
        assert_eq!(confirm.city_name, entry.city_name);
        assert_eq!(confirm.path, entry.path);
        assert_eq!(screen.state.confirm_selected, 0);
    }

    #[test]
    fn confirming_delete_removes_city_and_file() {
        let dir = TestDir::new("delete");
        let path = dir.path.join("delete.tc2");
        std::fs::write(&path, b"delete me").expect("save file should be creatable");
        let entry = save::SaveEntry {
            path: path.clone(),
            city_name: "Delete Me".to_string(),
            year: 1955,
            month: 8,
            population: 6789,
            treasury: 12_345,
            modified_at: Some(SystemTime::now()),
        };
        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: vec![entry.clone()],
                saves_snapshot: Arc::from(vec![entry]),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: false,
                confirm_delete: Some(DeleteConfirmState {
                    path: path.clone(),
                    city_name: "Delete Me".to_string(),
                }),
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: None,
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        let transition = screen.on_action(
            Action::MenuSelect,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(!path.exists());
        assert!(screen.state.confirm_delete.is_none());
        assert!(screen.state.saves_snapshot.is_empty());
    }

    #[test]
    fn canceling_delete_keeps_city_and_file() {
        let dir = TestDir::new("cancel-delete");
        let path = dir.path.join("keep.tc2");
        std::fs::write(&path, b"keep me").expect("save file should be creatable");
        let entry = save::SaveEntry {
            path: path.clone(),
            city_name: "Keep Me".to_string(),
            year: 1955,
            month: 8,
            population: 6789,
            treasury: 12_345,
            modified_at: Some(SystemTime::now()),
        };
        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: vec![entry.clone()],
                saves_snapshot: Arc::from(vec![entry]),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: false,
                confirm_delete: Some(DeleteConfirmState {
                    path: path.clone(),
                    city_name: "Keep Me".to_string(),
                }),
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: None,
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        let transition = screen.on_action(
            Action::MenuBack,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(path.exists());
        assert!(screen.state.confirm_delete.is_none());
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }

    #[test]
    fn pressing_n_cancels_delete_without_removing_the_file() {
        let dir = TestDir::new("hotkey-cancel-delete");
        let path = dir.path.join("keep-on-n.tc2");
        std::fs::write(&path, b"keep me").expect("save file should be creatable");
        let entry = save::SaveEntry {
            path: path.clone(),
            city_name: "Keep On N".to_string(),
            year: 1955,
            month: 8,
            population: 6789,
            treasury: 12_345,
            modified_at: Some(SystemTime::now()),
        };
        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: vec![entry.clone()],
                saves_snapshot: Arc::from(vec![entry]),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: false,
                confirm_delete: Some(DeleteConfirmState {
                    path: path.clone(),
                    city_name: "Keep On N".to_string(),
                }),
                confirm_selected: 0,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: None,
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        let transition = screen.on_action(
            Action::CharInput('n'),
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(path.exists());
        assert!(screen.state.confirm_delete.is_none());
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }

    #[test]
    fn selecting_cancel_and_pressing_enter_keeps_city_and_file() {
        let dir = TestDir::new("enter-cancel-delete");
        let path = dir.path.join("keep-on-enter.tc2");
        std::fs::write(&path, b"keep me").expect("save file should be creatable");
        let entry = save::SaveEntry {
            path: path.clone(),
            city_name: "Keep On Enter".to_string(),
            year: 1955,
            month: 8,
            population: 6789,
            treasury: 12_345,
            modified_at: Some(SystemTime::now()),
        };
        let mut screen = LoadCityScreen {
            state: LoadCityState {
                saves: vec![entry.clone()],
                saves_snapshot: Arc::from(vec![entry]),
                selected: 0,
                row_areas: Vec::new(),
                dialog_items: Vec::new(),
                is_loading: false,
                confirm_delete: Some(DeleteConfirmState {
                    path: path.clone(),
                    city_name: "Keep On Enter".to_string(),
                }),
                confirm_selected: 1,
                spinner_frame: 0,
                spinner_ticks: 0,
                saves_rx: None,
            },
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));

        let cmd_tx = None;

        let transition = screen.on_action(
            Action::MenuSelect,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
    
            },
        );

        assert!(transition.is_none());
        assert!(path.exists());
        assert!(screen.state.confirm_delete.is_none());
        assert_eq!(screen.state.saves_snapshot.len(), 1);
    }
}
