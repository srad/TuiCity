use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicCue {
    #[default]
    None,
    StartTheme,
}

pub struct MusicManager {
    current: MusicCue,
    #[cfg(windows)]
    backend: WindowsAudioBackend,
}

impl MusicManager {
    pub fn new() -> Self {
        Self {
            current: MusicCue::None,
            #[cfg(windows)]
            backend: WindowsAudioBackend::new(),
        }
    }

    pub fn sync(&mut self, cue: MusicCue) {
        if self.current == cue {
            return;
        }

        self.stop_all();

        match cue {
            MusicCue::None => {
                self.current = MusicCue::None;
            }
            MusicCue::StartTheme => {
                #[cfg(windows)]
                {
                    if let Some(path) = asset_path("assets/music/01_civic_sunrise_theme.wav")
                        .or_else(|| asset_path("assets/music/01_civic_sunrise_theme.mid"))
                    {
                        if self.backend.play_loop(&path) {
                            self.current = cue;
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    self.current = cue;
                }
            }
        }
    }

    pub fn stop_all(&mut self) {
        #[cfg(windows)]
        self.backend.stop();
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

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
struct WindowsAudioBackend {
    alias: &'static str,
    enabled: bool,
    wave_path: Option<Vec<u16>>,
}

#[cfg(windows)]
impl WindowsAudioBackend {
    fn new() -> Self {
        Self {
            alias: "tc2000_music",
            enabled: !cfg!(test),
            wave_path: None,
        }
    }

    fn play_loop(&mut self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }

        self.stop();
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("wav") => self.play_wave_loop(path),
            Some("mid") | Some("midi") => {
                let command = format!(
                    "open \"{}\" type sequencer alias {}",
                    path.display(),
                    self.alias
                );
                if mci_send(&command) == 0 {
                    return mci_send(&format!("play {} repeat", self.alias)) == 0;
                }
                false
            }
            _ => false,
        }
    }

    fn play_wave_loop(&mut self, path: &Path) -> bool {
        use std::{iter, os::windows::ffi::OsStrExt};

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();
        self.wave_path = Some(wide);
        unsafe {
            PlaySoundW(
                self.wave_path
                    .as_ref()
                    .map(|buffer| buffer.as_ptr())
                    .unwrap_or(std::ptr::null()),
                std::ptr::null_mut(),
                SND_ASYNC | SND_FILENAME | SND_LOOP | SND_NODEFAULT,
            ) != 0
        }
    }

    fn stop(&mut self) {
        if !self.enabled {
            return;
        }
        unsafe {
            PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0);
        }
        self.wave_path = None;
        let _ = mci_send(&format!("stop {}", self.alias));
        let _ = mci_send(&format!("close {}", self.alias));
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(sound_name: *const u16, module: *mut std::ffi::c_void, flags: u32) -> i32;
    fn mciSendStringW(
        command: *const u16,
        return_string: *mut u16,
        return_length: u32,
        callback: isize,
    ) -> u32;
}

#[cfg(windows)]
const SND_ASYNC: u32 = 0x0001;
#[cfg(windows)]
const SND_NODEFAULT: u32 = 0x0002;
#[cfg(windows)]
const SND_LOOP: u32 = 0x0008;
#[cfg(windows)]
const SND_FILENAME: u32 = 0x0002_0000;

#[cfg(windows)]
fn mci_send(command: &str) -> u32 {
    use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt};

    let wide: Vec<u16> = OsStr::new(command)
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    unsafe { mciSendStringW(wide.as_ptr(), std::ptr::null_mut(), 0, 0) }
}
