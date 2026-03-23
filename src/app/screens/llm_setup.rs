use crate::{
    app::{config, input::Action, ClickArea},
    textgen::download::{self, DownloadHandle, DownloadProgress},
    ui::view::{ConfirmDialogButtonRole, ConfirmDialogButtonViewModel, ConfirmDialogViewModel},
};

use super::{confirm_dialog, AppContext, Screen, ScreenTransition};

pub struct LlmSetupState {
    pub selected: usize,
    pub row_areas: Vec<ClickArea>,
}

impl LlmSetupState {
    fn new() -> Self {
        Self {
            selected: 0,
            row_areas: Vec::new(),
        }
    }
}

#[derive(Clone)]
enum DownloadStatus {
    Idle,
    Downloading(String),
    Done,
    Failed(String),
}

#[derive(Clone, Copy)]
enum ConfirmAction {
    DownloadModel,
    DeleteModel,
}

pub struct LlmSetupScreen {
    pub state: LlmSetupState,
    download_handle: Option<DownloadHandle>,
    download_status: DownloadStatus,
    confirm_dialog: Option<ConfirmDialogViewModel>,
    confirm_action: Option<ConfirmAction>,
}

impl LlmSetupScreen {
    pub fn new() -> Self {
        Self {
            state: LlmSetupState::new(),
            download_handle: None,
            download_status: DownloadStatus::Idle,
            confirm_dialog: None,
            confirm_action: None,
        }
    }

    fn item_count() -> usize {
        4 // 3 options + back
    }

    fn model_installed() -> bool {
        download::model_files_present(&crate::textgen::default_model_dir())
    }

    fn activate_selected(&mut self) -> Option<ScreenTransition> {
        match self.state.selected {
            0 => self.toggle_llm(),
            1 => self.activate_download(),
            2 => self.activate_delete(),
            3 => Some(ScreenTransition::Pop),
            _ => None,
        }
    }

    fn toggle_llm(&mut self) -> Option<ScreenTransition> {
        let current = config::is_llm_enabled();
        let _ = config::persist_llm_preference(!current);
        Some(ScreenTransition::ReinitTextGen)
    }

    fn activate_download(&mut self) -> Option<ScreenTransition> {
        if Self::model_installed() {
            return None; // Already installed
        }
        if matches!(self.download_status, DownloadStatus::Downloading(_)) {
            return None; // Already downloading
        }

        self.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Download Model".to_string(),
            message: "Download SmolLM2-1.7B (~1.0 GB)?\nThis enables AI-generated text for city names,\nnewspaper articles, and advisor tips.".to_string(),
            selected: 0,
            buttons: vec![
                ConfirmDialogButtonViewModel {
                    label: "Download".to_string(),
                    role: ConfirmDialogButtonRole::Accept,
                },
                ConfirmDialogButtonViewModel {
                    label: "Cancel".to_string(),
                    role: ConfirmDialogButtonRole::Cancel,
                },
            ],
        });
        self.confirm_action = Some(ConfirmAction::DownloadModel);
        None
    }

    fn activate_delete(&mut self) -> Option<ScreenTransition> {
        if !Self::model_installed() {
            return None; // Nothing to delete
        }

        self.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Delete Model".to_string(),
            message: "Delete model files?\nText generation will use the static fallback.".to_string(),
            selected: 0,
            buttons: vec![
                ConfirmDialogButtonViewModel {
                    label: "Delete".to_string(),
                    role: ConfirmDialogButtonRole::Accept,
                },
                ConfirmDialogButtonViewModel {
                    label: "Cancel".to_string(),
                    role: ConfirmDialogButtonRole::Cancel,
                },
            ],
        });
        self.confirm_action = Some(ConfirmAction::DeleteModel);
        None
    }

    fn handle_confirm(&mut self) -> Option<ScreenTransition> {
        let dialog = self.confirm_dialog.as_ref()?;
        let role = dialog.selected_role()?;
        let action = self.confirm_action?;

        if role == ConfirmDialogButtonRole::Cancel {
            self.confirm_dialog = None;
            self.confirm_action = None;
            return None;
        }

        // Accept
        self.confirm_dialog = None;
        self.confirm_action = None;

        match action {
            ConfirmAction::DownloadModel => {
                let model_dir = crate::textgen::default_model_dir();
                if let Some(handle) = download::start_download(model_dir) {
                    self.download_handle = Some(handle);
                    self.download_status = DownloadStatus::Downloading("Starting...".to_string());
                } else {
                    self.download_status =
                        DownloadStatus::Failed("LLM feature not available".to_string());
                }
                None
            }
            ConfirmAction::DeleteModel => {
                let model_dir = crate::textgen::default_model_dir();
                match download::delete_model_files(&model_dir) {
                    Ok(()) => {
                        let _ = config::persist_llm_preference(false);
                        self.download_status = DownloadStatus::Idle;
                        Some(ScreenTransition::ReinitTextGen)
                    }
                    Err(e) => {
                        self.download_status = DownloadStatus::Failed(e);
                        None
                    }
                }
            }
        }
    }

    fn handle_dialog_action(&mut self, action: &Action) -> bool {
        let Some(dialog) = self.confirm_dialog.as_mut() else {
            return false;
        };

        match action {
            Action::MenuSelect => true, // handled in on_action via handle_confirm
            Action::MenuBack => {
                self.confirm_dialog = None;
                self.confirm_action = None;
                true
            }
            Action::MoveCursor(dx, _) => {
                let count = dialog.button_count();
                confirm_dialog::cycle_selection(&mut dialog.selected, count, *dx);
                true
            }
            Action::MouseClick { col, row } => {
                // Simple: check if click is on a button area — for now just consume
                let _ = (col, row);
                true
            }
            _ => true, // consume all input while dialog is open
        }
    }
}

