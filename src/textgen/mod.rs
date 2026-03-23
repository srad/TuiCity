pub mod backend_static;
pub mod context;
pub mod download;
pub mod generator;
pub mod models;
pub mod prompt;
pub mod types;

#[cfg(feature = "llm")]
pub mod backend_llamacpp;

#[cfg(test)]
mod integration_tests;

use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};

use generator::TextGenerator;
use models::{compiled_gpu_backend_name, LlmExecutionMode, LlmModelId};
use types::{LlmResponse, LlmTask};

const REQUIRED_NEWSPAPER_PAGES: [usize; 4] = [1, 2, 3, 4];
const REQUIRED_NEWSPAPER_SECTIONS: [&str; 12] = [
    "LEAD STORY",
    "CITY BEAT",
    "CITY OWNER'S ADVERTISEMENT",
    "LETTERS FROM READERS",
    "SIDEWALK QUOTE",
    "EDITORIAL",
    "SHOPKEEPER SPOTLIGHT",
    "CONTACT ADS",
    "CLASSIFIEDS",
    "JOKE CORNER",
    "COMMUNITY CALENDAR",
    "WEATHER DESK",
];

/// Handle to the background text generation worker. All methods are non-blocking.
pub struct TextGenService {
    task_tx: mpsc::Sender<LlmTask>,
    response_rx: mpsc::Receiver<LlmResponse>,
    runtime_info: Arc<RwLock<TextGenRuntimeInfo>>,
}

#[derive(Clone, Debug)]
pub struct TextGenRuntimeInfo {
    pub backend_name: String,
    pub status_line: String,
    pub acceleration_line: String,
}

impl TextGenRuntimeInfo {
    fn initializing(model: LlmModelId, execution_mode: LlmExecutionMode) -> Self {
        Self {
            backend_name: "initializing".to_string(),
            status_line: format!("Preparing {}...", model.label()),
            acceleration_line: execution_mode.selection_hint().to_string(),
        }
    }
}

struct BackendSelection {
    generator: Box<dyn TextGenerator>,
    runtime_info: TextGenRuntimeInfo,
}

impl TextGenService {
    /// Spawn the text generation worker thread. Always succeeds — falls back to the
    /// static generator if no model backend is available.
    ///
    /// Loading happens asynchronously, so `backend_name()` will return "initializing"
    /// until the model is ready.
    pub fn start(model_dir: PathBuf) -> Self {
        let selected_model = crate::app::config::get_llm_model();
        let execution_mode = crate::app::config::get_llm_execution_mode();
        let runtime_info = Arc::new(RwLock::new(TextGenRuntimeInfo::initializing(
            selected_model,
            execution_mode,
        )));
        let info_clone = runtime_info.clone();

        let (task_tx, task_rx) = mpsc::channel::<LlmTask>();
        let (response_tx, response_rx) = mpsc::channel::<LlmResponse>();

        std::thread::Builder::new()
            .name("textgen-worker".to_string())
            .spawn(move || {
                let selection = select_backend(&model_dir, selected_model, execution_mode);
                let name = selection.generator.backend_name().to_string();
                log::info!("[textgen] using backend: {name}");

                if let Ok(mut lock) = info_clone.write() {
                    *lock = selection.runtime_info.clone();
                }

                worker_loop(selection.generator, task_rx, response_tx);
            })
            .expect("failed to spawn textgen worker thread");

        Self {
            task_tx,
            response_rx,
            runtime_info,
        }
    }

