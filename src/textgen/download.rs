/// Background model download using hf-hub.
///
/// Downloads a quantized GGUF model and tokenizer from HuggingFace Hub
/// into the local model directory (~/.tuicity2000/models/).
use std::path::PathBuf;
use std::sync::mpsc;

/// Progress updates sent from the download thread.
#[derive(Debug, Clone)]
pub enum DownloadProgress {
    /// Currently downloading a file (name, bytes so far if known).
    Downloading(String),
    /// All files downloaded successfully.
    Done,
    /// Download failed with an error message.
    Failed(String),
}

/// Receiver handle for polling download progress from the UI thread.
pub struct DownloadHandle {
    rx: mpsc::Receiver<DownloadProgress>,
}

impl DownloadHandle {
    /// Non-blocking poll for the latest progress update.
    pub fn poll(&self) -> Option<DownloadProgress> {
        // Drain to get the most recent message.
        let mut last = None;
        while let Ok(msg) = self.rx.try_recv() {
            last = Some(msg);
        }
        last
    }
}

/// Default HuggingFace repo and filenames for SmolLM2-360M Instruct (Q8_0 quantization).
/// At ~360M parameters (~380 MB quantized) this gives better output quality on CPU.
const HF_MODEL_REPO: &str = "bartowski/SmolLM2-360M-Instruct-GGUF";
const HF_MODEL_FILE: &str = "SmolLM2-360M-Instruct-Q8_0.gguf";
const HF_TOKENIZER_REPO: &str = "HuggingFaceTB/SmolLM2-360M-Instruct";
const HF_TOKENIZER_FILE: &str = "tokenizer.json";

/// Local filenames we save to.
const LOCAL_MODEL_FILE: &str = "model.gguf";
const LOCAL_TOKENIZER_FILE: &str = "tokenizer.json";

/// Spawn a background thread that downloads the model files into `model_dir`.
/// Returns a handle to poll progress, or `None` if the `llm` feature is not compiled.
pub fn start_download(model_dir: PathBuf) -> Option<DownloadHandle> {
    #[cfg(not(feature = "llm"))]
    {
        let _ = model_dir;
        None
    }

    #[cfg(feature = "llm")]
    {
        let (tx, rx) = mpsc::channel();

        std::thread::Builder::new()
            .name("llm-download".to_string())
            .spawn(move || {
                download_inner(model_dir, tx);
            })
            .ok()?;

        Some(DownloadHandle { rx })
    }
}

