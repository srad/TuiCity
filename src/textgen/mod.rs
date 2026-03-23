pub mod backend_static;
pub mod context;
pub mod download;
pub mod generator;
pub mod prompt;
pub mod types;

#[cfg(feature = "llm")]
pub mod backend_llamacpp;

#[cfg(test)]
mod integration_tests;

use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};

use generator::TextGenerator;
use types::{LlmResponse, LlmTask};

/// Handle to the background text generation worker. All methods are non-blocking.
pub struct TextGenService {
    task_tx: mpsc::Sender<LlmTask>,
    response_rx: mpsc::Receiver<LlmResponse>,
    backend_name: Arc<RwLock<String>>,
}

impl TextGenService {
    /// Spawn the text generation worker thread. Always succeeds — falls back to the
    /// static generator if no model backend is available.
    ///
    /// Loading happens asynchronously, so `backend_name()` will return "initializing"
    /// until the model is ready.
    pub fn start(model_dir: PathBuf) -> Self {
        let backend_name = Arc::new(RwLock::new("initializing".to_string()));
        let name_clone = backend_name.clone();

        let (task_tx, task_rx) = mpsc::channel::<LlmTask>();
        let (response_tx, response_rx) = mpsc::channel::<LlmResponse>();

        std::thread::Builder::new()
            .name("textgen-worker".to_string())
            .spawn(move || {
                let generator = select_backend(&model_dir);
                let name = generator.backend_name().to_string();
                log::info!("[textgen] using backend: {name}");

                if let Ok(mut lock) = name_clone.write() {
                    *lock = name;
                }

                worker_loop(generator, task_rx, response_tx);
            })
            .expect("failed to spawn textgen worker thread");

        Self {
            task_tx,
            response_rx,
            backend_name,
        }
    }

    /// Submit a task to the worker (non-blocking). Silently drops if worker is gone.
    pub fn request(&self, task: LlmTask) {
        let _ = self.task_tx.send(task);
    }

    /// Poll for a completed response (non-blocking). Returns `None` if nothing ready.
    pub fn poll(&self) -> Option<LlmResponse> {
        self.response_rx.try_recv().ok()
    }

    /// Returns the name of the active backend (e.g. "llama.cpp", "static").
    pub fn backend_name(&self) -> String {
        self.backend_name.read().unwrap().clone()
    }

    /// Returns true if using a real model backend (not the static fallback).
    pub fn has_model(&self) -> bool {
        let name = self.backend_name.read().unwrap();
        *name != "static" && *name != "initializing"
    }
}

fn worker_loop(
    mut generator: Box<dyn TextGenerator>,
    task_rx: mpsc::Receiver<LlmTask>,
    response_tx: mpsc::Sender<LlmResponse>,
) {
    while let Ok(task) = task_rx.recv() {
        let tag = task.tag();
        let (prompt_text, max_tokens, temperature) = build_inference_params(&task);

        let text = match generator.generate(&prompt_text, max_tokens, temperature) {
            Ok(text) => text,
            Err(e) => {
                log::error!("[textgen] generation failed for {tag:?}: {e}");
                String::new()
            }
        };

        let _ = response_tx.send(LlmResponse {
            task_tag: tag,
            text,
        });
    }
}

/// Select the best available backend. Priority:
/// 1. llama-cpp (if `llm` feature + model files present + load succeeds)
/// 2. Static generator (always available)
fn select_backend(model_dir: &std::path::Path) -> Box<dyn TextGenerator> {
    #[cfg(feature = "llm")]
    if crate::app::config::is_llm_enabled() && download::model_files_present(model_dir) {
        log::info!("[textgen] loading model from {}", model_dir.display());
        match backend_llamacpp::LlamaCppGenerator::load(model_dir) {
            Ok(g) => {
                log::info!("[textgen] model loaded successfully");
                return Box::new(g);
            }
            Err(e) => {
                log::error!("[textgen] failed to load model: {e}");
            }
        }
    }

    #[cfg(not(feature = "llm"))]
    let _ = model_dir;

    log::info!("[textgen] using static text generator (no model)");
    Box::new(backend_static::StaticGenerator::new())
}

/// Map each task to its prompt string, max token count, and temperature.
fn build_inference_params(task: &LlmTask) -> (String, usize, f32) {
    match task {
        LlmTask::GenerateCityName => (prompt::city_name_prompt(), 3, 1.0),
        LlmTask::WriteNewspaper { context } => (prompt::newspaper_prompt(context), 300, 0.7),
        LlmTask::AdvisorAdvice { context, domain } => {
            (prompt::advisor_prompt(context, domain), 250, 0.6)
        }
        LlmTask::GenerateAlert {
            context,
            alert_kind,
        } => (prompt::alert_prompt(context, alert_kind), 60, 0.8),
        LlmTask::WriteNewspaperArticle { context } => {
            (prompt::newspaper_article_prompt(context), 300, 0.7)
        }
    }
}

/// Returns the default model directory path: `~/.tuicity2000/models/`
pub fn default_model_dir() -> PathBuf {
    let base = dirs_next_or_home();
    base.join(".tuicity2000").join("models")
}

fn dirs_next_or_home() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        PathBuf::from(home)
    } else {
        PathBuf::from(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_dir_ends_with_models() {
        let dir = default_model_dir();
        assert!(dir.ends_with("models"));
        assert!(dir.to_string_lossy().contains(".tuicity2000"));
    }

    #[test]
    fn service_starts_with_static_backend() {
        // Without model files, should start initializing then fall back to static.
        let service = TextGenService::start(PathBuf::from("/nonexistent/path/to/model"));

        let start = std::time::Instant::now();
        while service.backend_name() == "initializing" {
            if start.elapsed() > std::time::Duration::from_secs(2) {
                panic!("timeout waiting for backend init");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(service.backend_name(), "static");
        assert!(!service.has_model());
    }

    #[test]
    fn dirs_next_or_home_returns_something() {
        let home = dirs_next_or_home();
        assert!(!home.as_os_str().is_empty());
    }

    #[test]
    fn static_fallback_generates_city_name() {
        let service = TextGenService::start(PathBuf::from("/nonexistent"));
        service.request(LlmTask::GenerateCityName);

        let start = std::time::Instant::now();
        loop {
            if let Some(resp) = service.poll() {
                assert_eq!(resp.task_tag, types::LlmTaskTag::CityName);
                assert!(!resp.text.is_empty());
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(2) {
                panic!("timeout waiting for response");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
