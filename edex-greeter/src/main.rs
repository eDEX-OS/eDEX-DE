//! eDEX greeter for greetd. Runs as a fullscreen window under `cage`.

mod config;
mod greetd;
mod screen;
mod sessions;
mod stats;
mod user_theme;
mod users;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use calloop::{
    channel,
    timer::{TimeoutAction, Timer},
    EventLoop,
};
use clap::Parser;
use config::{GreeterConfig, State};
use greetd::{Greetd, Step};
use platform::{button, KeyInput, Platform, PlatformEvent, SurfaceId};
use renderer::{GpuContext, SurfaceRenderer};
use sessions::Session;
use tracing::{error, info, warn};
use users::User;
use xkbcommon::xkb::keysyms as ks;

#[derive(Parser, Debug)]
#[command(name = "edex-greeter", version, about = "eDEX greeter for greetd")]
struct Cli {
    /// Configuration file.
    #[arg(long, default_value = config::DEFAULT_PATH)]
    config: PathBuf,
    /// Run without greetd (renders the UI; Enter prints the session command).
    #[arg(long)]
    demo: bool,
    /// Exit after N seconds with a JSON report (CI).
    #[arg(long, value_name = "SECS")]
    smoke_test: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    PickUser,
    Prompt,
    Busy,
    Starting,
}

pub enum Event {
    Greetd(Box<(Greetd, Result<Step>)>),
    Tick,
}

pub struct Greeter {
    pub cfg: GreeterConfig,
    pub theme: ::ui::Theme,
    themes: std::collections::BTreeMap<String, ::ui::Theme>,
    /// User whose published theme is applied (re-checked when the selection changes).
    themed_for: Option<String>,
    pub stats: stats::Stats,
    /// Timestamped events shown in the ACCESS LOG panel.
    pub log: std::collections::VecDeque<(String, String, bool)>,
    stats_at: Instant,
    pub metrics: ::ui::Metrics,
    pub users: Vec<User>,
    pub sessions: Vec<Session>,
    pub user_idx: usize,
    pub session_idx: usize,
    pub username_input: String,
    pub input: String,
    pub secret: bool,
    pub prompt: String,
    pub message: Option<(String, bool)>,
    pub phase: Phase,
    pub caps_lock: bool,
    pub hostname: String,
    pub clock: String,
    pub date: String,
    started: Instant,
    greetd: Option<Greetd>,
    demo: bool,
    tx: channel::Sender<Event>,
    quit: bool,
    exit_code: i32,
}

