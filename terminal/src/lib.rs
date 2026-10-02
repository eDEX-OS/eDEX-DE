//! Multi-tab terminal emulator built on `alacritty_terminal`.

pub mod colors;
pub mod frame;
pub mod input;

use std::{collections::HashMap, path::PathBuf, sync::Arc, thread::JoinHandle, time::Instant};

use alacritty_terminal::{
    event::{Event as AlacEvent, EventListener, Notify, WindowSize},
    event_loop::{EventLoop, EventLoopSender, Msg, Notifier},
    grid::{Dimensions, Scroll},
    index::{Column, Point, Side},
    selection::{Selection, SelectionType},
    sync::FairMutex,
    term::{self, Config as TermConfig, Term, TermMode},
    tty,
    vte::ansi::{CursorShape as AlacCursorShape, CursorStyle, Rgb},
};
use anyhow::{Context, Result};
use tracing::{info, warn};
use ui::terminal_model::{CursorShape, TabInfo, TerminalFrame};

pub use colors::Palette;
pub use input::{KeyModes, Modifiers};

/// Grid dimensions passed to `Term::new` / `Term::resize`.
#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// Terminal configuration (from the settings crate).
#[derive(Clone, Debug)]
pub struct TerminalConfig {
    /// Shell program; `None` uses the user's login shell.
    pub shell: Option<String>,
    pub shell_args: Vec<String>,
    pub scrollback: usize,
    pub cursor: CursorShape,
    pub cursor_blink: bool,
    pub osc52_read: bool,
    pub working_directory: Option<PathBuf>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: None,
            shell_args: Vec::new(),
            scrollback: 10_000,
            cursor: CursorShape::Block,
            cursor_blink: true,
            osc52_read: false,
            working_directory: None,
        }
    }
}

/// Formatter turning clipboard text into the escape sequence an application requested.
pub type ClipboardFormatter = Arc<dyn Fn(&str) -> String + Sync + Send + 'static>;
/// Formatter for an OSC 4/10/11/12 colour query reply.
pub type ColorFormatter = Arc<dyn Fn(Rgb) -> String + Sync + Send + 'static>;
/// Formatter for a text-area size (CSI 14/18 t) query reply.
pub type SizeFormatter = Arc<dyn Fn(WindowSize) -> String + Sync + Send + 'static>;

/// Events delivered from PTY threads to the main loop.
#[derive(Clone)]
pub enum TermEvent {
    Wakeup(u64),
    Title(u64, Option<String>),
    Bell(u64),
    Exit(u64, i32),
    ClipboardStore(String),
    ClipboardLoad(u64, ClipboardFormatter),
    CursorBlinkingChange(u64),
    /// Bytes the terminal must send back to the application (device attributes, cursor
    /// position reports, mode reports...).
    PtyWrite(u64, String),
    ColorRequest(u64, usize, ColorFormatter),
    TextAreaSize(u64, SizeFormatter),
}

impl std::fmt::Debug for TermEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermEvent::Wakeup(id) => write!(f, "Wakeup({id})"),
            TermEvent::Title(id, t) => write!(f, "Title({id}, {t:?})"),
            TermEvent::Bell(id) => write!(f, "Bell({id})"),
            TermEvent::Exit(id, c) => write!(f, "Exit({id}, {c})"),
            TermEvent::ClipboardStore(_) => write!(f, "ClipboardStore"),
            TermEvent::ClipboardLoad(id, _) => write!(f, "ClipboardLoad({id})"),
            TermEvent::CursorBlinkingChange(id) => write!(f, "CursorBlinkingChange({id})"),
            TermEvent::PtyWrite(id, s) => write!(f, "PtyWrite({id}, {s:?})"),
            TermEvent::ColorRequest(id, i, _) => write!(f, "ColorRequest({id}, {i})"),
            TermEvent::TextAreaSize(id, _) => write!(f, "TextAreaSize({id})"),
        }
    }
}

/// Sink for terminal events (a calloop channel sender in the shell, a Vec in tests).
pub trait EventSink: Send + Sync + 'static {
    fn send(&self, event: TermEvent);
}

impl<F: Fn(TermEvent) + Send + Sync + 'static> EventSink for F {
    fn send(&self, event: TermEvent) {
        self(event)
    }
}

#[derive(Clone)]
struct Listener {
    id: u64,
    sink: Arc<dyn EventSink>,
}

