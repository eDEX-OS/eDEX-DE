//! The shell application: owns every subsystem and drives the calloop loop.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    os::fd::AsFd,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use calloop::{
    channel,
    generic::Generic,
    timer::{TimeoutAction, Timer},
    EventLoop, Interest, Mode,
};
use hypr::{events::EventStream, HyprSocket, HyprState};
use ipc::IpcServer;
use launcher::{AppEntry, AppSearch, LaunchHistory};
use notifications::{server::ServerEvent, NotificationServer, NotificationStore};
use platform::{Edge, LayerSpec, OutputId, Platform, PlatformEvent, SurfaceId, SurfaceRole};
use renderer::{GpuContext, SurfaceRenderer};
use settings::{Config, ConfigWatcher};
use sysmon::{PrivacyProbe, SysmonCollector};
use system::{RealRunner, SysReply, SysRequest, SystemBackend};
use terminal::{TermEvent, TerminalConfig, TerminalTabs};
use tracing::{debug, error, info, warn};
use ui::{
    hit::HitMap,
    layout::PanelLayout,
    overlays::toasts::{required_height, TOAST_W},
    panels::terminal_view,
    shell::{osd_expired, render_canvas, render_overlay, render_strip, render_toasts},
    state::OverlayKind,
    theme::{builtin_tron, load_themes},
    ShellState, Theme,
};

use crate::events::{AppEvent, Tick};

pub struct RunOptions {
    pub config_path: PathBuf,
    pub no_hypr: bool,
    pub smoke: Option<Duration>,
}

/// Everything the shell draws on one output.
pub struct OutputShell {
    pub output: OutputId,
    pub name: String,
    pub canvas: SurfaceId,
    pub reservers: [SurfaceId; 4],
    pub overlay: Option<SurfaceId>,
    pub toast: Option<SurfaceId>,
    /// Tab strip bar (Top layer) and the logical rect it currently occupies.
    pub strip: Option<(SurfaceId, ui::geometry::Rect)>,
    pub layout: Option<PanelLayout>,
    pub size: (u32, u32),
}

/// Latest backend states used to build the settings and privacy forms.
#[derive(Default)]
pub struct SysCache {
    pub audio: system::audio::AudioState,
    pub brightness: system::brightness::BrightnessState,
    pub network: system::network::NetworkState,
    pub bluetooth: system::bluetooth::BluetoothState,
    pub power: system::power::PowerState,
    pub users: Vec<system::users::UserInfo>,
    pub services_system: system::services::ServicesState,
    pub services_user: system::services::ServicesState,
    pub display: system::display::DisplayState,
    pub layouts: Vec<system::input::LayoutInfo>,
    pub privacy: system::privacy::PrivacyState,
    pub fprint: system::fprint::FprintState,
    pub fprint_progress: Option<String>,
    pub about: system::about::AboutInfo,
    pub tailscale_login_url: Option<String>,
    pub last_error: Option<String>,
}

pub type Loop = EventLoop<'static, Platform<AppEvent>>;

pub struct App {
    pub opts: RunOptions,
    pub config: Config,
    pub watcher: Option<ConfigWatcher>,
    pub themes: BTreeMap<String, Theme>,
    pub gpu: GpuContext,
    pub renderers: HashMap<SurfaceId, SurfaceRenderer>,
    pub dirty: HashSet<SurfaceId>,
    pub hits: HashMap<SurfaceId, HitMap>,
    pub outputs: Vec<OutputShell>,
    pub state: ShellState,
    pub terminal: TerminalTabs,
    pub hypr: Option<HyprSocket>,
    pub hypr_events: Option<EventStream>,
    pub hypr_state: HyprState,
    /// Whether each output's side panels are currently reserved (by output name).
    pub applied_sides: HashMap<String, bool>,
    pub ipc: Option<IpcServer>,
    pub notif_server: Option<NotificationServer>,
    pub store: NotificationStore,
    pub history_path: PathBuf,
    pub sysmon: SysmonCollector,
    pub privacy_probe: PrivacyProbe,
    pub system: SystemBackend,
    pub sys: SysCache,
    pub apps: Vec<AppEntry>,
    pub search: AppSearch,
    pub launch_history: LaunchHistory,
    pub pointer: HashMap<SurfaceId, (f64, f64)>,
    pub last_click: Option<(Instant, f64, f64, u8)>,
    pub button_held: bool,
    pub last_input: Instant,
    /// Last drawn cursor visibility and pulse level: animation ticks redraw only on change.
    last_cursor_visible: bool,
    last_pulse: f32,
    /// (theme, font, size) last exported to Qt/GTK, so unrelated config changes do not rewrite it.
    toolkit_key: Option<(String, String, u32)>,
    pub started: Instant,
    pub quit: bool,
    pub frames: u64,
    pub tx: Arc<Mutex<channel::Sender<AppEvent>>>,
    pub focus_surface: Option<SurfaceId>,
    /// Canvas holding a temporary exclusive keyboard grab until focus arrives.
    focus_grab: Option<SurfaceId>,
    pub install_button: bool,
    pub smoke_toasts_seen: usize,
    pub scratch: crate::forms::settings::SettingsScratch,
    pub pscratch: crate::forms::privacy::PrivacyScratch,
}

