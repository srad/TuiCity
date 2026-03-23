#[cfg(feature = "llm")]
mod inner {
    use super::super::generator::TextGenerator;
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;
    use llama_cpp_2::sampling::LlamaSampler;
    use std::path::Path;

    pub struct LlamaCppGenerator {
        model: LlamaModel,
        backend: LlamaBackend,
    }

    impl LlamaCppGenerator {
        pub fn load(model_dir: &Path) -> Result<Self, String> {
            let model_path = model_dir.join("model.gguf");
            if !model_path.exists() {
                return Err(format!("model not found: {}", model_path.display()));
            }

            // Suppress all llama.cpp internal logging (prevents terminal corruption)
            llama_cpp_2::send_logs_to_tracing(
                llama_cpp_2::LogOptions::default().with_logs_enabled(false),
            );

            let backend =
                LlamaBackend::init().map_err(|e| format!("llama backend init: {e}"))?;

            let model_params = LlamaModelParams::default();
            let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
                .map_err(|e| format!("model load: {e}"))?;

            Ok(Self { model, backend })
        }
    }

    impl TextGenerator for LlamaCppGenerator {
        fn generate(
            &mut self,
            prompt: &str,
            max_tokens: usize,
            temperature: f32,
        ) -> Result<String, String> {
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(Some(std::num::NonZeroU32::new(2048).unwrap()));
            let mut ctx = self
                .model
                .new_context(&self.backend, ctx_params)
                .map_err(|e| format!("context create: {e}"))?;

            let tokens = self
                .model
                .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
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
            Ok(text.trim().to_string())
        }

        fn backend_name(&self) -> &str {
            "llama.cpp"
        }
    }
}

#[cfg(feature = "llm")]
pub use inner::LlamaCppGenerator;
