/// Background model download using hf-hub.
///
/// Downloads the selected quantized GGUF model from HuggingFace Hub into a per-model
/// directory under `~/.tuicity2000/models/<model-id>/`.
use crate::textgen::models::LlmModelId;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{mpsc, Arc};

const LOCAL_MODEL_FILE: &str = "model.gguf";
const TEMP_MODEL_FILE: &str = "model.gguf.part";
const TEMP_CACHE_DIR: &str = ".hf-download-cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgressSnapshot {
    pub label: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

/// Progress updates sent from the download thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadProgress {
    /// The active transfer or finalize stage.
    Progress(DownloadProgressSnapshot),
    /// All files downloaded successfully.
    Done,
    /// Download was canceled and temporary files were cleaned up.
    Cancelled,
    /// Download failed with an error message.
    Failed(String),
}

/// Receiver handle for polling download progress from the UI thread.
pub struct DownloadHandle {
    rx: mpsc::Receiver<DownloadProgress>,
    cancel_requested: Arc<AtomicBool>,
}

impl DownloadHandle {
    /// Non-blocking poll for the latest progress update.
    pub fn poll(&self) -> Option<DownloadProgress> {
        let mut last = None;
        while let Ok(msg) = self.rx.try_recv() {
            last = Some(msg);
        }
        last
    }

    pub fn cancel(&self) {
        self.cancel_requested.store(true, SeqCst);
    }
}

#[cfg(test)]
impl DownloadHandle {
    pub(crate) fn for_test(
        rx: mpsc::Receiver<DownloadProgress>,
        cancel_requested: Arc<AtomicBool>,
    ) -> Self {
        Self {
            rx,
            cancel_requested,
        }
    }
}

/// Spawn a background thread that downloads the selected model into `model_dir`.
/// Returns a handle to poll progress, or `None` if the `llm` feature is not compiled.
pub fn start_download(model_dir: PathBuf, model: LlmModelId) -> Option<DownloadHandle> {
    #[cfg(not(feature = "llm"))]
    {
        log::warn!(
            "[llm-download] download requested for {}, but the llm feature is disabled",
            model_dir.display()
        );
        let _ = (model_dir, model);
        None
    }

    #[cfg(feature = "llm")]
    {
        let (tx, rx) = mpsc::channel();
        let cancel_requested = Arc::new(AtomicBool::new(false));
        log::info!(
            "[llm-download] starting background download of {} into {}",
            model.label(),
            model_dir.display()
        );

        let thread_model_dir = model_dir.clone();
        let thread_cancel = cancel_requested.clone();
        match std::thread::Builder::new()
            .name(format!("llm-download-{}", model.spec().directory_name))
            .spawn(move || {
                download_inner(model_dir, model, tx, thread_cancel);
            }) {
            Ok(_) => Some(DownloadHandle {
                rx,
                cancel_requested,
            }),
            Err(e) => {
                log::error!(
                    "[llm-download] failed to spawn download thread for {}: {e}",
                    thread_model_dir.display()
                );
                None
            }
        }
    }
}

#[cfg(feature = "llm")]
#[derive(Debug)]
enum DownloadFailure {
    Cancelled,
    Failed(String),
}

#[cfg(feature = "llm")]
fn download_inner(
    model_dir: PathBuf,
    model: LlmModelId,
    tx: mpsc::Sender<DownloadProgress>,
    cancel_requested: Arc<AtomicBool>,
) {
    log::info!(
        "[llm-download] preparing download of {} into {}",
        model.label(),
        model_dir.display()
    );

    if let Err(e) = std::fs::create_dir_all(&model_dir) {
        log::error!(
            "[llm-download] cannot create model directory {}: {e}",
            model_dir.display()
        );
        let _ = tx.send(DownloadProgress::Failed(format!(
            "cannot create {}: {e}",
            model_dir.display()
        )));
        return;
    }

    if model_files_present(&model_dir) {
        log::info!(
            "[llm-download] model files already present for {} in {}",
            model.label(),
            model_dir.display()
        );
        let _ = tx.send(DownloadProgress::Done);
        return;
    }

    if let Err(e) = cleanup_partial_artifacts(&model_dir) {
        log::error!(
            "[llm-download] failed to clean partial artifacts in {}: {e}",
            model_dir.display()
        );
        let _ = tx.send(DownloadProgress::Failed(format!(
            "cleanup before download: {e}"
        )));
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&model_dir) {
        log::error!(
            "[llm-download] failed to recreate model directory {} after cleanup: {e}",
            model_dir.display()
        );
        let _ = tx.send(DownloadProgress::Failed(format!(
            "recreate model directory after cleanup: {e}"
        )));
        return;
    }

    let result = download_model_file(&model_dir, model, &tx, &cancel_requested);

    match result {
        Ok(()) => {
            if let Err(e) = remove_cache_dir(&model_dir) {
                log::warn!(
                    "[llm-download] model download finished, but cache cleanup failed in {}: {e}",
                    model_dir.display()
                );
            }
            log::info!(
                "[llm-download] model download completed successfully for {} in {}",
                model.label(),
                model_dir.display()
            );
            let _ = tx.send(DownloadProgress::Done);
        }
        Err(DownloadFailure::Cancelled) => match cleanup_partial_artifacts(&model_dir) {
            Ok(()) => {
                log::info!(
                    "[llm-download] download canceled for {}; partial files removed",
                    model.label()
                );
                let _ = tx.send(DownloadProgress::Cancelled);
            }
            Err(cleanup_error) => {
                log::error!(
                    "[llm-download] download canceled for {}, but cleanup failed: {cleanup_error}",
                    model.label()
                );
                let _ = tx.send(DownloadProgress::Failed(format!(
                    "download canceled, but cleanup failed: {cleanup_error}"
                )));
            }
        },
        Err(DownloadFailure::Failed(error)) => match cleanup_partial_artifacts(&model_dir) {
            Ok(()) => {
                log::error!(
                    "[llm-download] model download failed for {}: {error}",
                    model.label()
                );
                let _ = tx.send(DownloadProgress::Failed(error));
            }
            Err(cleanup_error) => {
                log::error!(
                    "[llm-download] model download failed for {}, and cleanup failed: {cleanup_error}",
                    model.label()
                );
                let _ = tx.send(DownloadProgress::Failed(format!(
                    "{error} (cleanup failed: {cleanup_error})"
                )));
            }
        },
    }
}