impl App {
    pub fn new(platform: &mut Platform<AppEvent>, opts: RunOptions) -> Result<Self> {
        let mut config = settings::load(&opts.config_path);
        config.sanitize();
        let watcher = settings::watch(&opts.config_path)
            .map_err(|e| warn!("config watcher unavailable: {e:#}"))
            .ok();
        let themes = load_all_themes();
        let theme = themes
            .get(&config.appearance.theme)
            .cloned()
            .unwrap_or_else(builtin_tron);
        let font = if config.appearance.font.is_empty() {
            None
        } else {
            Some(config.appearance.font.clone())
        };
        let mut gpu = GpuContext::new(font);
        let metrics = gpu.metrics(config.appearance.font_size, config.terminal.font_size);
        let mut state = ShellState::new(theme, metrics);
        state.boot =
            ui::boot::BootAnimation::new(config.appearance.boot_animation && opts.smoke.is_none());
        state.username = std::env::var("USER").unwrap_or_default();
        state.live_iso = std::path::Path::new("/run/archiso").exists();
        state.filesystem = ui::filesystem::FilesystemPanel::new();

        let (tx, rx) = channel::channel::<AppEvent>();
        platform
            .loop_handle
            .insert_source(rx, |ev, _, p: &mut Platform<AppEvent>| {
                if let channel::Event::Msg(e) = ev {
                    p.push_app_event(e);
                }
            })
            .map_err(|e| anyhow::anyhow!("insert app channel: {e}"))?;
        let tx = Arc::new(Mutex::new(tx));

        // Terminal
        let term_tx = tx.clone();
        let term_sink: Arc<dyn terminal::EventSink> = Arc::new(move |e: TermEvent| {
            if let Ok(t) = term_tx.lock() {
                let _ = t.send(AppEvent::Term(e));
            }
        });
        let terminal = TerminalTabs::new(
            terminal_config(&config),
            term_sink,
            80,
            24,
            metrics.cell_w,
            metrics.cell_h,
        )
        .context("terminal setup")?;

        // Hyprland
        let (hypr, hypr_events, hypr_state) = if opts.no_hypr {
            (None, None, HyprState::unavailable())
        } else {
            connect_hypr(platform)
        };

        // IPC
        let ipc = match IpcServer::bind(ipc::socket_path()) {
            Ok(server) => {
                if let Err(e) =
                    register_readable(platform, server.listener(), AppEvent::IpcReadable)
                {
                    warn!("ipc listener not registered: {e:#}");
                }
                Some(server)
            }
            Err(e) => {
                warn!("ipc unavailable: {e:#}");
                None
            }
        };

        // Notifications
        let ntx = tx.clone();
        let notif_sink: notifications::server::EventSink = Arc::new(move |e: ServerEvent| {
            if let Ok(t) = ntx.lock() {
                let _ = t.send(AppEvent::Notify(e));
            }
        });
        let notif_server = match NotificationServer::start(notif_sink) {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("notification server not started: {e:#}");
                None
            }
        };
        let mut store = NotificationStore::default();
        apply_notification_config(&mut store, &config);
        let history_path = settings::state_dir().join("notifications.json");
        store.load_history(&history_path);

        // System backend
        let stx = tx.clone();
        let sys_sink: Arc<dyn Fn(SysReply) + Send + Sync> = Arc::new(move |r: SysReply| {
            if let Ok(t) = stx.lock() {
                let _ = t.send(AppEvent::Sys(r));
            }
        });
        let system = SystemBackend::spawn(
            Arc::new(RealRunner {
                timeout: Duration::from_secs(20),
            }),
            sys_sink,
        );

        // Launcher
        let apps = launcher::scan_applications();
        let launch_history = LaunchHistory::load(launcher::runner::history_path());

        let mut sysmon = SysmonCollector::new();
        sysmon.refresh();

        let mut app = Self {
            opts,
            config,
            watcher,
            themes,
            gpu,
            renderers: HashMap::new(),
            dirty: HashSet::new(),
            hits: HashMap::new(),
            outputs: Vec::new(),
            state,
            terminal,
            hypr,
            hypr_events,
            hypr_state,
            applied_sides: HashMap::new(),
            ipc,
            notif_server,
            store,
            history_path,
            sysmon,
            privacy_probe: PrivacyProbe::new(5),
            system,
            sys: SysCache {
                // Known before the first (slower) privacy query returns, so the panel never
                // claims Tor or Tailscale are missing while it is still loading.
                privacy: system::privacy::PrivacyState::installed_only(),
                ..SysCache::default()
            },
            apps,
            search: AppSearch::new(),
            launch_history,
            pointer: HashMap::new(),
            last_click: None,
            button_held: false,
            last_input: Instant::now(),
            last_cursor_visible: true,
            focus_grab: None,
            last_pulse: -1.0,
            toolkit_key: None,
            started: Instant::now(),
            quit: false,
            frames: 0,
            tx,
            focus_surface: None,
            install_button: false,
            smoke_toasts_seen: 0,
            scratch: Default::default(),
            pscratch: Default::default(),
        };
        app.apply_config_to_state();
        app.update_hypr_view();
        crate::status::refresh_sysinfo(&mut app);
        crate::status::refresh_privacy(&mut app);
        app.tick_clock();
        app.install_timers(platform)?;
        app.system.send(SysRequest::AudioQuery);
        app.system.send(SysRequest::BrightnessQuery);
        app.system.send(SysRequest::NetworkQuery { rescan: false });
        app.system.send(SysRequest::BluetoothQuery);
        app.system.send(SysRequest::PowerQuery);

