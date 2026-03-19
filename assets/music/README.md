# Music Assets

This folder currently keeps the two music cues that survived the bulk-pack cleanup:

- `01_civic_sunrise_theme.mid`
- `01_civic_sunrise_theme.mp3`
- `01_civic_sunrise_theme.wav`
- `02_riverfront_reflections.mid`
- `02_riverfront_reflections.mp3`

What each file is for:

- `.mid`: composition guide material and import source for DAW work
- `.mp3`: quick listening/reference render
- `01_civic_sunrise_theme.wav`: Windows runtime asset for the title screen
- `manifest.json`: current metadata for the kept tracks

What is actually used by the game right now:

- `01_civic_sunrise_theme.wav`: used on the Windows start screen
- `02_riverfront_reflections.mp3`: kept as a gameplay music asset/reference file, but not auto-played yet

Current direction:

- The generator now only emits the two kept tracks.
- The current default render path prefers FluidSynth plus a local soundfont when available, with the built-in renderer kept only as fallback.
- The practical workflow for higher-quality revisions is still: use the `.mid` as guide material, refine the arrangement and sound design in LMMS with Surge XT, export final audio, then replace the `.mp3` and `.wav` assets here.
- See `LMMS_SURGE_WORKFLOW.md` in this folder for the exact process.

Copyright note:
These tracks were written specifically for this project and should remain original rather than quoting or closely paraphrasing existing soundtrack melodies.

Adding more tracks later:

- export the new audio into this folder using the same naming pattern
- add a metadata entry to `manifest.json`
- if the track should play in-game, wire it into `src/audio.rs` and the cue-selection logic in `src/app/mod.rs`
- keep the title-screen runtime WAV if the track is meant for reliable Windows playback
