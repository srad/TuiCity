use crate::{
    app::{config, input::Action, ClickArea},
    textgen::{
        download::{self, DownloadHandle, DownloadProgress, DownloadProgressSnapshot},
        models::{LlmExecutionMode, LlmModelId},
    },
    ui::view::{
        ConfirmDialogButtonRole, ConfirmDialogButtonViewModel, ConfirmDialogViewModel,
        DownloadProgressViewModel,
    },
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
    InProgress(DownloadProgressSnapshot),
    Cancelling(DownloadProgressSnapshot),
    Cancelled,
    Failed(String),
}

impl DownloadStatus {
    fn is_busy(&self) -> bool {
        matches!(self, Self::InProgress(_) | Self::Cancelling(_))
    }

    fn snapshot(&self) -> Option<&DownloadProgressSnapshot> {
        match self {
            Self::InProgress(snapshot) | Self::Cancelling(snapshot) => Some(snapshot),
            _ => None,
        }
    }

    fn is_cancelling(&self) -> bool {
        matches!(self, Self::Cancelling(_))
    }
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
    model_installed_override: Option<bool>,
}

impl LlmSetupScreen {
    pub fn new() -> Self {
        Self {
            state: LlmSetupState::new(),
            download_handle: None,
            download_status: DownloadStatus::Idle,
            confirm_dialog: None,
            confirm_action: None,
            model_installed_override: None,
        }
    }

    fn item_count() -> usize {
        7 // 6 options + back
    }

    fn selected_model(&self) -> LlmModelId {
        config::get_llm_model()
    }

    fn execution_mode(&self) -> LlmExecutionMode {
        config::get_llm_execution_mode()
    }

    fn model_installed(&self) -> bool {
        self.model_installed_override
            .unwrap_or_else(|| download::model_files_present(&crate::textgen::default_model_dir()))
    }

    fn clear_download_message(&mut self) {
        if !self.download_status.is_busy() {
            self.download_status = DownloadStatus::Idle;
        }
    }

    fn activate_selected(&mut self) -> Option<ScreenTransition> {
        match self.state.selected {
            0 => self.toggle_llm(),
            1 => self.cycle_model(1),
            2 => self.cycle_execution_mode(1),
            3 => self.activate_download(),
            4 => self.cancel_download(),
            5 => self.activate_delete(),
            6 => Some(ScreenTransition::Pop),
            _ => None,
        }
    }

    fn toggle_llm(&mut self) -> Option<ScreenTransition> {
        if self.download_status.is_busy() {
            return None;
        }
        let current = config::is_llm_enabled();
        let _ = config::persist_llm_preference(!current);
        self.clear_download_message();
        Some(ScreenTransition::ReinitTextGen)
    }

    fn cycle_model(&mut self, direction: i32) -> Option<ScreenTransition> {
        if self.download_status.is_busy() {
            return None;
        }
        let current = self.selected_model();
        let next = current.cycle(direction);
        if next == current {
            return None;
        }
        let _ = config::persist_llm_model_preference(next);
        self.clear_download_message();
        if config::is_llm_enabled() {
            Some(ScreenTransition::ReinitTextGen)
        } else {
            None
        }
    }

    fn cycle_execution_mode(&mut self, direction: i32) -> Option<ScreenTransition> {
        if self.download_status.is_busy() {
            return None;
        }
        let current = self.execution_mode();
        let next = current.cycle(direction);
        if next == current {
            return None;
        }
        let _ = config::persist_llm_execution_mode_preference(next);
        self.clear_download_message();
        if config::is_llm_enabled() {
            Some(ScreenTransition::ReinitTextGen)
        } else {
            None
        }
    }