        for out in platform.outputs() {
            app.add_output(platform, out.id);
        }
        Ok(app)
    }

    fn install_timers(&mut self, platform: &mut Platform<AppEvent>) -> Result<()> {
        let handle = platform.loop_handle.clone();
        let timers: [(Tick, Duration); 5] = [
            (Tick::Clock, Duration::from_secs(1)),
            (Tick::Sysmon, Duration::from_millis(1000)),
            (Tick::Privacy, Duration::from_secs(3)),
            (Tick::Anim, Duration::from_millis(40)),
            (Tick::Toast, Duration::from_millis(200)),
        ];
        for (tick, period) in timers {
            handle
                .insert_source(
                    Timer::from_duration(period),
                    move |_, _, p: &mut Platform<AppEvent>| {
                        p.push_app_event(AppEvent::Tick(tick));
                        TimeoutAction::ToDuration(period)
                    },
                )
                .map_err(|e| anyhow::anyhow!("insert timer: {e}"))?;
        }
        Ok(())
    }

    // ─── Outputs and surfaces ───────────────────────────────────────────────

    pub fn add_output(&mut self, platform: &mut Platform<AppEvent>, output: OutputId) {
        if self.outputs.iter().any(|o| o.output == output) {
            return;
        }
        if !platform.has_layer_shell() {
            warn!(
                "compositor has no layer shell; cannot draw on output {:?}",
                output
            );
            return;
        }
        let info = platform.output_info(output);
        let name = info
            .as_ref()
            .and_then(|i| i.name.clone())
            .unwrap_or_else(|| format!("output-{}", output.0));
        let (lw, lh) = info
            .map(|i| {
                (
                    i.logical_size.0.max(1) as f32,
                    i.logical_size.1.max(1) as f32,
                )
            })
            .unwrap_or((1280.0, 720.0));
        let layout = PanelLayout::compute(lw, lh, &self.state.metrics, &self.state.layout_cfg);
        let zones = self.zones(&layout, &name);
        self.applied_sides
            .insert(name.clone(), self.reserve_sides_on(&name));
        let canvas = match platform.create_layer_surface(LayerSpec::canvas(output)) {
            Ok(id) => id,
            Err(e) => {
                error!("cannot create canvas on {name}: {e:#}");
                return;
            }
        };
        let mut reservers = [SurfaceId(0); 4];
        for (i, (edge, size)) in [
            (Edge::Top, zones.0),
            (Edge::Bottom, zones.1),
            (Edge::Left, zones.2),
            (Edge::Right, zones.3),
        ]
        .into_iter()
        .enumerate()
        {
            match platform.create_layer_surface(LayerSpec::reserver(output, edge, size.max(1))) {
                Ok(id) => reservers[i] = id,
                Err(e) => warn!("cannot create reserver on {name}: {e:#}"),
            }
        }
        info!(output = %name, %lw, %lh, "shell surfaces created");
        if self.outputs.is_empty() {
            // Start with the terminal focused so the user can type right away.
            platform.grab_keyboard(canvas);
            self.focus_grab = Some(canvas);
        }
        self.outputs.push(OutputShell {
            output,
            name,
            canvas,
            reservers,
            overlay: None,
            toast: None,
            strip: None,
            layout: Some(layout),
            size: (lw as u32, lh as u32),
        });
    }

    pub fn remove_output(&mut self, platform: &mut Platform<AppEvent>, output: OutputId) {
        let Some(pos) = self.outputs.iter().position(|o| o.output == output) else {
            return;
        };
        let shell = self.outputs.remove(pos);
        for id in std::iter::once(shell.canvas)
            .chain(shell.reservers)
            .chain(shell.overlay)
            .chain(shell.toast)
            .chain(shell.strip.map(|(id, _)| id))
        {
            platform.destroy_surface(id);
            self.renderers.remove(&id);
            self.dirty.remove(&id);
            self.hits.remove(&id);
            self.pointer.remove(&id);
        }
        info!(output = %shell.name, "output removed");
    }

    fn zones(&self, layout: &PanelLayout, output: &str) -> (u32, u32, u32, u32) {
        let (t, b, l, r) = layout.reserved_zones();
        if self.reserve_sides_on(output) {
            (t, b, l, r)
        } else {
            (t, b, 0, 0)
        }
    }

    /// Side panels keep their space unless the user hid them or a maximized window on this
    /// output's workspace wants the full width.
    pub fn reserve_sides_on(&self, output: &str) -> bool {
        self.config.layout.reserve_side_panels && !self.hypr_state.maximized_on(output)
    }

    pub fn primary(&self) -> Option<&OutputShell> {
        self.outputs.first()
    }

    pub fn shell_of_surface(&self, id: SurfaceId) -> Option<&OutputShell> {
        self.outputs.iter().find(|o| {
            o.canvas == id
                || o.overlay == Some(id)
                || o.toast == Some(id)
                || o.strip.is_some_and(|(s, _)| s == id)
                || o.reservers.contains(&id)
        })
    }

    /// Recompute the layout of every canvas and push the reserver sizes to the compositor.
    pub fn relayout(&mut self, platform: &mut Platform<AppEvent>) {
        let metrics = self.state.metrics;
        let cfg = self.state.layout_cfg;
        let sides: Vec<bool> = self
            .outputs
            .iter()
            .map(|o| self.reserve_sides_on(&o.name))
            .collect();
        self.applied_sides = self
            .outputs
            .iter()
            .zip(&sides)
            .map(|(o, s)| (o.name.clone(), *s))
            .collect();
        let mut primary_grid = None;
        for (i, shell) in self.outputs.iter_mut().enumerate() {
            let reserve_sides = sides[i];
            let (w, h) = platform.logical_size(shell.canvas).unwrap_or(shell.size);
            shell.size = (w, h);
            let layout = PanelLayout::compute(w as f32, h as f32, &metrics, &cfg);
            let (t, b, l, r) = layout.reserved_zones();
            let (l, r) = if reserve_sides { (l, r) } else { (0, 0) };
            for (id, size) in shell.reservers.iter().zip([t, b, l, r]) {
                if id.0 != 0 {
                    platform.set_reserver_size(*id, size.max(1));
                }
            }
            if i == 0 {
                primary_grid = Some(
                    layout.terminal_grid(&metrics, terminal_view::tab_bar_height(metrics.line)),
                );
            }
            shell.layout = Some(layout);
            self.dirty.insert(shell.canvas);
        }
        if let Some((cols, rows)) = primary_grid {
            if self.terminal.grid_size() != (cols, rows) {
                self.terminal
                    .resize(cols, rows, metrics.cell_w, metrics.cell_h);
            }
        }
        self.update_hypr_view();
        self.sync_strip_surfaces(platform);
    }

    pub fn mark_all_dirty(&mut self) {
        for shell in &self.outputs {
            self.dirty.insert(shell.canvas);
            if let Some(o) = shell.overlay {
                self.dirty.insert(o);
            }
            if let Some(t) = shell.toast {
                self.dirty.insert(t);
            }
        }
    }

    pub fn mark_canvas_dirty(&mut self) {
        for shell in &self.outputs {
            self.dirty.insert(shell.canvas);
            if let Some((id, _)) = shell.strip {
                self.dirty.insert(id);
            }
        }
    }

    /// Create or move each output's tab strip surface to cover its strip (full width on the
    /// primary output while apps have the side panels' space).
    pub fn sync_strip_surfaces(&mut self, platform: &mut Platform<AppEvent>) {
        let wide = self.state.wide_tab_strip;
        for (i, shell) in self.outputs.iter_mut().enumerate() {
            let Some(layout) = shell.layout.as_ref() else {
                continue;
            };
            let rect = if wide && i == 0 {
                ui::layout::wide_tab_strip(layout)
            } else {
                layout.tab_strip
            };
            let (x, y) = (rect.x.round() as i32, rect.y.round() as i32);
            let (w, h) = (
                rect.w.round().max(1.0) as u32,
                rect.h.round().max(1.0) as u32,
            );
            match shell.strip {
                Some((id, old)) if old == rect => {
                    self.dirty.insert(id);
                }
                Some((id, _)) => {
                    platform.set_layer_margin(id, (y, 0, 0, x));
                    platform.set_layer_size(id, w, h);
                    shell.strip = Some((id, rect));
                    self.dirty.insert(id);
                }
                None => {
                    match platform.create_layer_surface(LayerSpec::strip(shell.output, x, y, w, h))
                    {
                        Ok(id) => {
                            shell.strip = Some((id, rect));
                            self.dirty.insert(id);
                        }
                        Err(e) => warn!("cannot create the tab strip surface: {e:#}"),
                    }
                }
            }
        }
    }

    pub fn mark_overlay_dirty(&mut self) {
        for shell in &self.outputs {
            if let Some(o) = shell.overlay {
                self.dirty.insert(o);
            }
        }
    }

    pub fn mark_toast_dirty(&mut self) {
        for shell in &self.outputs {
            if let Some(t) = shell.toast {
                self.dirty.insert(t);
            }
        }
    }

    /// Output the pointer was last seen on, else the primary output.
    pub fn active_output_index(&self) -> Option<usize> {
        if let Some(fs) = self.focus_surface {
            if let Some(i) = self
                .outputs
                .iter()
                .position(|o| o.canvas == fs || o.overlay == Some(fs))
            {
                return Some(i);
            }
        }
        for id in self.pointer.keys() {
            if let Some(i) = self
                .outputs
                .iter()
                .position(|o| o.canvas == *id || o.overlay == Some(*id))
            {
                return Some(i);
            }
        }
        if self.outputs.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    // ─── Overlays ───────────────────────────────────────────────────────────

    pub fn open_overlay(&mut self, platform: &mut Platform<AppEvent>, kind: OverlayKind) {
        if self.state.overlay == Some(kind) {
            return;
        }
        if self.state.overlay.is_some() {
            self.close_overlay(platform);
        }
        let Some(idx) = self.active_output_index() else {
            warn!("no output to show the overlay on");
            return;
        };
        crate::overlays::prepare(self, kind);
        self.state.overlay = Some(kind);
        let output = self.outputs[idx].output;
        match platform.create_layer_surface(LayerSpec::overlay(output)) {
            Ok(id) => {
                self.outputs[idx].overlay = Some(id);
                self.dirty.insert(id);
            }
            Err(e) => {
                error!("cannot create overlay surface: {e:#}");
                self.state.overlay = None;
            }
        }
    }

    pub fn close_overlay(&mut self, platform: &mut Platform<AppEvent>) {
        self.state.overlay = None;
        self.state.settings.form_state.editing = None;
        self.state.privacy.form_state.editing = None;
        for shell in &mut self.outputs {
            if let Some(id) = shell.overlay.take() {
                platform.destroy_surface(id);
                self.renderers.remove(&id);
                self.dirty.remove(&id);
                self.hits.remove(&id);
                self.pointer.remove(&id);
            }
        }
        self.mark_canvas_dirty();
    }

    pub fn toggle_overlay(&mut self, platform: &mut Platform<AppEvent>, kind: OverlayKind) {
        if self.state.overlay == Some(kind) {
            self.close_overlay(platform);
        } else {
            self.open_overlay(platform, kind);
        }
    }

    /// Create, resize or destroy the toast surface to match the toasts/OSD in the state.
    pub fn sync_toast_surface(&mut self, platform: &mut Platform<AppEvent>) {
        let want = !self.state.toasts.is_empty() || self.state.osd.is_some();
        let Some(idx) = self.active_output_index() else {
            return;
        };
        let line = self.state.metrics.line;
        let height = required_height(&self.state, line).ceil().max(1.0) as u32;
        let width = (TOAST_W + 16.0) as u32;
        let top = self.outputs[idx]
            .layout
            .as_ref()
            .map(|l| l.top_bar.bottom().round() as i32 + 8)
            .unwrap_or(48);
        // Destroy toast surfaces on other outputs.
        for (i, shell) in self.outputs.iter_mut().enumerate() {
            if i != idx || !want {
                if let Some(id) = shell.toast.take() {
                    platform.destroy_surface(id);
                    self.renderers.remove(&id);
                    self.dirty.remove(&id);
                    self.hits.remove(&id);
                }
            }
        }
        if !want {
            return;
        }
        let shell = &mut self.outputs[idx];
        match shell.toast {
            Some(id) => {
                if platform.logical_size(id) != Some((width, height)) {
                    platform.set_layer_size(id, width, height);
                }
                self.dirty.insert(id);
            }
            None => match platform.create_layer_surface(LayerSpec::toast(
                shell.output,
                width,
                height,
                top,
            )) {
                Ok(id) => {
                    shell.toast = Some(id);
                    self.dirty.insert(id);
                }
                Err(e) => warn!("cannot create toast surface: {e:#}"),
            },
        }
    }

    // ─── Rendering ──────────────────────────────────────────────────────────

    pub fn render_dirty(&mut self, platform: &mut Platform<AppEvent>) {
        let ids: Vec<SurfaceId> = self.dirty.iter().copied().collect();
        for id in ids {
            self.render_surface(platform, id);
        }
    }

    pub fn render_surface(&mut self, platform: &mut Platform<AppEvent>, id: SurfaceId) {
        if !platform.is_configured(id) || platform.frame_pending(id) {
            return;
        }
        let Some(role) = platform.surface_role(id) else {
            self.dirty.remove(&id);
            return;
        };
        let Some((w, h)) = platform.logical_size(id) else {
            return;
        };
        if !self.renderers.contains_key(&id) {
            let handles = match platform.raw_handles(id) {
                Ok(h) => h,
                Err(e) => {
                    error!("no raw handles for surface {:?}: {e:#}", id);
                    self.dirty.remove(&id);
                    return;
                }
            };
            let scale = platform.scale(id) as f32;
            let (bw, bh) = platform.buffer_size(id).unwrap_or((w, h));
            match SurfaceRenderer::new(&mut self.gpu, handles, bw, bh, scale) {
                Ok(r) => {
                    self.renderers.insert(id, r);
                }
                Err(e) => {
                    error!("cannot create GPU surface: {e:#}");
                    self.dirty.remove(&id);
                    return;
                }
            }
        }
        self.state.now = Instant::now();
        let (wf, hf) = (w as f32, h as f32);
        let rendered = match role {
            SurfaceRole::Canvas => {
                let cursor_visible = self.cursor_visible();
                let focused =
                    self.state.shell_focused && self.state.focus == ui::state::PanelFocus::Terminal;
                let palette = terminal::Palette::from_theme(&self.state.theme);
                self.state.terminal.frame =
                    self.terminal
                        .frame(&palette, cursor_visible, focused, self.state.now);
                self.state.terminal.tabs = self.terminal.tab_infos();
                self.state.terminal.active = self.terminal.active_index();
                let (r, layout) = render_canvas(&self.state, wf, hf);
                if let Some(shell) = self.outputs.iter_mut().find(|o| o.canvas == id) {
                    shell.layout = Some(layout);
                }
                Some(r)
            }
            SurfaceRole::Overlay => render_overlay(&self.state, wf, hf),
            SurfaceRole::Toast => render_toasts(&self.state, wf, hf),
            SurfaceRole::Strip => {
                self.state.terminal.tabs = self.terminal.tab_infos();
                self.state.terminal.active = self.terminal.active_index();
                Some(render_strip(&self.state, wf, hf))
            }
            SurfaceRole::Reserver(_) | SurfaceRole::Window => None,
        };
        let Some(rendered) = rendered else {
            self.dirty.remove(&id);
            return;
        };
        self.hits.insert(id, rendered.hits);
        if !platform.request_frame(id) {
            return;
        }
        let renderer = self.renderers.get_mut(&id).expect("renderer");
        match renderer.render(&mut self.gpu, &rendered.scene) {
            Ok(true) => {
                self.frames += 1;
                self.dirty.remove(&id);
            }
            Ok(false) => {
                // Surface lost/outdated: keep dirty and retry on the next frame.
                platform.commit(id);
            }
            Err(e) => {
                error!("render failed on {:?}: {e:#}", id);
                platform.commit(id);
                self.dirty.remove(&id);
            }
        }
    }

    /// Move keyboard focus to the shell canvas (from an app window or at login). Apps tile over
    /// the terminal, so when this workspace has windows switch to an empty one first, where the
    /// terminal is visible (SUPER+1..9 goes back).
    pub fn focus_shell(&mut self, platform: &mut Platform<AppEvent>) {
        if let Some(h) = &self.hypr {
            if h.active_workspace_windows().unwrap_or(0) > 0 {
                if let Err(e) = h.focus_empty_workspace() {
                    warn!("switching to an empty workspace: {e:#}");
                }
            }
        }
        if let Some(canvas) = self.primary().map(|s| s.canvas) {
            if self.focus_surface != Some(canvas) {
                platform.grab_keyboard(canvas);
                self.focus_grab = Some(canvas);
            }
        }
    }

    /// Show the terminal in the centre: windows tiled over it are minimized into tabs (click a
    /// tab to bring one back), then the shell takes keyboard focus.
    pub fn bring_terminal_forward(&mut self, platform: &mut Platform<AppEvent>) {
        if self.state.apps_cover_terminal {
            let covering: Vec<String> = self
                .state
                .windows
                .iter()
                .filter(|w| !w.minimized && !w.floating)
                .map(|w| w.address.clone())
                .collect();
            for address in covering {
                crate::input::window_action(self, &address, crate::input::WindowAction::Minimize);
            }
        }
        if let Some(canvas) = self.primary().map(|s| s.canvas) {
            if self.focus_surface != Some(canvas) {
                platform.grab_keyboard(canvas);
                self.focus_grab = Some(canvas);
            }
        }
    }

    /// ranger in a new terminal tab (the eDEX file manager), in `path` or the file panel's
    /// directory; the tab closes when ranger quits.
    pub fn open_files(
        &mut self,
        platform: &mut Platform<AppEvent>,
        path: Option<std::path::PathBuf>,
    ) -> std::result::Result<(), String> {
        let dir = path.unwrap_or_else(|| self.state.filesystem.cwd.clone());
        let (dir, select) = if dir.is_dir() {
            (dir, None)
        } else {
            (
                dir.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
                Some(dir),
            )
        };
        // Borders and column ratios closer to the eDEX look; colours follow the terminal palette,
        // which follows the theme.
        let mut args = vec![
            "--cmd=set draw_borders both".to_string(),
            "--cmd=set column_ratios 1,3,3".to_string(),
        ];
        if let Some(file) = select {
            args.push(format!("--selectfile={}", file.display()));
        } else {
            args.push(dir.display().to_string());
        }
        if !launcher::desktop::executable_exists("ranger") {
            return Err("ranger is not installed".into());
        }
        self.terminal
            .new_command_tab("ranger", args, Some(dir))
            .map_err(|e| format!("{e:#}"))?;
        self.state.focus = ui::state::PanelFocus::Terminal;
        self.bring_terminal_forward(platform);
        self.mark_canvas_dirty();
        Ok(())
    }

    fn cursor_visible(&self) -> bool {
        if !self.config.terminal.cursor_blink || !self.terminal.cursor_blinks() {
            return true;
        }
        let since = self.state.now.duration_since(self.last_input).as_millis();
        since < 500 || (since / 530).is_multiple_of(2)
    }

    // ─── Config ─────────────────────────────────────────────────────────────

    pub fn apply_config_to_state(&mut self) {
        let c = &self.config;
        let theme = self
            .themes
            .get(&c.appearance.theme)
            .cloned()
            .unwrap_or_else(builtin_tron);
        self.state.theme = theme;
        self.state.theme.glow = c.appearance.border_glow;
        if self.opts.smoke.is_none() {
            publish_greeter_theme(&self.state.theme.name);
        }
        if c.appearance.theme_apps && self.opts.smoke.is_none() {
            let font = if c.appearance.font.is_empty() {
                "JetBrainsMono Nerd Font".to_string()
            } else {
                c.appearance.font.clone()
            };
            let pt = ((c.appearance.font_size * 0.75).round() as u32).clamp(8, 16);
            let key = (self.state.theme.name.clone(), font.clone(), pt);
            if self.toolkit_key.as_ref() != Some(&key) {
                let xdg = |var: &str, fallback: &str| {
                    std::env::var_os(var)
                        .filter(|v| !v.is_empty())
                        .map(std::path::PathBuf::from)
                        .or_else(|| {
                            std::env::var_os("HOME")
                                .map(|h| std::path::PathBuf::from(h).join(fallback))
                        })
                };
                if let (Some(config), Some(data)) = (
                    xdg("XDG_CONFIG_HOME", ".config"),
                    xdg("XDG_DATA_HOME", ".local/share"),
                ) {
                    crate::toolkits::apply(&config, &data, &self.state.theme, &font, pt);
                }
                self.toolkit_key = Some(key);
            }
        }
        let font = if c.appearance.font.is_empty() {
            None
        } else {
            Some(c.appearance.font.clone())
        };
        if self.gpu.font_family() != font.as_deref() {
            self.gpu.set_font_family(font);
        }
        self.state.metrics = self
            .gpu
            .metrics(c.appearance.font_size, c.terminal.font_size);
        self.state.layout_cfg = ui::LayoutConfig {
            fs_split: c.layout.fs_split,
            sysinfo_split: c.layout.sysinfo_split,
            keyboard_visible: c.appearance.keyboard_visible,
        };
        self.state.scanlines = c.appearance.scanlines;
        // A software renderer redraws the whole canvas on the CPU: no continuous border pulse there.
        self.state.animations = c.appearance.animations && !self.gpu.is_software();
        self.terminal.set_config(terminal_config(c));
        apply_notification_config(&mut self.store, c);
        self.state.notifications.dnd = self.store.dnd;
        self.state.status.dnd = self.store.dnd;
    }

    /// Persist the config, apply it to the shell and export the Hyprland side.
    pub fn commit_config(&mut self, platform: &mut Platform<AppEvent>, export_hypr: bool) {
        self.config.sanitize();
        if let Err(e) = settings::save(&self.opts.config_path, &self.config) {
            warn!("cannot save config: {e:#}");
            self.state.settings.status = Some(format!("save failed: {e}"));
        }
        self.apply_config_to_state();
        self.relayout(platform);
        if export_hypr {
            self.export_hypr();
        }
        self.mark_all_dirty();
    }

    pub fn export_hypr(&mut self) {
        let dir = settings::paths::hypr_config_dir();
        match settings::hypr_export::export(&self.config, &dir, "hyprlock") {
            Ok(()) => {
                if let Some(h) = &self.hypr {
                    if let Err(e) = h.reload() {
                        warn!("hyprctl reload failed: {e:#}");
                    }
                }
            }
            Err(e) => warn!("cannot export Hyprland config: {e:#}"),
        }
    }

    pub fn reload_config(&mut self, platform: &mut Platform<AppEvent>) {
        let mut c = settings::load(&self.opts.config_path);
        c.sanitize();
        if c != self.config {
            info!("configuration reloaded");
            self.config = c;
            self.apply_config_to_state();
            self.relayout(platform);
            self.mark_all_dirty();
            if self.state.overlay == Some(OverlayKind::Settings) {
                crate::forms::settings::rebuild(self);
            }
        }
    }

    pub fn set_theme(&mut self, platform: &mut Platform<AppEvent>, name: &str) -> bool {
        if !self.themes.contains_key(name) {
            return false;
        }
        self.config.appearance.theme = name.to_string();
        self.commit_config(platform, false);
        true
    }

    // ─── Hyprland view ──────────────────────────────────────────────────────

    pub fn update_hypr_view(&mut self) {
        self.state.hypr_connected = self.hypr_state.connected;
        self.state.workspaces = self.hypr_state.workspace_strip(None);
        self.state.active_window = self
            .hypr_state
            .active_window
            .as_ref()
            .map(|(_, t)| t.clone());
        self.state.windows = self.hypr_state.window_tabs();
        let primary = self.primary().map(|o| o.name.clone()).unwrap_or_default();
        let tiled = self.hypr_state.tiled_on(&primary);
        self.state.apps_cover_terminal = tiled;
        self.state.wide_tab_strip = tiled && !self.reserve_sides_on(&primary);
        if self.hypr_state.connected {
            self.state.kb_layout = self.hypr_state.short_layout();
        } else {
            self.state.kb_layout = self.config.input.kb_layout.clone();
        }
        self.mark_canvas_dirty();
    }

    pub fn drain_hypr(&mut self, platform: &mut Platform<AppEvent>) {
        let Some(stream) = self.hypr_events.as_mut() else {
            return;
        };
        let mut events = Vec::new();
        match stream.drain(&mut events) {
            Ok(true) => {}
            Ok(false) => {
                warn!("Hyprland event socket closed");
                self.hypr_events = None;
                self.hypr_state.connected = false;
            }
            Err(e) => {
                warn!("Hyprland events: {e:#}");
            }
        }
        let mut resync = false;
        for ev in &events {
            if self.hypr_state.apply(ev) {
                resync = true;
            }
            if let hypr::HyprEvent::Bell(_) = ev {
                self.state.terminal.frame.bell = true;
            }
        }
        if resync {
            if let Some(sock) = &self.hypr {
                self.hypr_state.resync(sock);
            }
        }
        if !events.is_empty() {
            self.update_hypr_view();
            // A window was maximized or restored: give it (or take back) the side panels' space.
            let changed = self
                .outputs
                .iter()
                .any(|o| self.applied_sides.get(&o.name) != Some(&self.reserve_sides_on(&o.name)));
            if changed {
                self.relayout(platform);
            } else {
                self.sync_strip_surfaces(platform);
            }
        }
    }

    // ─── Timers ─────────────────────────────────────────────────────────────

    pub fn tick_clock(&mut self) {
        let now = chrono::Local::now();
        self.state.clock = now.format("%H:%M:%S").to_string();
        self.state.date = now.format("%a %d %b %Y").to_string();
        if self.state.hostname.is_empty() || self.state.hostname == "edex" {
            self.state.hostname = self.sysmon.snapshot().hostname.clone();
        }
        self.mark_canvas_dirty();
    }

    pub fn handle_tick(&mut self, platform: &mut Platform<AppEvent>, tick: Tick) {
        match tick {
            Tick::Clock => {
                self.tick_clock();
                if self.watcher.as_ref().is_some_and(|w| w.changed()) {
                    self.reload_config(platform);
                }
                if let Some(smoke) = self.opts.smoke {
                    if self.started.elapsed() >= smoke {
                        self.quit = true;
                    }
                }
            }
            Tick::Sysmon => {
                crate::status::refresh_sysinfo(self);
                self.mark_canvas_dirty();
            }
            Tick::Privacy => {
                crate::status::refresh_privacy(self);
                if self.state.overlay == Some(OverlayKind::Privacy) {
                    self.system.send(SysRequest::PrivacyQuery);
                }
                self.mark_canvas_dirty();
            }
            Tick::Anim => {
                let now = Instant::now();
                self.state.now = now;
                let mut dirty = false;
                if !self.state.boot.done {
                    self.state.boot.update(now);
                    dirty = true;
                }
                if self.state.animations && self.gpu.is_software() {
                    // The adapter is only known after the first surface is configured.
                    info!("software renderer: border pulse disabled to save CPU");
                    self.state.animations = false;
                    dirty = true;
                }
                let mut pulse_changed = false;
                if self.state.animations {
                    let pulse = self.state.pulse();
                    if pulse != self.last_pulse {
                        self.last_pulse = pulse;
                        pulse_changed = true;
                        dirty = true;
                    }
                }
                let cursor = self.cursor_visible();
                if cursor != self.last_cursor_visible {
                    self.last_cursor_visible = cursor;
                    dirty = true;
                }
                if dirty {
                    self.mark_canvas_dirty();
                    if pulse_changed {
                        self.mark_overlay_dirty();
                    }
                }
            }
            Tick::Toast => {
                let now = Instant::now();
                let expired = self.store.expire(now);
                let mut changed = !expired.is_empty();
                for id in expired {
                    if let Some(s) = &self.notif_server {
                        s.emit_closed(id, notifications::server::CLOSE_EXPIRED);
                    }
                }
                if self.state.osd.is_some() && osd_expired(&self.state, now) {
                    self.state.osd = None;
                    changed = true;
                }
                if changed {
                    crate::status::sync_toasts(self);
                    self.sync_toast_surface(platform);
                }
            }
        }
    }

    // ─── Events ─────────────────────────────────────────────────────────────

    pub fn handle_event(
        &mut self,
        platform: &mut Platform<AppEvent>,
        event: PlatformEvent<AppEvent>,
    ) {
        match event {
            PlatformEvent::OutputAdded(info) => {
                self.add_output(platform, info.id);
            }
            PlatformEvent::OutputChanged(info) => {
                if self.outputs.iter().any(|o| o.output == info.id) {
                    self.relayout(platform);
                }
            }
            PlatformEvent::OutputRemoved(id) => self.remove_output(platform, id),
            PlatformEvent::Configure {
                surface,
                width,
                height,
                scale,
            } => {
                if let Some(r) = self.renderers.get_mut(&surface) {
                    let (bw, bh) = platform.buffer_size(surface).unwrap_or((width, height));
                    r.resize(&self.gpu, bw, bh, scale as f32);
                }
                if platform.surface_role(surface) == Some(SurfaceRole::Canvas) {
                    self.relayout(platform);
                }
                if !matches!(
                    platform.surface_role(surface),
                    Some(SurfaceRole::Reserver(_))
                ) {
                    self.dirty.insert(surface);
                }
            }
            PlatformEvent::ScaleChanged { surface, scale } => {
                if let Some(r) = self.renderers.get_mut(&surface) {
                    let (w, h) = platform.buffer_size(surface).unwrap_or(r.size());
                    r.resize(&self.gpu, w, h, scale as f32);
                }
                self.dirty.insert(surface);
            }
            PlatformEvent::Frame { surface } => {
                if self.dirty.contains(&surface) {
                    self.render_surface(platform, surface);
                }
            }
            PlatformEvent::KeyboardEnter { surface } => {
                self.focus_surface = Some(surface);
                if self.focus_grab == Some(surface) {
                    platform.release_keyboard_grab(surface);
                    self.focus_grab = None;
                }
                if platform.surface_role(surface) == Some(SurfaceRole::Canvas) {
                    self.state.shell_focused = true;
                }
                self.mark_canvas_dirty();
            }
            PlatformEvent::KeyboardLeave { surface } => {
                if self.focus_surface == Some(surface) {
                    self.focus_surface = None;
                }
                if platform.surface_role(surface) == Some(SurfaceRole::Canvas) {
                    self.state.shell_focused = false;
                    self.state.keyboard.pressed.clear();
                }
                self.mark_canvas_dirty();
            }
            PlatformEvent::Key { surface, key } => {
                self.last_input = Instant::now();
                crate::input::key(self, platform, surface, key);
            }
            PlatformEvent::ModifiersChanged { modifiers } => {
                let kb = &mut self.state.keyboard;
                kb.shift = modifiers.shift;
                kb.ctrl = modifiers.ctrl;
                kb.alt = modifiers.alt;
                kb.logo = modifiers.logo;
                kb.caps_lock = modifiers.caps_lock;
                self.mark_canvas_dirty();
            }
            PlatformEvent::PointerEnter { surface, x, y } => {
                self.pointer.insert(surface, (x, y));
                crate::input::pointer_motion(self, platform, surface, x, y);
            }
            PlatformEvent::PointerLeave { surface } => {
                self.pointer.remove(&surface);
                self.state.keyboard.hover = None;
                self.state.resize.hover = None;
                self.mark_canvas_dirty();
            }
            PlatformEvent::PointerMotion { surface, x, y } => {
                self.pointer.insert(surface, (x, y));
                crate::input::pointer_motion(self, platform, surface, x, y);
            }
            PlatformEvent::PointerButton {
                surface,
                button,
                pressed,
                x,
                y,
            } => {
                self.last_input = Instant::now();
                self.pointer.insert(surface, (x, y));
                crate::input::pointer_button(self, platform, surface, button, pressed, x, y);
            }
            PlatformEvent::PointerAxis {
                surface,
                dx,
                dy,
                x,
                y,
            } => {
                crate::input::pointer_axis(self, platform, surface, dx, dy, x, y);
            }
            PlatformEvent::Closed { surface } => {
                if self.outputs.iter().any(|o| o.canvas == surface) {
                    warn!("canvas closed by the compositor");
                    if let Some(shell) = self.outputs.iter().find(|o| o.canvas == surface) {
                        let out = shell.output;
                        self.remove_output(platform, out);
                    }
                } else if self.outputs.iter().any(|o| o.overlay == Some(surface)) {
                    self.close_overlay(platform);
                }
            }
            PlatformEvent::Paste { text, primary: _ } => {
                if self.state.overlay.is_some() {
                    crate::overlays::paste(self, &text);
                    self.mark_overlay_dirty();
                } else {
                    self.terminal.paste(&text);
                }
            }
            PlatformEvent::App(app_event) => self.handle_app_event(platform, app_event),
        }
    }

    fn handle_app_event(&mut self, platform: &mut Platform<AppEvent>, event: AppEvent) {
        match event {
            AppEvent::Term(e) => {
                match &e {
                    TermEvent::Bell(_) => self.terminal_bell(),
                    TermEvent::ClipboardStore(text) => platform.copy_to_clipboard(text.clone()),
                    TermEvent::ClipboardLoad(..) if self.config.terminal.osc52_read => {
                        platform.request_paste();
                    }
                    _ => {}
                }
                if matches!(e, TermEvent::ColorRequest(..)) {
                    self.terminal
                        .set_palette(terminal::Palette::from_theme(&self.state.theme));
                }
                if self.terminal.handle_event(e) {
                    self.mark_canvas_dirty();
                }
                if self.terminal.active_exited() && self.terminal.len() == 1 {
                    // Keep at least one live shell.
                    self.state.terminal.frame.exited = self.state.terminal.frame.exited.or(Some(0));
                }
            }
            AppEvent::Notify(ev) => crate::status::notification_event(self, platform, ev),
            AppEvent::Sys(reply) => crate::status::system_reply(self, platform, reply),
            AppEvent::HyprReadable => self.drain_hypr(platform),
            AppEvent::IpcReadable => crate::ipc_handler::drain(self, platform),
            AppEvent::Tick(t) => self.handle_tick(platform, t),
        }
    }

    fn terminal_bell(&mut self) {
        match self.config.terminal.bell.as_str() {
            "audible" => {
                for p in [
                    "/usr/share/sounds/freedesktop/stereo/bell.oga",
                    "/usr/share/sounds/freedesktop/stereo/message.oga",
                ] {
                    if std::path::Path::new(p).exists() {
                        let _ = launcher::runner::spawn_detached(
                            &format!("pw-play {p} || paplay {p}"),
                            false,
                        );
                        break;
                    }
                }
            }
            "none" => {}
            _ => {
                self.state.terminal.frame.bell = true;
            }
        }
        self.mark_canvas_dirty();
    }

    pub fn shutdown(&mut self) {
        self.store.save_history(&self.history_path);
    }
}