#[cfg(feature = "llm")]
fn download_model_file(
    model_dir: &Path,
    model: LlmModelId,
    tx: &mpsc::Sender<DownloadProgress>,
    cancel_requested: &Arc<AtomicBool>,
) -> Result<(), DownloadFailure> {
    use hf_hub::api::sync::ApiBuilder;
    use hf_hub::api::Progress;

    #[derive(Debug)]
    struct CancelSignal;

    struct ChannelProgress {
        tx: mpsc::Sender<DownloadProgress>,
        cancel_requested: Arc<AtomicBool>,
        label: String,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    }

    impl ChannelProgress {
        fn maybe_cancel(&self) {
            if self.cancel_requested.load(SeqCst) {
                std::panic::panic_any(CancelSignal);
            }
        }

        fn send_update(&self) {
            let _ = self
                .tx
                .send(DownloadProgress::Progress(DownloadProgressSnapshot {
                    label: self.label.clone(),
                    downloaded_bytes: self.downloaded_bytes,
                    total_bytes: self.total_bytes,
                }));
        }
    }

    impl Progress for ChannelProgress {
        fn init(&mut self, size: usize, _filename: &str) {
            self.total_bytes = Some(size as u64);
            self.downloaded_bytes = 0;
            self.send_update();
            self.maybe_cancel();
        }

        fn update(&mut self, size: usize) {
            self.downloaded_bytes = self.downloaded_bytes.saturating_add(size as u64);
            self.send_update();
            self.maybe_cancel();
        }

        fn finish(&mut self) {
            self.send_update();
        }
    }

    if cancel_requested.load(SeqCst) {
        return Err(DownloadFailure::Cancelled);
    }

    let cache_dir = model_dir.join(TEMP_CACHE_DIR);
    let api = ApiBuilder::new()
        .with_progress(false)
        .with_cache_dir(cache_dir.clone())
        .build()
        .map_err(|e| DownloadFailure::Failed(format!("hf-hub init: {e}")))?;

    let spec = model.spec();
    let progress = ChannelProgress {
        tx: tx.clone(),
        cancel_requested: cancel_requested.clone(),
        label: format!("Downloading {}", model.label()),
        downloaded_bytes: 0,
        total_bytes: None,
    };

    let cached_path = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        api.model(spec.repo_id.to_string())
            .download_with_progress(spec.remote_file, progress)
    })) {
        Ok(Ok(path)) => path,
        Ok(Err(e)) => {
            return Err(DownloadFailure::Failed(format!(
                "download {}: {e}",
                spec.remote_file
            )));
        }
        Err(payload) => {
            if payload.is::<CancelSignal>() {
                return Err(DownloadFailure::Cancelled);
            }
            return Err(DownloadFailure::Failed(
                "download worker panicked".to_string(),
            ));
        }
    };

    if cancel_requested.load(SeqCst) {
        return Err(DownloadFailure::Cancelled);
    }

    let _ = tx.send(DownloadProgress::Progress(DownloadProgressSnapshot {
        label: "Finalizing model files...".to_string(),
        downloaded_bytes: 0,
        total_bytes: None,
    }));

    let temp_path = model_dir.join(TEMP_MODEL_FILE);
    std::fs::copy(&cached_path, &temp_path)
        .map_err(|e| DownloadFailure::Failed(format!("copy model file: {e}")))?;

    if cancel_requested.load(SeqCst) {
        return Err(DownloadFailure::Cancelled);
    }

    std::fs::rename(&temp_path, model_dir.join(LOCAL_MODEL_FILE))
        .map_err(|e| DownloadFailure::Failed(format!("finalize model file: {e}")))?;

    Ok(())
}

