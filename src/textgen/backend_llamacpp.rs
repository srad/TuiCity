#[cfg(feature = "llm")]
mod inner {
    use super::super::generator::TextGenerator;
    use crate::textgen::models::{LlmExecutionMode, PromptStyle};
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;
    use llama_cpp_2::sampling::LlamaSampler;
    use std::path::Path;

    const SYSTEM_PROMPT: &str = "You write in-universe text for a city simulation game. Follow the user's requested format exactly and return only the requested content.";

    pub struct LlamaLoadReport {
        pub generator: LlamaCppGenerator,
        pub gpu_available: bool,
        pub gpu_active: bool,
        pub acceleration_summary: String,
    }

    pub struct LlamaCppGenerator {
        model: LlamaModel,
        backend: LlamaBackend,
        prompt_style: PromptStyle,
        ctx_template: LlamaContextParams,
    }

    impl LlamaCppGenerator {
        fn wrap_prompt(prompt_style: PromptStyle, prompt: &str) -> String {
            match prompt_style {
                PromptStyle::Gemma => format!(
                    "<start_of_turn>user\n{SYSTEM_PROMPT}\n\n{prompt}<end_of_turn>\n<start_of_turn>model\n"
                ),
            }
        }

        fn cleanup_output(prompt_style: PromptStyle, text: &str) -> String {
            match prompt_style {
                PromptStyle::Gemma => text
                    .replace("<bos>", "")
                    .replace("<start_of_turn>model", "")
                    .replace("<end_of_turn>", "")
                    .replace("<eos>", "")
                    .replace("<|endoftext|>", "")
                    .trim()
                    .to_string(),
            }
        }

        pub fn load(
            model_dir: &Path,
            prompt_style: PromptStyle,
            execution_mode: LlmExecutionMode,
        ) -> Result<LlamaLoadReport, String> {
            let model_path = model_dir.join("model.gguf");
            if !model_path.exists() {
                return Err(format!("model not found: {}", model_path.display()));
            }

            // Suppress all llama.cpp internal logging (prevents terminal corruption)
            llama_cpp_2::send_logs_to_tracing(
                llama_cpp_2::LogOptions::default().with_logs_enabled(false),
            );

            let backend = LlamaBackend::init().map_err(|e| format!("llama backend init: {e}"))?;
            let gpu_available = backend.supports_gpu_offload();
            let gpu_active = match execution_mode {
                LlmExecutionMode::CpuOnly => false,
                LlmExecutionMode::Auto | LlmExecutionMode::GpuAccelerated => gpu_available,
            };

            let model_params = if gpu_active {
                LlamaModelParams::default()
            } else {
                LlamaModelParams::default().with_n_gpu_layers(0)
            };
            let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
                .map_err(|e| format!("model load: {e}"))?;

            let ctx_template = LlamaContextParams::default()
                .with_n_ctx(Some(std::num::NonZeroU32::new(2048).unwrap()))
                .with_offload_kqv(gpu_active);

            Ok(LlamaLoadReport {
                generator: Self {
                    model,
                    backend,
                    prompt_style,
                    ctx_template,
                },
                gpu_available,
                gpu_active,
                acceleration_summary: execution_mode.runtime_status(gpu_available, gpu_active),
            })
        }
    }

    impl TextGenerator for LlamaCppGenerator {
        fn generate(
            &mut self,
            prompt: &str,
            max_tokens: usize,
            temperature: f32,
        ) -> Result<String, String> {
            let wrapped_prompt = Self::wrap_prompt(self.prompt_style, prompt);
            let ctx_params = self.ctx_template.clone();
            let mut ctx = self
                .model
                .new_context(&self.backend, ctx_params)
                .map_err(|e| format!("context create: {e}"))?;

            let tokens = self
                .model
                .str_to_token(&wrapped_prompt, llama_cpp_2::model::AddBos::Always)
                .map_err(|e| format!("tokenize: {e}"))?;

            // Feed prompt tokens
            let mut batch = LlamaBatch::new(2048, 1);
            for (i, &token) in tokens.iter().enumerate() {
                let is_last = i == tokens.len() - 1;
                batch
                    .add(token, i as i32, &[0], is_last)
                    .map_err(|e| format!("batch add: {e}"))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| format!("decode prompt: {e}"))?;

            let mut generated_tokens = Vec::new();
            let mut n_cur = tokens.len();

            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u32)
                .unwrap_or(42);
            let mut sampler = LlamaSampler::chain_simple([
                LlamaSampler::min_p(0.05, 1),
                LlamaSampler::top_p(0.9, 1),
                LlamaSampler::temp(temperature),
                LlamaSampler::dist(seed),
            ]);

            for _ in 0..max_tokens {
                let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);

                if self.model.is_eog_token(new_token) {
                    break;
                }

                let piece = self
                    .model
                    .token_to_piece_bytes(new_token, 32, false, None)
                    .map_err(|e| format!("detokenize: {e}"))?;
                generated_tokens.extend(piece);

                batch.clear();
                batch
                    .add(new_token, n_cur as i32, &[0], true)
                    .map_err(|e| format!("batch add gen: {e}"))?;
                n_cur += 1;

                ctx.decode(&mut batch)
                    .map_err(|e| format!("decode gen: {e}"))?;
            }

            let text = String::from_utf8_lossy(&generated_tokens).into_owned();
            Ok(Self::cleanup_output(self.prompt_style, &text))
        }

        fn backend_name(&self) -> &str {
            "llama.cpp"
        }
    }

    #[cfg(test)]
    mod tests {
        use super::LlamaCppGenerator;
        use crate::textgen::models::PromptStyle;

        #[test]
        fn wrap_prompt_uses_gemma_turn_format() {
            let wrapped = LlamaCppGenerator::wrap_prompt(PromptStyle::Gemma, "Write a headline.");
            assert!(wrapped.starts_with("<start_of_turn>user\n"));
            assert!(wrapped.contains("Write a headline.<end_of_turn>\n<start_of_turn>model\n"));
        }

        #[test]
        fn cleanup_output_strips_gemma_tokens() {
            let cleaned = LlamaCppGenerator::cleanup_output(
                PromptStyle::Gemma,
                "<start_of_turn>model\nHeadline<end_of_turn>\n",
            );
            assert_eq!(cleaned, "Headline");
        }

        #[test]
        fn cleanup_output_strips_bos_and_eos_tokens() {
            let cleaned =
                LlamaCppGenerator::cleanup_output(PromptStyle::Gemma, "<bos>Headline<eos>");
            assert_eq!(cleaned, "Headline");
        }
    }
}

#[cfg(feature = "llm")]
pub use inner::LlamaCppGenerator;