impl Screen for LlmSetupScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_tick(&mut self, _context: AppContext) -> Option<ScreenTransition> {
        if let Some(handle) = &self.download_handle {
            if let Some(progress) = handle.poll() {
                match progress {
                    DownloadProgress::Downloading(what) => {
                        self.download_status = DownloadStatus::Downloading(what);
                    }
                    DownloadProgress::Done => {
                        self.download_status = DownloadStatus::Done;
                        self.download_handle = None;
                        // Enable LLM and reinit
                        let _ = config::persist_llm_preference(true);
                        return Some(ScreenTransition::ReinitTextGen);
                    }
                    DownloadProgress::Failed(err) => {
                        self.download_status = DownloadStatus::Failed(err);
                        self.download_handle = None;
                    }
                }
            }
        }
        None
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        // If confirm dialog is open, handle it first
        if self.confirm_dialog.is_some() {
            if matches!(action, Action::MenuSelect) {
                return self.handle_confirm();
            }
            self.handle_dialog_action(&action);
            return None;
        }

        let count = Self::item_count();
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(_, dy) => {
                if dy > 0 {
                    self.state.selected = (self.state.selected + 1) % count;
                } else if dy < 0 {
                    self.state.selected = self.state.selected.checked_sub(1).unwrap_or(count - 1);
                }
                None
            }
            Action::MouseClick { col, row } => {
                if let Some(idx) = self
                    .state
                    .row_areas
                    .iter()
                    .position(|area| area.contains(col, row))
                {
                    self.state.selected = idx;
                    return self.activate_selected();
                }
                None
            }
            Action::MenuSelect => self.activate_selected(),
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        let llm_enabled = config::is_llm_enabled();
        let model_installed = Self::model_installed();

        let download_progress = match &self.download_status {
            DownloadStatus::Downloading(what) => Some(what.clone()),
            _ => None,
        };
        let download_failed = match &self.download_status {
            DownloadStatus::Failed(err) => Some(err.clone()),
            _ => None,
        };

        crate::ui::view::ScreenView::LlmSetup(crate::ui::view::LlmSetupViewModel {
            llm_enabled,
            model_installed,
            download_progress,
            download_failed,
            selected: self.state.selected,
            confirm_dialog: self.confirm_dialog.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn test_context() -> (
        Arc<RwLock<crate::core::engine::SimulationEngine>>,
        crate::textgen::TextGenService,
    ) {
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(4, 4),
            crate::core::sim::SimState::default(),
        )));
        let textgen =
            crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));
        (engine, textgen)
    }

    #[test]
    fn new_screen_starts_at_first_item() {
        let screen = LlmSetupScreen::new();
        assert_eq!(screen.state.selected, 0);
        assert!(screen.confirm_dialog.is_none());
    }

    #[test]
    fn build_view_returns_llm_setup() {
        let screen = LlmSetupScreen::new();
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let view = screen.build_view(context);
        assert!(matches!(view, crate::ui::view::ScreenView::LlmSetup(_)));
    }

    #[test]
    fn back_action_pops() {
        let mut screen = LlmSetupScreen::new();
        screen.state.selected = 3;
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(matches!(result, Some(ScreenTransition::Pop)));
    }

    #[test]
    fn toggle_returns_reinit() {
        let mut screen = LlmSetupScreen::new();
        screen.state.selected = 0;
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(matches!(result, Some(ScreenTransition::ReinitTextGen)));
    }

    #[test]
    fn download_shows_confirm_dialog() {
        let mut screen = LlmSetupScreen::new();
        screen.state.selected = 1;
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(result.is_none());
        assert!(screen.confirm_dialog.is_some());
        assert!(matches!(screen.confirm_action, Some(ConfirmAction::DownloadModel)));
    }

    #[test]
    fn cancel_dialog_dismisses() {
        let mut screen = LlmSetupScreen::new();
        // Open dialog
        screen.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Test".to_string(),
            message: "Test".to_string(),
            selected: 1, // Cancel button
            buttons: vec![
                ConfirmDialogButtonViewModel {
                    label: "OK".to_string(),
                    role: ConfirmDialogButtonRole::Accept,
                },
                ConfirmDialogButtonViewModel {
                    label: "Cancel".to_string(),
                    role: ConfirmDialogButtonRole::Cancel,
                },
            ],
        });
        screen.confirm_action = Some(ConfirmAction::DownloadModel);

        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(result.is_none());
        assert!(screen.confirm_dialog.is_none());
    }
}