#[cfg(feature = "llm")]
fn remove_cache_dir(model_dir: &Path) -> Result<(), String> {
    let cache_dir = model_dir.join(TEMP_CACHE_DIR);
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).map_err(|e| format!("remove cache dir: {e}"))?;
    }
    Ok(())
}

#[cfg(feature = "llm")]
fn cleanup_partial_artifacts(model_dir: &Path) -> Result<(), String> {
    let temp_model = model_dir.join(TEMP_MODEL_FILE);
    if temp_model.exists() {
        std::fs::remove_file(&temp_model).map_err(|e| format!("remove partial model: {e}"))?;
    }
    remove_cache_dir(model_dir)?;
    if model_dir.exists()
        && std::fs::read_dir(model_dir)
            .map_err(|e| format!("read dir: {e}"))?
            .next()
            .is_none()
    {
        std::fs::remove_dir(model_dir).map_err(|e| format!("remove empty model dir: {e}"))?;
    }
    Ok(())
}

/// Delete model files from the given directory. Returns Ok even if files don't exist.
pub fn delete_model_files(model_dir: &Path) -> Result<(), String> {
    if model_dir.exists() {
        log::info!(
            "[llm-download] deleting all model files from {}",
            model_dir.display()
        );
        std::fs::remove_dir_all(model_dir).map_err(|e| format!("delete model dir: {e}"))?;
    }
    Ok(())
}

/// Returns true if the model file already exists in the given directory.
pub fn model_files_present(model_dir: &Path) -> bool {
    let model = model_dir.join(LOCAL_MODEL_FILE);
    model.exists()
        && std::fs::metadata(&model)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_files_not_present_in_empty_dir() {
        let dir = std::env::temp_dir().join("tuicity_test_llm_download");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!model_files_present(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn model_files_present_when_model_exists() {
        let dir = std::env::temp_dir().join("tuicity_test_llm_download_present");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(LOCAL_MODEL_FILE), b"fake model data").unwrap();
        assert!(model_files_present(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "llm")]
    #[test]
    fn cleanup_partial_artifacts_removes_temp_and_cache() {
        let dir = std::env::temp_dir().join("tuicity_test_llm_partial_cleanup");
        let cache_dir = dir.join(TEMP_CACHE_DIR);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(dir.join(TEMP_MODEL_FILE), b"partial").unwrap();
        std::fs::write(cache_dir.join("blob"), b"cached").unwrap();

        cleanup_partial_artifacts(&dir).unwrap();

        assert!(!dir.exists());
    }

    #[test]
    fn download_handle_poll_returns_none_when_empty() {
        let (_tx, rx) = mpsc::channel::<DownloadProgress>();
        let handle = DownloadHandle {
            rx,
            cancel_requested: Arc::new(AtomicBool::new(false)),
        };
        assert!(handle.poll().is_none());
    }

    /// Downloads the actual default model if not already present. Run with:
    /// cargo test --release textgen::download::tests::download_model_and_verify -- --nocapture --ignored
    #[test]
    #[ignore]
    fn download_model_and_verify() {
        let model = LlmModelId::default();
        let model_dir = crate::textgen::model_dir_for(model);
        if model_files_present(&model_dir) {
            eprintln!("model already present at {}", model_dir.display());
            return;
        }
        eprintln!(
            "downloading {} to {}...",
            model.label(),
            model_dir.display()
        );
        let handle =
            start_download(model_dir.clone(), model).expect("start_download returned None");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Some(progress) = handle.poll() {
                match progress {
                    DownloadProgress::Progress(progress) => {
                        eprintln!("  {} {:?}", progress.label, progress.total_bytes)
                    }
                    DownloadProgress::Done => {
                        eprintln!("download complete!");
                        break;
                    }
                    DownloadProgress::Cancelled => {
                        panic!("download unexpectedly canceled");
                    }
                    DownloadProgress::Failed(err) => {
                        panic!("download failed: {err}");
                    }
                }
            }
        }
        assert!(model_files_present(&model_dir));
    }

    #[test]
    fn download_handle_poll_returns_latest() {
        let (tx, rx) = mpsc::channel();
        tx.send(DownloadProgress::Progress(DownloadProgressSnapshot {
            label: "a".to_string(),
            downloaded_bytes: 1,
            total_bytes: Some(2),
        }))
        .unwrap();
        tx.send(DownloadProgress::Progress(DownloadProgressSnapshot {
            label: "b".to_string(),
            downloaded_bytes: 2,
            total_bytes: Some(2),
        }))
        .unwrap();
        tx.send(DownloadProgress::Done).unwrap();
        let handle = DownloadHandle {
            rx,
            cancel_requested: Arc::new(AtomicBool::new(false)),
        };
        match handle.poll().unwrap() {
            DownloadProgress::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }
}