pub fn terminal_config(c: &Config) -> TerminalConfig {
    TerminalConfig {
        shell: if c.terminal.shell.is_empty() {
            None
        } else {
            Some(c.terminal.shell.clone())
        },
        shell_args: Vec::new(),
        scrollback: c.terminal.scrollback,
        cursor: match c.terminal.cursor.as_str() {
            "underline" => ui::terminal_model::CursorShape::Underline,
            "beam" => ui::terminal_model::CursorShape::Beam,
            _ => ui::terminal_model::CursorShape::Block,
        },
        cursor_blink: c.terminal.cursor_blink,
        osc52_read: c.terminal.osc52_read,
        working_directory: None,
    }
}

fn apply_notification_config(store: &mut NotificationStore, c: &Config) {
    store.dnd = c.notifications.dnd;
    store.default_timeout = Duration::from_millis(c.notifications.timeout_ms.max(500) as u64);
    store.max_visible = c.notifications.max_visible.clamp(1, 10);
    store.muted_apps = c.notifications.muted_apps.clone();
}

pub fn load_all_themes() -> BTreeMap<String, Theme> {
    let system = settings::system_share_dir().join("themes");
    let user = settings::user_theme_dir();
    let mut themes = load_themes(&[system.as_path(), user.as_path()]);
    themes.entry("tron".into()).or_insert_with(builtin_tron);
    themes
}