impl EventListener for Listener {
    fn send_event(&self, event: AlacEvent) {
        let out = match event {
            AlacEvent::Wakeup => TermEvent::Wakeup(self.id),
            AlacEvent::Title(t) => TermEvent::Title(self.id, Some(t)),
            AlacEvent::ResetTitle => TermEvent::Title(self.id, None),
            AlacEvent::Bell => TermEvent::Bell(self.id),
            AlacEvent::ChildExit(status) => TermEvent::Exit(self.id, status.code().unwrap_or(-1)),
            AlacEvent::ClipboardStore(_, text) => TermEvent::ClipboardStore(text),
            AlacEvent::ClipboardLoad(_, formatter) => TermEvent::ClipboardLoad(self.id, formatter),
            AlacEvent::CursorBlinkingChange => TermEvent::CursorBlinkingChange(self.id),
            AlacEvent::PtyWrite(text) => TermEvent::PtyWrite(self.id, text),
            AlacEvent::ColorRequest(index, formatter) => {
                TermEvent::ColorRequest(self.id, index, formatter)
            }
            AlacEvent::TextAreaSizeRequest(formatter) => {
                TermEvent::TextAreaSize(self.id, formatter)
            }
            AlacEvent::MouseCursorDirty | AlacEvent::Exit => return,
        };
        self.sink.send(out);
    }
}

struct Tab {
    id: u64,
    term: Arc<FairMutex<Term<Listener>>>,
    notifier: Notifier,
    sender: EventLoopSender,
    join: Option<
        JoinHandle<(
            EventLoop<tty::Pty, Listener>,
            alacritty_terminal::event_loop::State,
        )>,
    >,
    title: Option<String>,
    exited: Option<i32>,
    /// Program tab (e.g. ranger): removed as soon as the program exits.
    close_on_exit: bool,
    bell_at: Option<Instant>,
    /// Pending OSC 52 / bracketed clipboard read formatter.
    pending_clipboard: Option<ClipboardFormatter>,
    dragging: bool,
    child_pid: u32,
}

impl Tab {
    fn write(&self, bytes: Vec<u8>) {
        self.notifier.notify(bytes);
    }

    fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| "shell".to_string())
    }
}

impl Drop for Tab {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Multi-tab terminal.
pub struct TerminalTabs {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    cols: usize,
    rows: usize,
    cell_size: (u16, u16),
    config: TerminalConfig,
    sink: Arc<dyn EventSink>,
    ids: HashMap<u64, usize>,
    palette: Option<Palette>,
}

impl TerminalTabs {
    pub fn new(
        config: TerminalConfig,
        sink: Arc<dyn EventSink>,
        cols: usize,
        rows: usize,
        cell_w: f32,
        cell_h: f32,
    ) -> Result<Self> {
        tty::setup_env();
        let mut tabs = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            cols: cols.max(2),
            rows: rows.max(1),
            cell_size: (
                cell_w.round().max(1.0) as u16,
                cell_h.round().max(1.0) as u16,
            ),
            config,
            sink,
            ids: HashMap::new(),
            palette: None,
        };
        tabs.new_tab()?;
        Ok(tabs)
    }

    pub fn set_config(&mut self, config: TerminalConfig) {
        self.config = config;
    }

    fn window_size(&self) -> WindowSize {
        WindowSize {
            num_lines: self.rows as u16,
            num_cols: self.cols as u16,
            cell_width: self.cell_size.0,
            cell_height: self.cell_size.1,
        }
    }

    fn term_config(&self) -> TermConfig {
        TermConfig {
            scrolling_history: self.config.scrollback,
            default_cursor_style: CursorStyle {
                shape: match self.config.cursor {
                    CursorShape::Block => AlacCursorShape::Block,
                    CursorShape::Underline => AlacCursorShape::Underline,
                    CursorShape::Beam => AlacCursorShape::Beam,
                    CursorShape::Hidden => AlacCursorShape::Hidden,
                },
                blinking: self.config.cursor_blink,
            },
            osc52: if self.config.osc52_read {
                term::Osc52::CopyPaste
            } else {
                term::Osc52::OnlyCopy
            },
            ..TermConfig::default()
        }
    }

    /// Spawn a new shell tab and make it active.
    pub fn new_tab(&mut self) -> Result<usize> {
        self.spawn_tab(None, None)
    }