    pub fn reinitializing() -> Self {
        let (task_tx, _task_rx) = mpsc::channel::<LlmTask>();
        let (_response_tx, response_rx) = mpsc::channel::<LlmResponse>();
        Self {
            task_tx,
            response_rx,
            runtime_info: Arc::new(RwLock::new(TextGenRuntimeInfo {
                backend_name: "initializing".to_string(),
                status_line: "Reinitializing text generation...".to_string(),
                acceleration_line: String::new(),
            })),
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
    #[cfg(test)]
    pub fn backend_name(&self) -> String {
        self.runtime_info.read().unwrap().backend_name.clone()
    }

    /// Returns true if using a real model backend (not the static fallback).
    pub fn has_model(&self) -> bool {
        let name = self.runtime_info.read().unwrap();
        let name = &name.backend_name;
        *name != "static" && *name != "initializing"
    }

    pub fn runtime_info(&self) -> TextGenRuntimeInfo {
        self.runtime_info.read().unwrap().clone()
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
        let text = generate_with_fallback(
            &mut *generator,
            &task,
            &prompt_text,
            max_tokens,
            temperature,
        );

        let _ = response_tx.send(LlmResponse {
            task_tag: tag,
            text,
        });
    }
}

/// Select the best available backend. Priority:
/// 1. llama-cpp (if `llm` feature + model files present + load succeeds)
/// 2. Static generator (always available)
fn select_backend(
    model_dir: &std::path::Path,
    selected_model: LlmModelId,
    execution_mode: LlmExecutionMode,
) -> BackendSelection {
    if !crate::app::config::is_llm_enabled() {
        log::info!(
            "[textgen] text generation is disabled; using static fallback for {}",
            selected_model.label()
        );
        return BackendSelection {
            generator: Box::new(backend_static::StaticGenerator::new()),
            runtime_info: TextGenRuntimeInfo {
                backend_name: "static".to_string(),
                status_line: "Text generation is off — using safe fallback text.".to_string(),
                acceleration_line: execution_mode.selection_hint().to_string(),
            },
        };
    }

    #[cfg(feature = "llm")]
    if download::model_files_present(model_dir) {
        log::info!(
            "[textgen] compiled GPU backend: {}",
            compiled_gpu_backend_name()
        );
        log::info!("[textgen] loading model from {}", model_dir.display());
        match backend_llamacpp::LlamaCppGenerator::load(
            model_dir,
            selected_model.spec().prompt_style,
            execution_mode,
        ) {
            Ok(load) => {
                log::info!("[textgen] model loaded successfully");
                log::info!(
                    "[textgen] GPU available: {}; execution mode: {}; GPU active: {}",
                    load.gpu_available,
                    execution_mode.label(),
                    load.gpu_active
                );
                return BackendSelection {
                    generator: Box::new(load.generator),
                    runtime_info: TextGenRuntimeInfo {
                        backend_name: "llama.cpp".to_string(),
                        status_line: format!("Active — {}", selected_model.label()),
                        acceleration_line: load.acceleration_summary,
                    },
                };
            }
            Err(e) => {
                log::error!("[textgen] failed to load model: {e}");
                return BackendSelection {
                    generator: Box::new(backend_static::StaticGenerator::new()),
                    runtime_info: TextGenRuntimeInfo {
                        backend_name: "static".to_string(),
                        status_line: format!(
                            "{} failed to load — using fallback text.",
                            selected_model.short_label()
                        ),
                        acceleration_line: execution_mode.selection_hint().to_string(),
                    },
                };
            }
        }
    }

    #[cfg(not(feature = "llm"))]
    {
        let _ = model_dir;
        log::info!("[textgen] local model support is not compiled into this build");
        return BackendSelection {
            generator: Box::new(backend_static::StaticGenerator::new()),
            runtime_info: TextGenRuntimeInfo {
                backend_name: "static".to_string(),
                status_line: "This build does not include local AI model support.".to_string(),
                acceleration_line: "GPU settings are unavailable in this build.".to_string(),
            },
        };
    }

    #[cfg(feature = "llm")]
    {
        log::info!(
            "[textgen] selected model {} is not installed at {}",
            selected_model.label(),
            model_dir.display()
        );
        BackendSelection {
            generator: Box::new(backend_static::StaticGenerator::new()),
            runtime_info: TextGenRuntimeInfo {
                backend_name: "static".to_string(),
                status_line: format!("{} is not downloaded yet.", selected_model.label()),
                acceleration_line: execution_mode.selection_hint().to_string(),
            },
        }
    }
}

/// Map each task to its prompt string, max token count, and temperature.
fn build_inference_params(task: &LlmTask) -> (String, usize, f32) {
    match task {
        LlmTask::GenerateCityName => (prompt::city_name_prompt(), 3, 1.0),
        LlmTask::WriteNewspaper { context } => (prompt::newspaper_prompt(context), 120, 0.75),
        LlmTask::AdvisorAdvice { context, domain } => {
            (prompt::advisor_prompt(context, domain), 250, 0.6)
        }
        LlmTask::GenerateAlert {
            context,
            alert_kind,
        } => (prompt::alert_prompt(context, alert_kind), 60, 0.8),
        LlmTask::WriteNewspaperArticle { context } => {
            (prompt::newspaper_article_prompt(context), 860, 0.85)
        }
    }
}

fn generate_with_fallback(
    generator: &mut dyn TextGenerator,
    task: &LlmTask,
    prompt_text: &str,
    max_tokens: usize,
    temperature: f32,
) -> String {
    match generator.generate(prompt_text, max_tokens, temperature) {
        Ok(text) => sanitize_generated_text(task, text, prompt_text, max_tokens, temperature),
        Err(e) => {
            log::error!("[textgen] generation failed for {:?}: {e}", task.tag());
            static_fallback_text(task, prompt_text, max_tokens, temperature)
        }
    }
}

fn sanitize_generated_text(
    task: &LlmTask,
    text: String,
    prompt_text: &str,
    max_tokens: usize,
    temperature: f32,
) -> String {
    let trimmed = text.trim();
    let valid = match task {
        LlmTask::WriteNewspaper { .. } => is_valid_newspaper_headlines(trimmed),
        LlmTask::WriteNewspaperArticle { .. } => is_valid_newspaper_article(trimmed),
        _ => !trimmed.is_empty(),
    };

    if valid {
        trimmed.to_string()
    } else {
        log::warn!(
            "[textgen] invalid {:?} output from backend; using static fallback",
            task.tag()
        );
        static_fallback_text(task, prompt_text, max_tokens, temperature)
    }
}

fn static_fallback_text(
    task: &LlmTask,
    prompt_text: &str,
    max_tokens: usize,
    temperature: f32,
) -> String {
    let mut fallback = backend_static::StaticGenerator::new();
    match fallback.generate(prompt_text, max_tokens, temperature) {
        Ok(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                log::error!(
                    "[textgen] static fallback produced empty output for {:?}",
                    task.tag()
                );
            }
            trimmed.to_string()
        }
        Err(e) => {
            log::error!("[textgen] static fallback failed for {:?}: {e}", task.tag());
            String::new()
        }
    }
}

fn normalize_header_line(line: &str) -> String {
    line.trim()
        .trim_start_matches(|c: char| matches!(c, '#' | '*' | '-' | '>' | ' '))
        .trim_end_matches(|c: char| matches!(c, '*' | ':' | '-' | '—' | '–' | ' '))
        .to_ascii_uppercase()
}

fn parse_newspaper_page_number(line: &str) -> Option<usize> {
    let normalized = normalize_header_line(line);
    if !normalized.starts_with("PAGE ") {
        return None;
    }
    let digits = normalized
        .chars()
        .skip(5)
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn parse_newspaper_section_title(line: &str) -> Option<String> {
    let normalized = normalize_header_line(line);
    if !normalized.starts_with("SECTION") {
        return None;
    }
    let title = normalized
        .get(7..)
        .unwrap_or("")
        .trim_start_matches(|c: char| matches!(c, ':' | '-' | '—' | '–' | ' '))
        .trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

fn looks_like_code(text: &str) -> bool {
    if text.contains("```") {
        return true;
    }

    let markers = text
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("def ")
                || line.starts_with("class ")
                || line.starts_with("import ")
                || line.starts_with("from ")
                || line.starts_with("print(")
                || line.starts_with("if __name__")
                || line.starts_with("for ")
                || line.starts_with("while ")
                || line.starts_with("return ")
                || line.contains("self.")
        })
        .count();

    markers >= 2
}

fn is_valid_newspaper_headlines(text: &str) -> bool {
    if text.is_empty() || looks_like_code(text) {
        return false;
    }

    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    lines.len() >= 3
        && lines.len() <= 6
        && lines
            .iter()
            .all(|line| line.len() >= 12 && line.len() <= 90 && !line.contains("LEAD STORY"))
}

fn is_valid_newspaper_article(text: &str) -> bool {
    if text.is_empty() || looks_like_code(text) {
        return false;
    }

    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return false;
    }

