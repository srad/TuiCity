# LMMS + Surge XT Workflow

This is the practical path for making the current tracks sound better.

Use these source files:

- `assets/music/01_civic_sunrise_theme.mid`
- `assets/music/02_riverfront_reflections.mid`

Target outputs:

- `assets/music/01_civic_sunrise_theme.mp3`
- `assets/music/01_civic_sunrise_theme.wav`
- `assets/music/02_riverfront_reflections.mp3`

The game currently plays `01_civic_sunrise_theme.wav` on the start screen on Windows.

## 1. Import the MIDI into LMMS

1. Open LMMS.
2. Start from the `Empty` template.
3. Use `File -> Import` and choose one of the `.mid` files above.
4. Save the project into a local working file such as `assets/music/01_civic_sunrise_theme.mmpz`.

LMMS has a MIDI importer, but command-line import was unreliable in this setup, so manual import is the safer path.

## 2. Replace the cheap GM sound with a small Surge palette

Do not try to keep lots of pseudo-orchestral parts. Use 3 or 4 parts max.

Recommended split:

- lead: soft synth keys, mellow pluck, or lightly detuned digital bell
- harmony/pad: warm, slow-attack pad
- bass: rounded electric bass or muted synth bass
- optional texture: very quiet air, shimmer, or motion layer

Good constraint:

- one clear lead sound
- one supportive pad
- one bass
- one subtle texture

If you pile on more than that, the track will get busy and still sound cheap.

## 3. Load Surge XT in LMMS

On this machine Surge XT is installed as a VST3 under:

- `C:\Program Files\Common Files\VST3\Surge Synth Team\Surge XT.vst3`

In LMMS, use `Vestige` to host Surge XT if needed.

Practical approach:

1. Add one instrument track per role.
2. Load Surge XT on each track.
3. Start from simple presets, not complex showcase patches.
4. Keep the arrangement sparse and readable.

## 4. Improve the arrangement before rendering

The existing MIDI is only guide material. Edit it.

Focus on:

- remove repeated filler notes
- leave space between phrases
- vary the melody on each return
- drop parts out between sections
- make the bass less constant
- shorten sections that overstay their welcome

For this project, better `2` memorable minutes beats `3+` minutes of mechanical repetition.

## 5. Basic mixing that matters

Keep it simple:

- high-pass the pad if it muddies the bass
- keep the bass mono-ish and steady
- add only light reverb
- keep the lead in front, but not bright or piercing
- avoid hard compression

This soundtrack should feel calm, civic, and reflective, not cinematic or glossy.

## 6. Export

Export a full-quality WAV from LMMS first.

For the title theme:

- export WAV to `assets/music/01_civic_sunrise_theme.wav`
- encode MP3 to `assets/music/01_civic_sunrise_theme.mp3`

For the gameplay cue:

- export WAV temporarily
- encode MP3 to `assets/music/02_riverfront_reflections.mp3`

Example MP3 conversion:

```powershell
ffmpeg -y -i .\assets\music\02_riverfront_reflections.wav .\assets\music\02_riverfront_reflections.mp3
```

## 7. Keep the metadata in sync

After replacing a track:

- update `assets/music/manifest.json`
- keep the title theme WAV present
- avoid adding files that the game does not reference yet

## 8. Add more tracks in the future

If you want a new cue rather than replacing an existing one:

1. Pick a slug and keep the same naming style, for example `03_new_track_name`.
2. Export the listening file as `assets/music/03_new_track_name.mp3`.
3. Export a WAV only if the game will use that file directly at runtime.
4. Add the new entry to `assets/music/manifest.json`.
5. If the track should actually play in the game, extend `src/audio.rs` for the cue/asset mapping.
6. Extend `src/app/mod.rs` for when that cue should be active.

Right now the code only selects the start-screen theme automatically. Extra files in `assets/music` do nothing by themselves until the Rust playback logic references them.

## Suggested sound direction

For `01_civic_sunrise_theme`:

- slightly hopeful
- gentle urban optimism
- clear theme statement
- restrained tempo and dynamics

For `02_riverfront_reflections`:

- lighter and brighter than the title theme
- more open voicings
- less solemn
- still calm enough to loop during long play sessions