    /// Run `program args…` in a new tab in `cwd`; the tab closes when the program exits.
    pub fn new_command_tab(
        &mut self,
        program: &str,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    ) -> Result<usize> {
        self.spawn_tab(Some((program.to_string(), args)), cwd)
    }

    fn spawn_tab(
        &mut self,
        command: Option<(String, Vec<String>)>,
        cwd: Option<PathBuf>,
    ) -> Result<usize> {
        let close_on_exit = command.is_some();
        let initial_title = command.as_ref().map(|(program, _)| program.clone());
        let id = self.next_id;
        self.next_id += 1;
        let listener = Listener {
            id,
            sink: self.sink.clone(),
        };
        let size = self.window_size();
        let term = Term::new(
            self.term_config(),
            &GridSize {
                cols: self.cols,
                rows: self.rows,
            },
            listener.clone(),
        );
        let term = Arc::new(FairMutex::new(term));
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        env.insert("TERM_PROGRAM".to_string(), "edex-de".to_string());
        env.insert(
            "TERM_PROGRAM_VERSION".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        let shell = match command {
            Some((program, args)) => Some(tty::Shell::new(program, args)),
            None => self
                .config
                .shell
                .clone()
                .map(|program| tty::Shell::new(program, self.config.shell_args.clone())),
        };
        let working_directory = cwd
            .or_else(|| self.config.working_directory.clone())
            .or_else(|| self.tabs.get(self.active).and_then(child_cwd))
            .or_else(|| std::env::var("HOME").ok().map(PathBuf::from));
        let options = tty::Options {
            shell,
            working_directory,
            drain_on_exit: false,
            env,
        };
        let pty = tty::new(&options, size, id).context("failed to spawn PTY")?;
        let child_pid = pty.child().id();
        let event_loop = EventLoop::new(term.clone(), listener, pty, false, false)
            .context("failed to create PTY event loop")?;
        let sender = event_loop.channel();
        let notifier = Notifier(sender.clone());
        let join = event_loop.spawn();
        self.tabs.push(Tab {
            id,
            term,
            notifier,
            sender,
            join: Some(join),
            title: initial_title,
            exited: None,
            bell_at: None,
            pending_clipboard: None,
            dragging: false,
            child_pid,
            close_on_exit,
        });
        self.active = self.tabs.len() - 1;
        self.reindex();
        info!(tab = id, "terminal tab spawned");
        Ok(self.active)
    }

    fn reindex(&mut self) {
        self.ids = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id, i))
            .collect();
    }

    /// Close a tab; the last tab is replaced with a fresh shell.
    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        if index >= self.tabs.len() {
            return Ok(());
        }
        let tab = self.tabs.remove(index);
        drop(tab);
        if self.tabs.is_empty() {
            self.active = 0;
            self.reindex();
            self.new_tab()?;
            return Ok(());
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        self.reindex();
        Ok(())
    }

    pub fn switch(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn tab_infos(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| TabInfo {
                title: t.display_title(),
                active: i == self.active,
                exited: t.exited.is_some(),
            })
            .collect()
    }

    pub fn active_exited(&self) -> bool {
        self.tabs
            .get(self.active)
            .is_some_and(|t| t.exited.is_some())
    }

    /// Resize every tab's grid and PTY.
    pub fn resize(&mut self, cols: usize, rows: usize, cell_w: f32, cell_h: f32) {
        let cols = cols.max(2);
        let rows = rows.max(1);
        let cell = (
            cell_w.round().max(1.0) as u16,
            cell_h.round().max(1.0) as u16,
        );
        if cols == self.cols && rows == self.rows && cell == self.cell_size {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.cell_size = cell;
        let size = self.window_size();
        for tab in &self.tabs {
            tab.term.lock().resize(GridSize { cols, rows });
            let _ = tab.sender.send(Msg::Resize(size));
        }
    }

    pub fn grid_size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    /// Write raw bytes to the active tab.
    pub fn write_input(&mut self, bytes: &[u8]) {
        if let Some(tab) = self.tabs.get(self.active) {
            if tab.exited.is_none() {
                self.scroll_to_bottom();
                tab.write(bytes.to_vec());
            }
        }
    }

    fn scroll_to_bottom(&self) {
        if let Some(tab) = self.tabs.get(self.active) {
            let mut term = tab.term.lock();
            if term.grid().display_offset() != 0 {
                term.scroll_display(Scroll::Bottom);
            }
        }
    }

    /// Current key encoding modes of the active tab.
    pub fn key_modes(&self) -> KeyModes {
        self.tabs
            .get(self.active)
            .map(|t| {
                let mode = *t.term.lock().mode();
                KeyModes {
                    app_cursor: mode.contains(TermMode::APP_CURSOR),
                    app_keypad: mode.contains(TermMode::APP_KEYPAD),
                }
            })
            .unwrap_or_default()
    }

    /// Handle a key press for the active tab. Returns true if the key was consumed.
    pub fn key_press(&mut self, keysym: u32, text: Option<&str>, mods: Modifiers) -> bool {
        let Some(tab) = self.tabs.get(self.active) else {
            return false;
        };
        if tab.exited.is_some() {
            if keysym == xkbcommon::xkb::keysyms::KEY_Return {
                let idx = self.active;
                let _ = self.close_tab(idx);
            }
            return true;
        }
        // Scrollback navigation.
        if mods.shift && !mods.ctrl && !mods.alt {
            match keysym {
                xkbcommon::xkb::keysyms::KEY_Page_Up => {
                    tab.term.lock().scroll_display(Scroll::PageUp);
                    return true;
                }
                xkbcommon::xkb::keysyms::KEY_Page_Down => {
                    tab.term.lock().scroll_display(Scroll::PageDown);
                    return true;
                }
                xkbcommon::xkb::keysyms::KEY_Home => {
                    tab.term.lock().scroll_display(Scroll::Top);
                    return true;
                }
                xkbcommon::xkb::keysyms::KEY_End => {
                    tab.term.lock().scroll_display(Scroll::Bottom);
                    return true;
                }
                _ => {}
            }
        }
        let modes = self.key_modes();
        if let Some(bytes) = input::key_to_bytes(keysym, text, mods, modes) {
            {
                let mut term = tab.term.lock();
                if term.selection.is_some() {
                    term.selection = None;
                }
            }
            self.write_input(&bytes);
            return true;
        }
        false
    }

    /// Paste text into the active tab (bracketed when the app requested it).
    pub fn paste(&mut self, text: &str) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        if let Some(formatter) = tab.pending_clipboard.clone() {
            let idx = self.active;
            self.tabs[idx].pending_clipboard = None;
            self.tabs[idx].write(formatter(text).into_bytes());
            return;
        }
        let bracketed = tab.term.lock().mode().contains(TermMode::BRACKETED_PASTE);
        let bytes = input::encode_paste(text, bracketed);
        self.write_input(&bytes);
    }

    /// Whether a clipboard read is pending for OSC 52.
    pub fn wants_clipboard(&self) -> bool {
        self.tabs.iter().any(|t| t.pending_clipboard.is_some())
    }

    /// Scroll the active tab by `lines` (negative = towards history).
    pub fn scroll(&mut self, lines: i32) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        let mut term = tab.term.lock();
        let mode = *term.mode();
        if mode.contains(TermMode::ALT_SCREEN)
            && mode.contains(TermMode::ALTERNATE_SCROLL)
            && !mode.intersects(TermMode::MOUSE_MODE)
        {
            drop(term);
            let modes = self.key_modes();
            let key = if lines < 0 {
                xkbcommon::xkb::keysyms::KEY_Up
            } else {
                xkbcommon::xkb::keysyms::KEY_Down
            };
            for _ in 0..lines.unsigned_abs() {
                if let Some(b) = input::key_to_bytes(key, None, Modifiers::default(), modes) {
                    tab.write(b);
                }
            }
            return;
        }
        term.scroll_display(Scroll::Delta(-lines));
    }

    fn active_mouse_mode(&self) -> Option<TermMode> {
        let tab = self.tabs.get(self.active)?;
        let mode = *tab.term.lock().mode();
        mode.intersects(TermMode::MOUSE_MODE).then_some(mode)
    }

    fn mouse_report(
        &self,
        button: u32,
        col: usize,
        row: usize,
        mods: Modifiers,
        pressed: bool,
        motion: bool,
    ) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        let mut b = match button {
            0x110 => 0,
            0x112 => 1,
            0x111 => 2,
            64 => 64,
            65 => 65,
            _ => 3,
        };
        if mods.shift {
            b += 4;
        }
        if mods.alt {
            b += 8;
        }
        if mods.ctrl {
            b += 16;
        }
        if motion {
            b += 32;
        }
        let sgr = tab.term.lock().mode().contains(TermMode::SGR_MOUSE);
        let seq = if sgr {
            format!(
                "\x1b[<{b};{};{}{}",
                col + 1,
                row + 1,
                if pressed { 'M' } else { 'm' }
            )
        } else {
            let b = if pressed { b } else { 3 };
            let mut v = vec![0x1b, b'[', b'M', (32 + b) as u8];
            v.push((32 + col.min(222) + 1) as u8);
            v.push((32 + row.min(222) + 1) as u8);
            match String::from_utf8(v) {
                Ok(s) => s,
                Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
            }
        };
        tab.write(seq.into_bytes());
    }

    /// Pointer button on the grid at (col, row). Returns true if the event was consumed
    /// by mouse reporting or selection.
    pub fn mouse_press(
        &mut self,
        col: usize,
        row: usize,
        button: u32,
        mods: Modifiers,
        click_count: u8,
    ) -> bool {
        if let Some(mode) = self.active_mouse_mode() {
            if !mods.shift {
                let _ = mode;
                self.mouse_report(button, col, row, mods, true, false);
                return true;
            }
        }
        if button != 0x110 {
            return false;
        }
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        let mut term = tab.term.lock();
        let display_offset = term.grid().display_offset();
        let point = term::viewport_to_point(
            display_offset,
            Point::new(row, Column(col.min(self.cols.saturating_sub(1)))),
        );
        let ty = match click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ if mods.ctrl => SelectionType::Block,
            _ => SelectionType::Simple,
        };
        term.selection = Some(Selection::new(ty, point, Side::Left));
        drop(term);
        tab.dragging = true;
        true
    }

    pub fn mouse_motion(&mut self, col: usize, row: usize, mods: Modifiers, buttons_held: bool) {
        if let Some(mode) = self.active_mouse_mode() {
            if mode.contains(TermMode::MOUSE_MOTION)
                || (mode.contains(TermMode::MOUSE_DRAG) && buttons_held)
            {
                self.mouse_report(
                    if buttons_held { 0x110 } else { 3 },
                    col,
                    row,
                    mods,
                    true,
                    true,
                );
            }
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        if !tab.dragging {
            return;
        }
        let mut term = tab.term.lock();
        let display_offset = term.grid().display_offset();
        let point = term::viewport_to_point(
            display_offset,
            Point::new(
                row.min(self.rows.saturating_sub(1)),
                Column(col.min(self.cols.saturating_sub(1))),
            ),
        );
        if let Some(sel) = term.selection.as_mut() {
            sel.update(point, Side::Right);
        }
    }

    pub fn mouse_release(&mut self, col: usize, row: usize, button: u32, mods: Modifiers) {
        if self.active_mouse_mode().is_some() && !mods.shift {
            self.mouse_report(button, col, row, mods, false, false);
            return;
        }
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.dragging = false;
            let mut term = tab.term.lock();
            if term.selection.as_ref().is_some_and(|s| s.is_empty()) {
                term.selection = None;
            }
        }
    }

    /// Mouse wheel over the grid.
    pub fn mouse_scroll(&mut self, col: usize, row: usize, delta_lines: i32, mods: Modifiers) {
        if self.active_mouse_mode().is_some() && !mods.shift {
            let button = if delta_lines < 0 { 64 } else { 65 };
            for _ in 0..delta_lines.unsigned_abs().max(1) {
                self.mouse_report(button, col, row, mods, true, false);
            }
            return;
        }
        self.scroll(delta_lines);
    }

    /// Text of the current selection in the active tab.
    pub fn selection_text(&self) -> Option<String> {
        self.tabs
            .get(self.active)
            .and_then(|t| t.term.lock().selection_to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn clear_selection(&mut self) {
        if let Some(tab) = self.tabs.get(self.active) {
            tab.term.lock().selection = None;
        }
    }

    /// Apply a terminal event. Returns true when the active view needs a redraw.
    pub fn handle_event(&mut self, event: TermEvent) -> bool {
        match event {
            TermEvent::Wakeup(id) => self.ids.get(&id) == Some(&self.active),
            TermEvent::Title(id, title) => {
                if let Some(&i) = self.ids.get(&id) {
                    self.tabs[i].title = title;
                }
                true
            }
            TermEvent::Bell(id) => {
                if let Some(&i) = self.ids.get(&id) {
                    self.tabs[i].bell_at = Some(Instant::now());
                }
                true
            }
            TermEvent::Exit(id, code) => {
                if let Some(&i) = self.ids.get(&id) {
                    info!(tab = id, code, "shell exited");
                    if self.tabs[i].close_on_exit {
                        let _ = self.close_tab(i);
                    } else {
                        self.tabs[i].exited = Some(code);
                    }
                }
                true
            }
            TermEvent::ClipboardLoad(id, formatter) => {
                if let Some(&i) = self.ids.get(&id) {
                    if self.config.osc52_read {
                        self.tabs[i].pending_clipboard = Some(formatter);
                    } else {
                        warn!("ignoring OSC 52 clipboard read (terminal.osc52_read = false)");
                    }
                }
                false
            }
            TermEvent::PtyWrite(id, text) => {
                if let Some(&i) = self.ids.get(&id) {
                    self.tabs[i].write(text.into_bytes());
                }
                false
            }
            TermEvent::ColorRequest(id, index, formatter) => {
                if let Some(&i) = self.ids.get(&id) {
                    let set = self.tabs[i].term.lock().colors()[index];
                    let rgb = set.or_else(|| self.palette.as_ref().map(|p| p.rgb_for_index(index)));
                    if let Some(rgb) = rgb {
                        self.tabs[i].write(formatter(rgb).into_bytes());
                    }
                }
                false
            }
            TermEvent::TextAreaSize(id, formatter) => {
                if let Some(&i) = self.ids.get(&id) {
                    let reply = formatter(self.window_size());
                    self.tabs[i].write(reply.into_bytes());
                }
                false
            }
            TermEvent::ClipboardStore(_) | TermEvent::CursorBlinkingChange(_) => true,
        }
    }

    /// Theme palette used to answer colour queries (OSC 4/10/11/12).
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = Some(palette);
    }

    /// Build the frame for the active tab.
    pub fn frame(
        &self,
        palette: &Palette,
        cursor_visible: bool,
        focused: bool,
        now: Instant,
    ) -> TerminalFrame {
        let Some(tab) = self.tabs.get(self.active) else {
            return TerminalFrame::default();
        };
        let term = tab.term.lock();
        let bell = tab
            .bell_at
            .is_some_and(|t| now.duration_since(t).as_millis() < 150);
        let opts = frame::FrameOptions {
            cursor_visible,
            focused,
            bell,
            exited: tab.exited,
            title: tab.display_title(),
        };
        frame::build_frame(&term, palette, &opts)
    }

    /// Whether the active tab's cursor should blink.
    pub fn cursor_blinks(&self) -> bool {
        self.config.cursor_blink
    }

    /// Write the same bytes to the active tab (used by the on-screen keyboard).
    pub fn inject_text(&mut self, text: &str) {
        self.write_input(text.as_bytes());
    }
}