fn connect_hypr(
    platform: &mut Platform<AppEvent>,
) -> (Option<HyprSocket>, Option<EventStream>, HyprState) {
    let Some(socket) = HyprSocket::from_env() else {
        info!("HYPRLAND_INSTANCE_SIGNATURE not set; running without Hyprland integration");
        return (None, None, HyprState::unavailable());
    };
    let mut state = HyprState::default();
    state.resync(&socket);
    let events = hypr::instance_dir().and_then(|dir| match EventStream::connect(&dir) {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("Hyprland event socket: {e:#}");
            None
        }
    });
    if let Some(stream) = &events {
        if let Err(e) = register_readable(platform, stream.stream(), AppEvent::HyprReadable) {
            warn!("hypr events not registered: {e:#}");
        }
    }
    info!(version = %state.version, monitors = state.monitors.len(), "connected to Hyprland");
    (Some(socket), events, state)
}

/// Wake the loop with `event` whenever `fd` becomes readable (level-triggered on a dup).
fn register_readable<F: AsFd>(
    platform: &mut Platform<AppEvent>,
    fd: &F,
    event: AppEvent,
) -> Result<()>
where
    AppEvent: CloneEvent,
{
    let owned = fd.as_fd().try_clone_to_owned().context("dup fd")?;
    let source = Generic::new(owned, Interest::READ, Mode::Level);
    platform
        .loop_handle
        .insert_source(source, move |_, _, p: &mut Platform<AppEvent>| {
            p.push_app_event(event.clone_event());
            Ok(calloop::PostAction::Continue)
        })
        .map_err(|e| anyhow::anyhow!("insert fd source: {e}"))?;
    Ok(())
}

