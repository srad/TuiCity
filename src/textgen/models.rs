#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmModelId {
    Gemma3_270mIt,
    Gemma3_1bIt,
    Gemma3_4bIt,
}

impl Default for LlmModelId {
    fn default() -> Self {
        Self::Gemma3_1bIt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStyle {
    Gemma,
}

pub fn compiled_gpu_backend_name() -> &'static str {
    if cfg!(feature = "llm-cuda") {
        "CUDA"
    } else if cfg!(feature = "llm-vulkan") {
        "Vulkan"
    } else if cfg!(feature = "llm-rocm") {
        "ROCm"
    } else if cfg!(feature = "llm-metal") {
        "Metal"
    } else {
        "none"
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LlmModelSpec {
    pub directory_name: &'static str,
    pub label: &'static str,
    pub short_label: &'static str,
    pub description: &'static str,
    pub download_size_label: &'static str,
    pub repo_id: &'static str,
    pub remote_file: &'static str,
    pub prompt_style: PromptStyle,
}

const GEMMA_270M_SPEC: LlmModelSpec = LlmModelSpec {
    directory_name: "gemma-3-270m-it",
    label: "Gemma 3 270M IT",
    short_label: "Gemma 270M",
    description: "Fastest to generate, but the writing is simpler and less detailed.",
    download_size_label: "~0.25 GB",
    repo_id: "bartowski/google_gemma-3-270m-it-GGUF",
    remote_file: "google_gemma-3-270m-it-Q4_K_M.gguf",
    prompt_style: PromptStyle::Gemma,
};

const GEMMA_1B_SPEC: LlmModelSpec = LlmModelSpec {
    directory_name: "gemma-3-1b-it",
    label: "Gemma 3 1B IT",
    short_label: "Gemma 1B",
    description: "Best balance of speed and newspaper quality for most players.",
    download_size_label: "~0.81 GB",
    repo_id: "bartowski/google_gemma-3-1b-it-GGUF",
    remote_file: "google_gemma-3-1b-it-Q4_K_M.gguf",
    prompt_style: PromptStyle::Gemma,
};

const GEMMA_4B_SPEC: LlmModelSpec = LlmModelSpec {
    directory_name: "gemma-3-4b-it",
    label: "Gemma 3 4B IT",
    short_label: "Gemma 4B",
    description: "Most interesting writing, but noticeably slower to generate.",
    download_size_label: "~2.49 GB",
    repo_id: "bartowski/google_gemma-3-4b-it-GGUF",
    remote_file: "google_gemma-3-4b-it-Q4_K_M.gguf",
    prompt_style: PromptStyle::Gemma,
};

const MODEL_ORDER: [LlmModelId; 3] = [
    LlmModelId::Gemma3_270mIt,
    LlmModelId::Gemma3_1bIt,
    LlmModelId::Gemma3_4bIt,
];

impl LlmModelId {
    pub fn all() -> &'static [Self] {
        &MODEL_ORDER
    }

    pub fn spec(self) -> &'static LlmModelSpec {
        match self {
            Self::Gemma3_270mIt => &GEMMA_270M_SPEC,
            Self::Gemma3_1bIt => &GEMMA_1B_SPEC,
            Self::Gemma3_4bIt => &GEMMA_4B_SPEC,
        }
    }

    pub fn cycle(self, direction: i32) -> Self {
        let all = Self::all();
        let current = all
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        if direction < 0 {
            all[(current + all.len() - 1) % all.len()]
        } else {
            all[(current + 1) % all.len()]
        }
    }

    pub fn label(self) -> &'static str {
        self.spec().label
    }

    pub fn short_label(self) -> &'static str {
        self.spec().short_label
    }

    pub fn description(self) -> &'static str {
        self.spec().description
    }

    pub fn download_size_label(self) -> &'static str {
        self.spec().download_size_label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmExecutionMode {
    Auto,
    CpuOnly,
    GpuAccelerated,
}