#[cfg(feature = "llm")]
fn download_inner(model_dir: PathBuf, tx: mpsc::Sender<DownloadProgress>) {
    // Ensure the directory exists.
    if let Err(e) = std::fs::create_dir_all(&model_dir) {
        let _ = tx.send(DownloadProgress::Failed(format!(
            "cannot create {}: {e}",
            model_dir.display()
        )));
        return;
    }

    let model_dest = model_dir.join(LOCAL_MODEL_FILE);
    let tokenizer_dest = model_dir.join(LOCAL_TOKENIZER_FILE);

    // Skip files that already exist and are non-empty.
    let model_exists = model_dest.exists()
        && std::fs::metadata(&model_dest)
            .map(|m| m.len() > 0)
            .unwrap_or(false);
    let tokenizer_exists = tokenizer_dest.exists()
        && std::fs::metadata(&tokenizer_dest)
            .map(|m| m.len() > 0)
            .unwrap_or(false);

    if model_exists && tokenizer_exists {
        let _ = tx.send(DownloadProgress::Done);
        return;
    }

    // Download model weights.
    if !model_exists {
        let _ = tx.send(DownloadProgress::Downloading(format!(
            "model ({HF_MODEL_REPO}/{HF_MODEL_FILE})"
        )));
        match download_hf_file(HF_MODEL_REPO, HF_MODEL_FILE) {
            Ok(cached_path) => {
                if let Err(e) = std::fs::copy(&cached_path, &model_dest) {
                    let _ = tx.send(DownloadProgress::Failed(format!("copy model: {e}")));
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(DownloadProgress::Failed(format!("download model: {e}")));
                return;
            }
        }
    }

    // Download tokenizer.
    if !tokenizer_exists {
        let _ = tx.send(DownloadProgress::Downloading(format!(
            "tokenizer ({HF_TOKENIZER_REPO}/{HF_TOKENIZER_FILE})"
        )));
        match download_hf_file(HF_TOKENIZER_REPO, HF_TOKENIZER_FILE) {
            Ok(cached_path) => {
                if let Err(e) = std::fs::copy(&cached_path, &tokenizer_dest) {
                    let _ = tx.send(DownloadProgress::Failed(format!("copy tokenizer: {e}")));
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(DownloadProgress::Failed(format!("download tokenizer: {e}")));
                return;
            }
        }
    }

    let _ = tx.send(DownloadProgress::Done);
}

/// Use hf-hub to download a single file from a HuggingFace repo.
/// Returns the local cached path on success.
#[cfg(feature = "llm")]
fn download_hf_file(repo_id: &str, filename: &str) -> Result<PathBuf, String> {
    use hf_hub::api::sync::Api;

    let api = Api::new().map_err(|e| format!("hf-hub init: {e}"))?;
    let repo = api.model(repo_id.to_string());
    let path = repo
        .get(filename)
        .map_err(|e| format!("hf-hub get {filename}: {e}"))?;
    Ok(path)
}

/// Delete model files from the given directory. Returns Ok even if files don't exist.
pub fn delete_model_files(model_dir: &std::path::Path) -> Result<(), String> {
    let model = model_dir.join(LOCAL_MODEL_FILE);
    let tokenizer = model_dir.join(LOCAL_TOKENIZER_FILE);
    if model.exists() {
        std::fs::remove_file(&model).map_err(|e| format!("delete model: {e}"))?;
    }
    if tokenizer.exists() {
        std::fs::remove_file(&tokenizer).map_err(|e| format!("delete tokenizer: {e}"))?;
    }
    Ok(())
}

/// Returns true if the model files already exist in the given directory.
pub fn model_files_present(model_dir: &std::path::Path) -> bool {
    let model = model_dir.join(LOCAL_MODEL_FILE);
    let tokenizer = model_dir.join(LOCAL_TOKENIZER_FILE);
    model.exists()
        && tokenizer.exists()
        && std::fs::metadata(&model)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
        && std::fs::metadata(&tokenizer)
            .map(|m| m.len() > 0)
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
    fn model_files_present_when_both_exist() {
        let dir = std::env::temp_dir().join("tuicity_test_llm_download_present");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.gguf"), b"fake model data").unwrap();
        std::fs::write(dir.join("tokenizer.json"), b"{}").unwrap();
        assert!(model_files_present(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn download_handle_poll_returns_none_when_empty() {
        let (_tx, rx) = mpsc::channel::<DownloadProgress>();
        let handle = DownloadHandle { rx };
        assert!(handle.poll().is_none());
    }

    /// Downloads the actual model if not already present. Run with:
    /// cargo test --release textgen::download::tests::download_model_and_verify -- --nocapture --ignored
    #[test]
    #[ignore]
    fn download_model_and_verify() {
        let model_dir = crate::textgen::default_model_dir();
        if model_files_present(&model_dir) {
            eprintln!("model already present at {}", model_dir.display());
            return;
        }
        eprintln!("downloading model to {}...", model_dir.display());
        let handle = start_download(model_dir.clone()).expect("start_download returned None");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Some(progress) = handle.poll() {
                match progress {
                    DownloadProgress::Downloading(what) => eprintln!("  downloading {what}..."),
                    DownloadProgress::Done => {
                        eprintln!("download complete!");
                        break;
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
        tx.send(DownloadProgress::Downloading("a".to_string()))
            .unwrap();
        tx.send(DownloadProgress::Downloading("b".to_string()))
            .unwrap();
        tx.send(DownloadProgress::Done).unwrap();
        let handle = DownloadHandle { rx };
        // poll drains all, returns the last one
        match handle.poll().unwrap() {
            DownloadProgress::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }
}