impl Greeter {
    pub fn pulse(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() * 0.8).sin() * 0.5 + 0.5
    }

    pub fn started_secs(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    pub fn date_short(&self) -> String {
        chrono::Local::now().format("%a %d %b %Y").to_string()
    }

    fn current_user(&self) -> String {
        if self.cfg.show_users && !self.users.is_empty() {
            self.users
                .get(self.user_idx)
                .map(|u| u.name.clone())
                .unwrap_or_default()
        } else {
            self.username_input.trim().to_string()
        }
    }

    fn tick_clock(&mut self) {
        let now = chrono::Local::now();
        self.clock = now.format("%H:%M:%S").to_string();
        self.date = now.format("%A %d %B %Y").to_string();
    }

    fn set_message(&mut self, msg: impl Into<String>, error: bool) {
        let msg = msg.into();
        self.log_event(msg.clone(), error);
        self.message = Some((msg, error));
    }

    pub fn log_event(&mut self, text: impl Into<String>, error: bool) {
        let text = text.into();
        if self.log.back().is_some_and(|(_, t, _)| *t == text) {
            return;
        }
        if self.log.len() >= 40 {
            self.log.pop_front();
        }
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        self.log.push_back((ts, text, error));
    }

    /// Use the theme the selected user picked in eDEX-DE, else the configured one.
    fn sync_user_theme(&mut self) {
        let (name, uid) = if self.cfg.show_users && !self.users.is_empty() {
            match self.users.get(self.user_idx) {
                Some(u) => (u.name.clone(), Some(u.uid)),
                None => return,
            }
        } else {
            let n = self.username_input.trim().to_string();
            let uid = self.users.iter().find(|u| u.name == n).map(|u| u.uid);
            (n, uid)
        };
        if self.themed_for.as_deref() == Some(name.as_str()) {
            return;
        }
        let published =
            uid.and_then(|uid| user_theme::read(std::path::Path::new(user_theme::DIR), &name, uid));
        let theme = published
            .as_ref()
            .and_then(|t| self.themes.get(t))
            .or_else(|| self.themes.get(&self.cfg.theme))
            .cloned()
            .unwrap_or_else(::ui::theme::builtin_tron);
        if theme.name != self.theme.name {
            self.log_event(format!("theme {} for {name}", theme.name), false);
        }
        self.theme = theme;
        self.themed_for = Some(name);
    }

    /// Clock, stats and theme; returns true when something visible changed.
    fn tick(&mut self) -> bool {
        let clock = self.clock.clone();
        self.tick_clock();
        let mut changed = clock != self.clock;
        if self.stats_at.elapsed() >= Duration::from_secs(1) {
            self.stats.refresh();
            self.stats_at = Instant::now();
            changed = true;
        }
        let themed = self.themed_for.clone();
        self.sync_user_theme();
        changed || themed != self.themed_for
    }

    /// Run a greetd call on a worker thread; the result comes back as `Event::Greetd`.
    fn call(&mut self, f: impl FnOnce(&mut Greetd) -> Result<Step> + Send + 'static) {
        let Some(mut g) = self.greetd.take() else {
            if self.demo {
                self.demo_step();
            }
            return;
        };
        self.phase = Phase::Busy;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = f(&mut g);
            let _ = tx.send(Event::Greetd(Box::new((g, res))));
        });
    }

    fn demo_step(&mut self) {
        match self.phase {
            Phase::PickUser => {
                self.phase = Phase::Prompt;
                self.prompt = "Password:".into();
                self.secret = true;
                self.input.clear();
                self.set_message("demo mode: any password is accepted", false);
            }
            Phase::Prompt => {
                let cmd = self.session_command();
                println!("{}", cmd.join(" "));
                self.set_message(format!("demo: would start `{}`", cmd.join(" ")), false);
                self.phase = Phase::PickUser;
                self.input.clear();
            }
            _ => {}
        }
    }

    fn session_command(&self) -> Vec<String> {
        self.sessions
            .get(self.session_idx)
            .map(|s| s.exec.clone())
            .unwrap_or_else(|| vec![self.cfg.fallback_command.clone()])
    }

    fn session_env(&self) -> Vec<String> {
        let id = self
            .sessions
            .get(self.session_idx)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "edex-de".into());
        let x11 = self.sessions.get(self.session_idx).is_some_and(|s| s.x11);
        let mut env = vec![
            format!("XDG_SESSION_TYPE={}", if x11 { "x11" } else { "wayland" }),
            format!("XDG_SESSION_DESKTOP={id}"),
        ];
        if id == "edex-de" {
            env.push("XDG_CURRENT_DESKTOP=eDEX-DE:Hyprland".into());
        }
        env
    }

    fn submit(&mut self) {
        match self.phase {
            Phase::PickUser => {
                let user = self.current_user();
                if user.is_empty() {
                    self.set_message("enter a user name", true);
                    return;
                }
                self.message = None;
                self.log_event(format!("session request for {user}"), false);
                self.call(move |g| g.create_session(&user));
            }
            Phase::Prompt => {
                let answer = std::mem::take(&mut self.input);
                self.log_event("verifying credentials", false);
                self.call(move |g| g.respond(Some(answer)));
            }
            _ => {}
        }
    }

    fn cancel(&mut self) {
        self.input.clear();
        self.phase = Phase::PickUser;
        self.message = None;
        self.call(|g| {
            g.cancel()?;
            Ok(Step::Failed("cancelled".into()))
        });
    }

    fn handle_step(&mut self, step: Result<Step>) {
        match step {
            Ok(Step::Prompt { message, secret }) => {
                let what = message.trim().trim_end_matches(':').to_lowercase();
                self.log_event(format!("challenge: {what}"), false);
                self.phase = Phase::Prompt;
                self.prompt = message;
                self.secret = secret;
                self.input.clear();
            }
            Ok(Step::Info { message, error }) => {
                self.set_message(message, error);
                // Info messages need an empty answer to continue the conversation.
                self.call(|g| g.respond(None));
            }
            Ok(Step::Success) => {
                self.phase = Phase::Starting;
                self.set_message("starting session…", false);
                let cmd = self.session_command();
                let env = self.session_env();
                let state = State {
                    last_user: self.current_user(),
                    last_session: self
                        .sessions
                        .get(self.session_idx)
                        .map(|s| s.id.clone())
                        .unwrap_or_default(),
                };
                config::save_state(&self.cfg.state_file, &state);
                info!(cmd = ?cmd, "starting session");
                self.call(move |g| {
                    g.start_session(cmd, env)?;
                    Ok(Step::Success)
                });
                // The second Success (from start_session) ends the greeter.
                self.exit_code = 0;
            }
            Ok(Step::Failed(msg)) => {
                if msg != "cancelled" {
                    self.set_message(msg, true);
                }
                self.phase = Phase::PickUser;
                self.input.clear();
            }
            Err(e) => {
                error!("greetd: {e:#}");
                self.set_message(format!("greetd error: {e}"), true);
                self.phase = Phase::PickUser;
            }
        }
    }

    fn on_greetd(&mut self, g: Greetd, step: Result<Step>) {
        self.greetd = Some(g);
        if self.phase == Phase::Starting {
            match step {
                Ok(_) => {
                    info!("session started; exiting greeter");
                    self.quit = true;
                }
                Err(e) => {
                    self.set_message(format!("cannot start session: {e}"), true);
                    self.phase = Phase::PickUser;
                }
            }
            return;
        }
        self.handle_step(step);
    }

    fn power(&mut self, action: system::LogindAction) {
        if let Err(e) = system::power::logind(action) {
            warn!("power action failed: {e:#}");
            self.set_message(format!("power action failed: {e}"), true);
        }
    }

    fn key(&mut self, key: &KeyInput) -> bool {
        if !key.pressed {
            return false;
        }
        self.caps_lock = key.modifiers.caps_lock;
        match key.keysym {
            ks::KEY_Return | ks::KEY_KP_Enter => self.submit(),
            ks::KEY_Escape => {
                if self.phase == Phase::Prompt {
                    self.cancel();
                } else {
                    self.input.clear();
                    self.username_input.clear();
                    self.message = None;
                }
            }
            ks::KEY_Up if self.phase == Phase::PickUser => {
                self.user_idx = self.user_idx.saturating_sub(1)
            }
            ks::KEY_Down if self.phase == Phase::PickUser => {
                self.user_idx = (self.user_idx + 1).min(self.users.len().saturating_sub(1))
            }
            ks::KEY_Tab | ks::KEY_ISO_Left_Tab if !self.sessions.is_empty() => {
                let n = self.sessions.len();
                self.session_idx = if key.modifiers.shift || key.keysym == ks::KEY_ISO_Left_Tab {
                    (self.session_idx + n - 1) % n
                } else {
                    (self.session_idx + 1) % n
                };
            }
            ks::KEY_F2 if self.cfg.power_buttons => self.power(system::LogindAction::Suspend),
            ks::KEY_F3 if self.cfg.power_buttons => self.power(system::LogindAction::Reboot),
            ks::KEY_F4 if self.cfg.power_buttons => self.power(system::LogindAction::PowerOff),
            ks::KEY_BackSpace => {
                let target = self.active_input();
                if key.modifiers.ctrl {
                    target.clear();
                } else {
                    target.pop();
                }
            }
            _ => {
                if let Some(t) = key.text.as_deref() {
                    if !key.modifiers.ctrl
                        && !key.modifiers.alt
                        && !t.chars().any(|c| c.is_control())
                    {
                        self.active_input().push_str(t);
                    }
                }
            }
        }
        true
    }

    fn active_input(&mut self) -> &mut String {
        if self.phase == Phase::PickUser && !(self.cfg.show_users && !self.users.is_empty()) {
            &mut self.username_input
        } else {
            &mut self.input
        }
    }

    fn click(&mut self, id: u32) {
        match id {
            screen::HIT_SUBMIT => self.submit(),
            screen::HIT_SESSION_PREV => {
                let n = self.sessions.len().max(1);
                self.session_idx = (self.session_idx + n - 1) % n;
            }
            screen::HIT_SESSION_NEXT => {
                let n = self.sessions.len().max(1);
                self.session_idx = (self.session_idx + 1) % n;
            }
            screen::HIT_SUSPEND => self.power(system::LogindAction::Suspend),
            screen::HIT_REBOOT => self.power(system::LogindAction::Reboot),
            screen::HIT_POWEROFF => self.power(system::LogindAction::PowerOff),
            i if i >= screen::HIT_USER => {
                let idx = (i - screen::HIT_USER) as usize;
                if idx < self.users.len() && self.phase == Phase::PickUser {
                    if self.user_idx == idx {
                        self.submit();
                    } else {
                        self.user_idx = idx;
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    match run(cli) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            error!("fatal: {e:#}");
            eprintln!("edex-greeter: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<i32> {
    let cfg = config::load(&cli.config);
    let share = settings_share_dir();
    let themes = ::ui::theme::load_themes(&[share.join("themes").as_path()]);
    let theme = themes
        .get(&cfg.theme)
        .cloned()
        .unwrap_or_else(::ui::theme::builtin_tron);
    let themes_all = themes;
    let mut gpu = GpuContext::new(Some("JetBrainsMono Nerd Font".into()));
    let metrics = gpu.metrics(cfg.font_size, cfg.font_size);

    let (mut event_loop, mut platform): (EventLoop<'static, Platform<Event>>, Platform<Event>) =
        Platform::new()?;
    let (tx, rx) = channel::channel::<Event>();
    platform
        .loop_handle
        .insert_source(rx, |ev, _, p: &mut Platform<Event>| {
            if let channel::Event::Msg(e) = ev {
                p.push_app_event(e);
            }
        })
        .map_err(|e| anyhow::anyhow!("insert channel: {e}"))?;
    platform
        .loop_handle
        .insert_source(
            Timer::from_duration(Duration::from_millis(40)),
            |_, _, p: &mut Platform<Event>| {
                p.push_app_event(Event::Tick);
                TimeoutAction::ToDuration(Duration::from_millis(40))
            },
        )
        .map_err(|e| anyhow::anyhow!("insert timer: {e}"))?;

    let state = config::load_state(&cfg.state_file);
    let users = users::load(std::path::Path::new("/etc/passwd"), cfg.min_uid);
    let sessions = sessions::scan(&sessions::default_dirs());
    let user_idx = users
        .iter()
        .position(|u| u.name == state.last_user)
        .unwrap_or(0);
    let session_idx = sessions
        .iter()
        .position(|s| s.id == state.last_session)
        .or_else(|| sessions.iter().position(|s| s.id == cfg.default_session))
        .unwrap_or(0);
    let greetd = if cli.demo {
        None
    } else {
        match Greetd::from_env() {
            Ok(g) => Some(g),
            Err(e) => {
                warn!("{e:#}; falling back to demo mode");
                None
            }
        }
    };
    let demo = greetd.is_none();
    let mut g = Greeter {
        cfg,
        theme,
        themes: themes_all,
        themed_for: None,
        stats: stats::Stats::new(),
        log: std::collections::VecDeque::new(),
        stats_at: Instant::now(),
        metrics,
        users,
        sessions,
        user_idx,
        session_idx,
        username_input: state.last_user.clone(),
        input: String::new(),
        secret: true,
        prompt: String::new(),
        message: if demo {
            Some(("demo mode (no greetd socket)".into(), false))
        } else {
            None
        },
        phase: Phase::PickUser,
        caps_lock: false,
        hostname: std::fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "edex".into()),
        clock: String::new(),
        date: String::new(),
        started: Instant::now(),
        greetd,
        demo,
        tx,
        quit: false,
        exit_code: 0,
    };
    g.log_event(
        if g.demo {
            "demo mode: greetd not connected".to_string()
        } else {
            "greetd link established".to_string()
        },
        false,
    );
    g.log_event(
        format!("{} users, {} sessions", g.users.len(), g.sessions.len()),
        false,
    );
    g.log_event("awaiting credentials", false);
    g.sync_user_theme();
    g.tick_clock();

    let window: SurfaceId = platform
        .create_window("eDEX login", "edex-greeter", true)
        .context("creating the greeter window")?;
    let mut renderer: Option<SurfaceRenderer> = None;
    let mut hits = ::ui::HitMap::default();
    let mut dirty = true;
    let mut last_anim: u64 = 0;
    let mut frames: u64 = 0;
    let mut pointer = (0.0f64, 0.0f64);
    let smoke = cli.smoke_test.map(Duration::from_secs);

    while !g.quit && !platform.should_exit() {
        platform.dispatch(&mut event_loop, Some(Duration::from_millis(50)))?;
        for ev in platform.drain_events() {
            match ev {
                PlatformEvent::Configure {
                    surface,
                    width,
                    height,
                    scale,
                } if surface == window => {
                    let (bw, bh) = platform.buffer_size(surface).unwrap_or((width, height));
                    match renderer.as_mut() {
                        Some(r) => r.resize(&gpu, bw, bh, scale as f32),
                        None => {
                            let handles = platform.raw_handles(surface)?;
                            renderer = Some(SurfaceRenderer::new(
                                &mut gpu,
                                handles,
                                bw,
                                bh,
                                scale as f32,
                            )?);
                        }
                    }
                    dirty = true;
                }
                PlatformEvent::ScaleChanged { surface, scale } if surface == window => {
                    if let Some(r) = renderer.as_mut() {
                        let (bw, bh) = platform.buffer_size(surface).unwrap_or(r.size());
                        r.resize(&gpu, bw, bh, scale as f32);
                    }
                    dirty = true;
                }
                PlatformEvent::Frame { .. } => dirty = true,
                PlatformEvent::Key { key, .. } => {
                    if g.key(&key) {
                        dirty = true;
                    }
                }
                PlatformEvent::ModifiersChanged { modifiers } => {
                    g.caps_lock = modifiers.caps_lock;
                    dirty = true;
                }
                PlatformEvent::PointerMotion { x, y, .. }
                | PlatformEvent::PointerEnter { x, y, .. } => pointer = (x, y),
                PlatformEvent::PointerButton {
                    button: b,
                    pressed: true,
                    x,
                    y,
                    ..
                } if b == button::LEFT => {
                    pointer = (x, y);
                    if let ::ui::HitTarget::OverlayItem(id) = hits.resolve(x as f32, y as f32) {
                        g.click(id);
                        dirty = true;
                    }
                }
                PlatformEvent::Closed { .. } => g.quit = true,
                PlatformEvent::App(Event::Greetd(boxed)) => {
                    let (gd, step) = *boxed;
                    g.on_greetd(gd, step);
                    dirty = true;
                }
                PlatformEvent::App(Event::Tick) => {
                    // The scan line animates at ~12 fps; otherwise redraw only on changes.
                    let frame = (g.started.elapsed().as_millis() / 80) as u64;
                    if g.tick() || (g.cfg.animations && frame != last_anim) {
                        dirty = true;
                    }
                    last_anim = frame;
                    if let Some(s) = smoke {
                        if g.started.elapsed() >= s {
                            g.quit = true;
                        }
                    }
                }
                _ => {}
            }
        }
        let _ = pointer;
        if dirty && platform.is_configured(window) && !platform.frame_pending(window) {
            if let (Some(r), Some((w, h))) = (renderer.as_mut(), platform.logical_size(window)) {
                let rendered = screen::render(&g, w as f32, h as f32);
                hits = rendered.hits;
                if platform.request_frame(window) {
                    match r.render(&mut gpu, &rendered.scene) {
                        Ok(true) => {
                            frames += 1;
                            dirty = false;
                        }
                        Ok(false) => platform.commit(window),
                        Err(e) => {
                            error!("render: {e:#}");
                            platform.commit(window);
                        }
                    }
                }
            }
        }
    }
    if smoke.is_some() {
        println!(
            "{}",
            serde_json::json!({"frames": frames, "users": g.users.len(), "sessions": g.sessions.len(), "demo": g.demo})
        );
        return Ok(if frames >= 3 { 0 } else { 1 });
    }
    Ok(g.exit_code)
}

fn settings_share_dir() -> PathBuf {
    std::env::var_os("EDEX_SHARE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/edex-de"))
}