    let page_positions = REQUIRED_NEWSPAPER_PAGES
        .iter()
        .map(|page| {
            lines
                .iter()
                .position(|line| parse_newspaper_page_number(line) == Some(*page))
        })
        .collect::<Option<Vec<_>>>();
    let Some(page_positions) = page_positions else {
        return false;
    };
    if !page_positions
        .windows(2)
        .all(|window| window[0] < window[1])
    {
        return false;
    }

    let section_titles = lines
        .iter()
        .filter_map(|line| parse_newspaper_section_title(line))
        .collect::<Vec<_>>();
    if section_titles.len() < REQUIRED_NEWSPAPER_SECTIONS.len() {
        return false;
    }
    if !REQUIRED_NEWSPAPER_SECTIONS
        .iter()
        .all(|required| section_titles.iter().any(|title| title == required))
    {
        return false;
    }

    let non_marker_lines = lines
        .iter()
        .filter(|line| {
            parse_newspaper_page_number(line).is_none()
                && parse_newspaper_section_title(line).is_none()
        })
        .collect::<Vec<_>>();
    let total_body_len = non_marker_lines
        .iter()
        .map(|line| line.len())
        .sum::<usize>();

    total_body_len >= 420 && non_marker_lines.len() >= 18
}

pub(crate) fn sanitize_newspaper_article_text(text: &str, context: &types::CityContext) -> String {
    let task = LlmTask::WriteNewspaperArticle {
        context: context.clone(),
    };
    let (prompt_text, max_tokens, temperature) = build_inference_params(&task);
    sanitize_generated_text(
        &task,
        text.to_string(),
        &prompt_text,
        max_tokens,
        temperature,
    )
}

