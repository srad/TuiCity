/// Abstract interface for text generation backends.
///
/// Implementations include `LlamaCppGenerator` (llama.cpp via llama-cpp-2 crate)
/// and `StaticGenerator` (hardcoded responses, always available).
pub trait TextGenerator: Send {
    /// Generate text continuation from a prompt. Blocking — called from the worker thread.
    fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, String>;

    /// Human-readable backend name for logging/status display.
    fn backend_name(&self) -> &str;
}
