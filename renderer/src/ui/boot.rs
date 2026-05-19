use std::time::{Duration, Instant};

const TYPING_STEP: Duration = Duration::from_millis(8);
const FADE_STEP: f32 = 0.08;

pub enum BootPhase {
    Typing {
        line: usize,
        char_pos: usize,
        timer: Instant,
    },
    FadeIn {
        alpha: f32,
    },
    Done,
}

pub struct BootAnimation {
    pub phase: BootPhase,
    pub lines: Vec<&'static str>,
    pub displayed: Vec<String>,
    pub start_time: Instant,
}

impl Default for BootAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl BootAnimation {
    pub fn new() -> Self {
        Self {
            phase: BootPhase::Typing {
                line: 0,
                char_pos: 0,
                timer: Instant::now(),
            },
            lines: vec![
                concat!(
                    "eDEX-DE v",
                    env!("CARGO_PKG_VERSION"),
                    " — Wayland Desktop Environment"
                ),
                "Initializing Wayland compositor...",
                "Loading wgpu renderer (Vulkan)...",
                "Mounting filesystem interface...",
                "Starting terminal emulator...",
                "Connecting system monitor...",
                "Boot sequence complete.",
            ],
            displayed: vec![String::new()],
            start_time: Instant::now(),
        }
    }

    pub fn update(&mut self) -> bool {
        match &mut self.phase {
            BootPhase::Typing {
                line,
                char_pos,
                timer,
            } => {
                if timer.elapsed() < TYPING_STEP {
                    return false;
                }

                *timer = Instant::now();
                let current_line = self.lines[*line];
                let next_len = *char_pos + 1;
                if next_len <= current_line.len() {
                    self.displayed[*line] = current_line[..next_len].to_string();
                    *char_pos = next_len;
                }

                if *char_pos >= current_line.len() {
                    if *line + 1 >= self.lines.len() {
                        self.phase = BootPhase::FadeIn { alpha: 1.0 };
                    } else {
                        *line += 1;
                        *char_pos = 0;
                        self.displayed.push(String::new());
                    }
                }

                false
            }
            BootPhase::FadeIn { alpha } => {
                *alpha = (*alpha - FADE_STEP).max(0.0);
                if *alpha <= f32::EPSILON {
                    self.phase = BootPhase::Done;
                    return true;
                }
                false
            }
            BootPhase::Done => true,
        }
    }

    pub fn current_lines(&self) -> &[String] {
        &self.displayed
    }

    pub fn overlay_alpha(&self) -> f32 {
        match self.phase {
            BootPhase::Typing { .. } => 1.0,
            BootPhase::FadeIn { alpha } => alpha,
            BootPhase::Done => 0.0,
        }
    }
}