/// Returns the root model directory path: `~/.tuicity2000/models/`
pub fn models_root_dir() -> PathBuf {
    let base = dirs_next_or_home();
    base.join(".tuicity2000").join("models")
}

pub fn model_dir_for(model: LlmModelId) -> PathBuf {
    models_root_dir().join(model.spec().directory_name)
}

/// Returns the configured model directory path.
pub fn default_model_dir() -> PathBuf {
    model_dir_for(crate::app::config::get_llm_model())
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
    use crate::textgen::types::sample_context;

    fn assert_multi_page_newspaper_article(text: &str) {
        assert!(is_valid_newspaper_article(text));
        for page in REQUIRED_NEWSPAPER_PAGES {
            assert!(
                text.contains(&format!("PAGE {page}:")),
                "missing PAGE {page} marker"
            );
        }
        for section in REQUIRED_NEWSPAPER_SECTIONS {
            assert!(
                text.contains(&format!("SECTION: {section}")),
                "missing SECTION: {section}"
            );
        }
    }

    struct FailingGenerator;

    impl TextGenerator for FailingGenerator {
        fn generate(
            &mut self,
            _prompt: &str,
            _max_tokens: usize,
            _temperature: f32,
        ) -> Result<String, String> {
            Err("boom".to_string())
        }

        fn backend_name(&self) -> &str {
            "failing"
        }
    }

    struct PythonGenerator;

    impl TextGenerator for PythonGenerator {
        fn generate(
            &mut self,
            _prompt: &str,
            _max_tokens: usize,
            _temperature: f32,
        ) -> Result<String, String> {
            Ok(
                "import sys\n\ndef main():\n    print('hello world')\n\nif __name__ == '__main__':\n    main()\n"
                    .to_string(),
            )
        }

        fn backend_name(&self) -> &str {
            "pythonish"
        }
    }

    #[test]
    fn default_model_dir_uses_selected_model_subdirectory() {
        let dir = default_model_dir();
        let parent = dir
            .parent()
            .expect("selected model dir should have a parent");
        assert!(parent.ends_with("models"));
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

    #[test]
    fn invalid_newspaper_article_is_detected() {
        assert!(!is_valid_newspaper_article(""));
        assert!(!is_valid_newspaper_article(
            "import os\n\ndef run():\n    print('oops')\n"
        ));
        let text = backend_static::StaticGenerator::new()
            .generate(
                &prompt::newspaper_article_prompt(&sample_context()),
                720,
                0.85,
            )
            .unwrap();
        assert_multi_page_newspaper_article(&text);
    }

    #[test]
    fn newspaper_article_falls_back_when_generation_fails() {
        let task = LlmTask::WriteNewspaperArticle {
            context: sample_context(),
        };
        let (prompt_text, max_tokens, temperature) = build_inference_params(&task);
        let mut generator = FailingGenerator;
        let text =
            generate_with_fallback(&mut generator, &task, &prompt_text, max_tokens, temperature);
        assert_multi_page_newspaper_article(&text);
    }

    #[test]
    fn newspaper_article_falls_back_when_output_looks_like_code() {
        let task = LlmTask::WriteNewspaperArticle {
            context: sample_context(),
        };
        let (prompt_text, max_tokens, temperature) = build_inference_params(&task);
        let mut generator = PythonGenerator;
        let text =
            generate_with_fallback(&mut generator, &task, &prompt_text, max_tokens, temperature);
        assert_multi_page_newspaper_article(&text);
        assert!(!text.contains("import sys"));
    }
}