/// Minimal cloning for the marker events used by fd sources.
pub trait CloneEvent {
    fn clone_event(&self) -> Self;
}

impl CloneEvent for AppEvent {
    fn clone_event(&self) -> Self {
        match self {
            AppEvent::HyprReadable => AppEvent::HyprReadable,
            AppEvent::IpcReadable => AppEvent::IpcReadable,
            AppEvent::Tick(t) => AppEvent::Tick(*t),
            _ => unreachable!("only marker events are cloned"),
        }
    }
}

/// Run the shell until it exits; returns the process exit code.
pub fn run(opts: RunOptions) -> Result<i32> {
    let smoke = opts.smoke;
    let (mut event_loop, mut platform): (Loop, Platform<AppEvent>) = Platform::new()?;
    let mut app = App::new(&mut platform, opts)?;
    let mut last_flush = Instant::now();
    while !app.quit && !platform.should_exit() {
        platform.dispatch(&mut event_loop, Some(Duration::from_millis(50)))?;
        let events = platform.drain_events();
        for ev in events {
            app.handle_event(&mut platform, ev);
        }
        app.render_dirty(&mut platform);
        if last_flush.elapsed() > Duration::from_secs(60) {
            app.store.save_history(&app.history_path);
            last_flush = Instant::now();
        }
    }
    app.shutdown();
    if smoke.is_some() {
        let report = crate::ipc_handler::state_json(&app, &platform);
        let ok = app.frames >= 3 && app.outputs.iter().any(|o| platform.is_configured(o.canvas));
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        debug!(frames = app.frames, ok, "smoke test finished");
        return Ok(if ok { 0 } else { 1 });
    }
    Ok(0)
}

/// Tell the login screen which theme this user uses (`/var/lib/edex-greeter/themes/<user>`,
/// created by tmpfiles; missing outside eDEX-OS, which is fine).
fn publish_greeter_theme(theme: &str) {
    static LAST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_deref() == Some(theme) {
        return;
    }
    *last = Some(theme.to_string());
    let dir = std::path::Path::new("/var/lib/edex-greeter/themes");
    let Some(user) = std::env::var_os("USER").filter(|u| !u.is_empty()) else {
        return;
    };
    if !dir.is_dir() {
        return;
    }
    let path = dir.join(&user);
    let tmp = dir.join(format!(".{}.tmp", user.to_string_lossy()));
    let res = std::fs::write(&tmp, format!("{theme}\n")).and_then(|_| std::fs::rename(&tmp, &path));
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        tracing::debug!("publishing the login theme: {e}");
    }
}