/// Current working directory of the shell running in a tab (from procfs).
fn child_cwd(tab: &Tab) -> Option<PathBuf> {
    if tab.exited.is_some() {
        return None;
    }
    std::fs::read_link(format!("/proc/{}/cwd", tab.child_pid))
        .ok()
        .filter(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::vte::ansi::NamedColor;

    fn palette() -> Palette {
        Palette {
            fg: [1.0, 1.0, 1.0, 1.0],
            bg: [0.0, 0.0, 0.0, 1.0],
            cursor: [0.0, 1.0, 1.0, 1.0],
            ansi: [[0.5, 0.5, 0.5, 1.0]; 16],
        }
    }

    /// Device-attribute queries must produce a reply for the application (fish, vim and
    /// others wait for it).
    #[test]
    fn device_attribute_query_is_answered() {
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = {
            let events = events.clone();
            move |e: TermEvent| events.lock().unwrap().push(e)
        };
        let listener = Listener {
            id: 7,
            sink: Arc::new(sink),
        };
        let mut term = Term::new(
            TermConfig::default(),
            &GridSize { cols: 20, rows: 5 },
            listener,
        );
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, b"\x1b[c\x1b[6n");
        let got: Vec<String> = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                TermEvent::PtyWrite(7, s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].starts_with("\x1b[?"), "{got:?}");
        assert_eq!(got[1], "\x1b[1;1R");
    }

    #[test]
    fn colour_queries_resolve_from_the_palette() {
        let p = palette();
        assert_eq!(
            p.rgb_for_index(NamedColor::Background as usize),
            Rgb { r: 0, g: 0, b: 0 }
        );
        assert_eq!(
            p.rgb_for_index(NamedColor::Foreground as usize),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(
            p.rgb_for_index(3),
            Rgb {
                r: 128,
                g: 128,
                b: 128
            }
        );
    }

    fn term_with(bytes: &[u8], cols: usize, rows: usize) -> Term<VoidListener> {
        let mut term = Term::new(
            TermConfig::default(),
            &GridSize { cols, rows },
            VoidListener,
        );
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, bytes);
        term
    }

    fn text_of(frame: &TerminalFrame, row: usize) -> String {
        frame.lines[row]
            .cells
            .iter()
            .map(|c| {
                if c.text.is_empty() {
                    " ".to_string()
                } else {
                    c.text.clone()
                }
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn frame_reflects_grid_and_attributes() {
        let term = term_with(b"hello \x1b[1;31mworld\x1b[0m\r\nline2", 20, 4);
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: "t".into(),
            },
        );
        assert_eq!(text_of(&frame, 0), "hello world");
        assert_eq!(text_of(&frame, 1), "line2");
        assert!(frame.lines[0].cells[6].style.bold);
        assert_eq!(frame.cursor.map(|c| (c.col, c.row)), Some((5, 1)));
    }

    #[test]
    fn alt_screen_and_scroll_region() {
        let term = term_with(b"\x1b[?1049hALT", 10, 3);
        assert!(term.mode().contains(TermMode::ALT_SCREEN));
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: String::new(),
            },
        );
        assert_eq!(text_of(&frame, 0), "ALT");
        let term = term_with(b"1\r\n2\r\n3\x1b[1;2r\x1b[H\x1b[Ma", 10, 3);
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: String::new(),
            },
        );
        assert_eq!(text_of(&frame, 0), "a");
        assert_eq!(text_of(&frame, 2), "3");
    }

    #[test]
    fn wide_characters_take_two_cells() {
        let term = term_with("日本x".as_bytes(), 10, 2);
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: String::new(),
            },
        );
        assert!(frame.lines[0].cells[0].wide);
        assert_eq!(frame.lines[0].cells[1].text, "");
        assert_eq!(frame.lines[0].cells[4].text, "x");
    }

    #[test]
    fn insert_delete_and_erase() {
        let term = term_with(b"abcdef\x1b[3G\x1b[2P\x1b[2@X\x1b[1X", 20, 2);
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: String::new(),
            },
        );
        assert_eq!(text_of(&frame, 0), "abX ef");
    }

    #[test]
    fn save_restore_cursor_and_sgr_colors() {
        let term = term_with(b"\x1b7\x1b[5;5H\x1b8Q\x1b[38;2;10;20;30mZ", 10, 6);
        let frame = frame::build_frame(
            &term,
            &palette(),
            &frame::FrameOptions {
                cursor_visible: true,
                focused: true,
                bell: false,
                exited: None,
                title: String::new(),
            },
        );
        assert_eq!(text_of(&frame, 0), "QZ");
        let fg = frame.lines[0].cells[1].style.fg;
        assert!((fg[0] - 10.0 / 255.0).abs() < 0.01 && (fg[2] - 30.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn spawns_a_real_shell_and_reads_output() {
        let (tx, rx) = std::sync::mpsc::channel::<TermEvent>();
        let sink: Arc<dyn EventSink> = Arc::new(move |e| {
            let _ = tx.send(e);
        });
        let config = TerminalConfig {
            shell: Some("/bin/sh".into()),
            shell_args: vec!["-c".into(), "printf 'PING\\n'; sleep 0.2".into()],
            ..Default::default()
        };
        let mut tabs = TerminalTabs::new(config, sink, 40, 5, 8.0, 16.0).unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        let mut saw_exit = false;
        while Instant::now() < deadline {
            if let Ok(ev) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
                if matches!(ev, TermEvent::Exit(..)) {
                    saw_exit = true;
                }
                tabs.handle_event(ev);
                if saw_exit {
                    break;
                }
            }
        }
        let frame = tabs.frame(&palette(), true, true, Instant::now());
        assert_eq!(text_of(&frame, 0), "PING");
        assert!(saw_exit, "shell exit was not reported");
        assert!(tabs.active_exited());
        tabs.key_press(
            xkbcommon::xkb::keysyms::KEY_Return,
            None,
            Modifiers::default(),
        );
        assert_eq!(tabs.len(), 1);
        assert!(!tabs.active_exited());
    }
}
