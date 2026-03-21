use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicCue {
    #[default]
    None,
    StartTheme,
    Gameplay,
}

pub struct MusicManager {
    current: MusicCue,
    backend: AudioBackend,
}

impl MusicManager {
    pub fn new() -> Self {
        Self {
            current: MusicCue::None,
            backend: AudioBackend::new(),
        }
    }

    pub fn sync(&mut self, cue: MusicCue) {
        if self.current == cue {
            if cue != MusicCue::None {
                self.backend.ensure_playing();
            }
            return;
        }

        self.current = cue;

        if cue == MusicCue::None {
            self.backend.stop_all();
        } else {
            self.backend.ensure_playing();
        }
    }

    pub fn stop_all(&mut self) {
        self.backend.stop_all();
        self.current = MusicCue::None;
    }
}

impl Drop for MusicManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn asset_path(relative: &str) -> Option<PathBuf> {
    let relative_path = Path::new(relative);
    let candidates = [
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path)),
        std::env::current_dir()
            .ok()
            .map(|dir| dir.join(relative_path)),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(relative_path))),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .and_then(|dir| dir.parent())
                .map(|dir| dir.join(relative_path))
        }),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .and_then(|dir| dir.parent())
                .and_then(|dir| dir.parent())
                .map(|dir| dir.join(relative_path))
        }),
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.exists())
}

enum AudioCommand {
    EnsurePlaying,
    StopAll,
}

struct AudioBackend {
    tx: Sender<AudioCommand>,
}

impl AudioBackend {
    fn new() -> Self {
        let (tx, rx) = channel();
        let enabled = !cfg!(test);

        thread::spawn(move || {
            // Keep output_stream alive as long as the thread runs
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(res) => res,
                Err(_) => return, // No audio device
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(res) => res,
                Err(_) => return,
            };

            let mut player = AudioPlayer::new(enabled, sink);

            loop {
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(AudioCommand::EnsurePlaying) => {
                        player.ensure_playing();
                    }
                    Ok(AudioCommand::StopAll) => {
                        player.stop_all();
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if player.playing {
                            player.ensure_playing();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        player.stop_all();
                        break;
                    }
                }
            }
        });

        Self { tx }
    }

    fn ensure_playing(&mut self) {
        let _ = self.tx.send(AudioCommand::EnsurePlaying);
    }

    fn stop_all(&mut self) {
        let _ = self.tx.send(AudioCommand::StopAll);
    }
}

struct AudioPlayer {
    enabled: bool,
    playlist: Vec<PathBuf>,
    current_track: usize,
    playing: bool,
    sink: Sink,
}

impl AudioPlayer {
    fn new(enabled: bool, sink: Sink) -> Self {
        let mut player = Self {
            enabled,
            playlist: Vec::new(),
            current_track: 0,
            playing: false,
            sink,
        };
        if enabled {
            player.load_playlist();
        }
        player
    }

    fn load_playlist(&mut self) {
        let mut paths = Vec::new();
        if let Some(assets_dir) = asset_path("assets/music") {
            let mut dirs = vec![assets_dir];
            while let Some(dir) = dirs.pop() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            dirs.push(path);
                        } else if let Some(ext) = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.to_ascii_lowercase())
                        {
                            if ext == "mp3" {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }
        self.playlist = paths;
        self.shuffle_playlist();
    }

    fn shuffle_playlist(&mut self) {
        let mut rng = rand::thread_rng();
        self.playlist.shuffle(&mut rng);
        self.current_track = 0;
    }

    fn ensure_playing(&mut self) {
        if !self.enabled || self.playlist.is_empty() {
            return;
        }

        if !self.playing {
            self.play_current();
        } else if self.sink.empty() {
            self.next_track();
        } else if self.sink.is_paused() {
            self.sink.play();
        }
    }

    fn play_current(&mut self) {
        self.sink.stop(); // Clear the sink

        if self.playlist.is_empty() {
            return;
        }

        let path = &self.playlist[self.current_track];
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            if let Ok(decoder) = Decoder::new(reader) {
                self.sink.append(decoder);
                self.sink.play();
                self.playing = true;
            } else {
                // If decoding fails, skip to next track to avoid stalling
                self.next_track();
            }
        } else {
            self.next_track();
        }
    }

    fn next_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        self.current_track += 1;
        if self.current_track >= self.playlist.len() {
            self.shuffle_playlist();
        }
        self.play_current();
    }

    fn stop_all(&mut self) {
        if !self.enabled {
            return;
        }
        self.sink.stop();
        self.playing = false;
    }
}
