# Async LLM Loading

## Objective
Refactor `TextGenService::start` to load the LLM model asynchronously in a background thread. This prevents the application startup from blocking for several seconds (or indefinitely on slow systems), allowing the UI to render immediately.

## Context
Currently, `TextGenService::start` calls `select_backend`, which synchronously loads the 360MB+ LLM model into memory. This happens inside `AppState::new()`, blocking the main thread before the window event loop starts. On Windows/winit, this can cause the application to appear as a "ghost window" or fail to show up entirely if the delay is too long.

## Implementation Steps

### 1. Modify `src/textgen/mod.rs`
- [x] **Change `TextGenService` struct:**
    -   Change `backend_name` field from `String` to `Arc<std::sync::RwLock<String>>`.
- [x] **Update `start` method:**
    -   Initialize `backend_name` with `"initializing"`.
    -   Move the `select_backend` call inside the worker thread spawn closure.
    -   Inside the thread:
        -   Call `select_backend`.
        -   Update the shared `backend_name` with the result's name.
        -   Proceed to `worker_loop`.
- [x] **Update `backend_name()` method:**
    -   Change return type from `&str` to `String`.
    -   Return a clone of the string from the `RwLock`.
- [x] **Update `has_model()` method:**
    -   Read from the lock.
    -   Return `true` if name != "static" AND name != "initializing".
- [x] **Update `worker_loop`:**
    -   It currently takes `Box<dyn TextGenerator>`. It should remain the same, as `select_backend` returns this.

### 2. Verify Changes
- [x] Run `cargo check` to ensure type compatibility.
- [x] Run tests in `src/textgen/mod.rs` (will need to update tests that check `backend_name()` return type).
- [x] Manual verification: Run the game and ensure it starts immediately. The LLM status in settings should transition from "Initializing" (or similar) to "Active".

## Notes
-   `select_backend` prints to stderr, which is fine for background threads.
-   The `backend_name` will initially be "initializing". We should treat this as "no model yet".
-   If loading fails, `select_backend` falls back to "static", so the name will update to "static".