impl Default for LlmExecutionMode {
    fn default() -> Self {
        Self::Auto
    }
}

const EXECUTION_MODE_ORDER: [LlmExecutionMode; 3] = [
    LlmExecutionMode::Auto,
    LlmExecutionMode::CpuOnly,
    LlmExecutionMode::GpuAccelerated,
];

impl LlmExecutionMode {
    pub fn all() -> &'static [Self] {
        &EXECUTION_MODE_ORDER
    }

    pub fn cycle(self, direction: i32) -> Self {
        let all = Self::all();
        let current = all
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        if direction < 0 {
            all[(current + all.len() - 1) % all.len()]
        } else {
            all[(current + 1) % all.len()]
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::CpuOnly => "CPU Only",
            Self::GpuAccelerated => "Prefer GPU",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Auto => "Best default. Uses your graphics card when it is supported.",
            Self::CpuOnly => "Most compatible. Slower, but avoids graphics card issues.",
            Self::GpuAccelerated => {
                "Pushes for faster GPU generation and falls back to the CPU if needed."
            }
        }
    }

    pub fn selection_hint(self) -> &'static str {
        match self {
            Self::Auto => "Will use your graphics card automatically when the model can.",
            Self::CpuOnly => "Will stay on the CPU even if a graphics card is available.",
            Self::GpuAccelerated => "Will try to use your graphics card and fall back to the CPU.",
        }
    }

    pub fn runtime_status(self, gpu_available: bool, gpu_active: bool) -> String {
        let compiled_backend = compiled_gpu_backend_name();
        match self {
            Self::Auto => {
                if gpu_active {
                    "Auto mode is using your graphics card for faster generation.".to_string()
                } else if gpu_available {
                    "Auto mode stayed on the CPU for this run.".to_string()
                } else if compiled_backend == "none" {
                    "Auto mode is using the CPU because this build was compiled without GPU acceleration."
                        .to_string()
                } else {
                    format!(
                        "Auto mode is using the CPU because the {compiled_backend} backend was not available at runtime."
                    )
                }
            }
            Self::CpuOnly => {
                if gpu_available {
                    "CPU only mode is active even though a graphics card is available.".to_string()
                } else {
                    "CPU only mode is active.".to_string()
                }
            }
            Self::GpuAccelerated => {
                if gpu_active {
                    "GPU mode is active and using your graphics card.".to_string()
                } else if gpu_available {
                    "GPU mode was requested, but generation stayed on the CPU.".to_string()
                } else if compiled_backend == "none" {
                    "GPU mode was requested, but this build was compiled without GPU acceleration."
                        .to_string()
                } else {
                    format!(
                        "GPU mode was requested, but the {compiled_backend} backend was not available at runtime."
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_balanced_gemma() {
        assert_eq!(LlmModelId::default(), LlmModelId::Gemma3_1bIt);
    }

    #[test]
    fn model_cycle_wraps_in_both_directions() {
        assert_eq!(LlmModelId::Gemma3_270mIt.cycle(-1), LlmModelId::Gemma3_4bIt);
        assert_eq!(LlmModelId::Gemma3_4bIt.cycle(1), LlmModelId::Gemma3_270mIt);
    }

    #[test]
    fn execution_mode_cycle_wraps_in_both_directions() {
        assert_eq!(
            LlmExecutionMode::Auto.cycle(-1),
            LlmExecutionMode::GpuAccelerated
        );
        assert_eq!(
            LlmExecutionMode::GpuAccelerated.cycle(1),
            LlmExecutionMode::Auto
        );
    }

    #[test]
    fn gpu_runtime_status_mentions_cpu_when_unavailable() {
        let status = LlmExecutionMode::GpuAccelerated.runtime_status(false, false);
        assert!(
            status.contains("compiled without GPU acceleration")
                || status.contains("backend was not available at runtime")
        );
    }
}