    fn activate_download(&mut self) -> Option<ScreenTransition> {
        if self.model_installed() || self.download_status.is_busy() {
            return None;
        }

        let model = self.selected_model();
        self.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Download Model".to_string(),
            message: format!(
                "Download {} ({})?\nThis enables locally generated city names,\nnewspaper articles, and advisor tips.",
                model.label(),
                model.download_size_label()
            ),
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

    fn cancel_download(&mut self) -> Option<ScreenTransition> {
        let Some(handle) = &self.download_handle else {
            return None;
        };

        handle.cancel();
        let snapshot =
            self.download_status
                .snapshot()
                .cloned()
                .unwrap_or(DownloadProgressSnapshot {
                    label: "Canceling download...".to_string(),
                    downloaded_bytes: 0,
                    total_bytes: None,
                });
        self.download_status = DownloadStatus::Cancelling(snapshot);
        None
    }

    fn activate_delete(&mut self) -> Option<ScreenTransition> {
        if !self.model_installed() || self.download_status.is_busy() {
            return None;
        }

        self.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Delete Model".to_string(),
            message: format!(
                "Delete {} from disk?\nThe game will fall back to the built-in static text.",
                self.selected_model().label()
            ),
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

        self.confirm_dialog = None;
        self.confirm_action = None;

        match action {
            ConfirmAction::DownloadModel => {
                let model = self.selected_model();
                let model_dir = crate::textgen::default_model_dir();
                log::info!(
                    "[llm-setup] user confirmed download of {} into {}",
                    model.label(),
                    model_dir.display()
                );
                if let Some(handle) = download::start_download(model_dir, model) {
                    self.download_handle = Some(handle);
                    self.download_status = DownloadStatus::InProgress(DownloadProgressSnapshot {
                        label: format!("Preparing {}", model.label()),
                        downloaded_bytes: 0,
                        total_bytes: None,
                    });
                } else {
                    log::error!(
                        "[llm-setup] unable to start model download: llm feature unavailable or downloader thread failed"
                    );
                    self.download_status =
                        DownloadStatus::Failed("LLM feature not available".to_string());
                }
                None
            }
            ConfirmAction::DeleteModel => {
                let model_dir = crate::textgen::default_model_dir();
                log::info!(
                    "[llm-setup] user confirmed model deletion from {}",
                    model_dir.display()
                );
                match download::delete_model_files(&model_dir) {
                    Ok(()) => {
                        self.download_status = DownloadStatus::Idle;
                        Some(ScreenTransition::ReinitTextGen)
                    }
                    Err(error) => {
                        log::error!("[llm-setup] failed to delete model files: {error}");
                        self.download_status = DownloadStatus::Failed(error);
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
            Action::MenuSelect => true,
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
                let _ = (col, row);
                true
            }
            _ => true,
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
                    DownloadProgress::Progress(snapshot) => {
                        log::info!(
                            "[llm-setup] download progress: {} {}/{:?}",
                            snapshot.label,
                            snapshot.downloaded_bytes,
                            snapshot.total_bytes
                        );
                        self.download_status = if self.download_status.is_cancelling() {
                            DownloadStatus::Cancelling(snapshot)
                        } else {
                            DownloadStatus::InProgress(snapshot)
                        };
                    }
                    DownloadProgress::Done => {
                        log::info!("[llm-setup] model download completed successfully");
                        self.download_status = DownloadStatus::Idle;
                        self.download_handle = None;
                        let _ = config::persist_llm_preference(true);
                        return Some(ScreenTransition::ReinitTextGen);
                    }
                    DownloadProgress::Cancelled => {
                        log::info!("[llm-setup] model download canceled");
                        self.download_status = DownloadStatus::Cancelled;
                        self.download_handle = None;
                    }
                    DownloadProgress::Failed(error) => {
                        log::error!("[llm-setup] model download failed: {error}");
                        self.download_status = DownloadStatus::Failed(error);
                        self.download_handle = None;
                    }
                }
            }
        }
        None
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
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
            Action::MoveCursor(dx, dy) => {
                if dy > 0 {
                    self.state.selected = (self.state.selected + 1) % count;
                } else if dy < 0 {
                    self.state.selected = self.state.selected.checked_sub(1).unwrap_or(count - 1);
                } else if dx != 0 {
                    return match self.state.selected {
                        1 => self.cycle_model(dx),
                        2 => self.cycle_execution_mode(dx),
                        _ => None,
                    };
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

    fn build_view(&self, context: AppContext<'_>) -> crate::ui::view::ScreenView {
        let llm_enabled = config::is_llm_enabled();
        let selected_model = self.selected_model();
        let execution_mode = self.execution_mode();
        let model_installed = self.model_installed();
        let runtime_info = context.textgen.runtime_info();

        let is_cancelling = self.download_status.is_cancelling();
        let download_progress = self.download_status.snapshot().map(|snapshot| {
            let percent = snapshot.total_bytes.and_then(|total| {
                if total == 0 {
                    None
                } else {
                    Some(((snapshot.downloaded_bytes.saturating_mul(100)) / total).min(100) as u8)
                }
            });
            DownloadProgressViewModel {
                label: snapshot.label.clone(),
                downloaded_bytes: snapshot.downloaded_bytes,
                total_bytes: snapshot.total_bytes,
                percent,
                cancelling: is_cancelling,
            }
        });

        let download_notice = match &self.download_status {
            DownloadStatus::Cancelled => {
                Some("Download canceled. Partial files were cleaned up.".to_string())
            }
            _ => None,
        };
        let download_failed = match &self.download_status {
            DownloadStatus::Failed(error) => Some(error.clone()),
            _ => None,
        };

        crate::ui::view::ScreenView::LlmSetup(crate::ui::view::LlmSetupViewModel {
            llm_enabled,
            model_installed,
            selected_model_label: selected_model.label().to_string(),
            selected_model_description: selected_model.description().to_string(),
            selected_model_size_label: selected_model.download_size_label().to_string(),
            gpu_mode_label: execution_mode.label().to_string(),
            gpu_mode_description: execution_mode.description().to_string(),
            backend_status: runtime_info.status_line,
            gpu_status: runtime_info.acceleration_line,
            download_progress,
            download_notice,
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
        screen.state.selected = 6;
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
        screen.model_installed_override = Some(false);
        screen.state.selected = 3;
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(result.is_none());
        assert!(screen.confirm_dialog.is_some());
        assert!(matches!(
            screen.confirm_action,
            Some(ConfirmAction::DownloadModel)
        ));
    }

    #[test]
    fn cancel_dialog_dismisses() {
        let mut screen = LlmSetupScreen::new();
        screen.confirm_dialog = Some(ConfirmDialogViewModel {
            title: "Test".to_string(),
            message: "Test".to_string(),
            selected: 1,
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

    #[test]
    fn cancel_download_switches_to_cancelling_state() {
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut screen = LlmSetupScreen::new();
        screen.download_handle = Some(DownloadHandle::for_test(
            rx,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        ));
        screen.download_status = DownloadStatus::InProgress(DownloadProgressSnapshot {
            label: "Downloading Gemma".to_string(),
            downloaded_bytes: 10,
            total_bytes: Some(100),
        });

        screen.cancel_download();

        assert!(matches!(
            screen.download_status,
            DownloadStatus::Cancelling(_)
        ));
    }
}
