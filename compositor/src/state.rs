use std::{
    ffi::OsString,
    os::unix::io::OwnedFd,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output,
    delegate_seat, delegate_shm, delegate_xdg_shell,
    desktop::{Space, Window},
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::{
        calloop::{EventLoop, LoopHandle, LoopSignal},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer, wl_surface::WlSurface},
            Client, Display, DisplayHandle,
        },
    },
    utils::{Clock, Logical, Monotonic, Point, Rectangle, Size},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState as SmithayCompositorState,
        },
        output::{OutputHandler, OutputManagerState},
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};
use tracing::info;

use crate::{
    backend::udev::{init_udev_backend, BackendState},
    shell,
    shell::tiling::TilingLayout,
};

pub type CompositorState = EdexState;

#[derive(Debug)]
pub struct EdexState {
    pub display_handle: DisplayHandle,
    pub clock: Clock<Monotonic>,
    pub space: Space<Window>,
    pub compositor_state: SmithayCompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub layer_shell_state: WlrLayerShellState,
    pub seat: Seat<Self>,
    pub seat_name: String,
    pub socket_name: Option<OsString>,
    pub tiling: TilingLayout,
    pub focused_window: Option<Window>,
    pub launcher_open: Arc<Mutex<bool>>,
    pub settings_open: Arc<Mutex<bool>>,
    pub lock_screen: bool,
    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub pointer_location: Point<f64, Logical>,
}

#[derive(Debug)]
pub struct CalloopData {
    pub display: Display<EdexState>,
    pub state: EdexState,
    pub backend: BackendState,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, client_id: ClientId) {
        info!(?client_id, "wayland client initialized");
    }

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        info!(?client_id, ?reason, "wayland client disconnected");
    }
}

impl BufferHandler for EdexState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for EdexState {
    fn compositor_state(&mut self) -> &mut SmithayCompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        shell::xdg::client_compositor_state(client)
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
    }
}

impl ShmHandler for EdexState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SelectionHandler for EdexState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for EdexState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for EdexState {}
impl ServerDndGrabHandler for EdexState {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

impl SeatHandler for EdexState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}

impl OutputHandler for EdexState {}

impl EdexState {
    pub fn refresh_layout(&mut self) {
        if let Some(size) = self.primary_output_size() {
            self.tiling.screen_size = size;
        }
        self.tiling.retile(&mut self.space);
        self.space.refresh();
    }

    pub fn focus_window(&mut self, window: Window) {
        self.space.raise_element(&window, true);
        if let Some(toplevel) = window.toplevel() {
            if let Some(keyboard) = self.seat.get_keyboard() {
                keyboard.set_focus(
                    self,
                    Some(toplevel.wl_surface().clone()),
                    smithay::utils::SERIAL_COUNTER.next_serial(),
                );
            }
        }
        self.focused_window = Some(window);
    }

    pub fn cycle_focus(&mut self) {
        if self.tiling.windows.is_empty() {
            return;
        }

        let next_index = self
            .focused_window
            .as_ref()
            .and_then(|focused| {
                self.tiling
                    .windows
                    .iter()
                    .position(|window| window == focused)
            })
            .map(|index| (index + 1) % self.tiling.windows.len())
            .unwrap_or(0);

        if let Some(window) = self.tiling.windows.get(next_index).cloned() {
            self.focus_window(window);
            self.refresh_layout();
        }
    }

    pub fn close_focused_window(&mut self) {
        if let Some(window) = self.focused_window.clone() {
            if let Some(toplevel) = window.toplevel() {
                toplevel.send_close();
            }
        }
    }

    pub fn primary_output_geometry(&self) -> Option<Rectangle<i32, Logical>> {
        self.space
            .outputs()
            .next()
            .and_then(|output| self.space.output_geometry(output))
    }

    pub fn primary_output_size(&self) -> Option<Size<i32, Logical>> {
        self.primary_output_geometry().map(|geometry| geometry.size)
    }
}

pub fn run_compositor() -> Result<()> {
    run_compositor_with_socket_notifier(|_| {})
}

pub fn run_compositor_with_socket_notifier<F>(on_socket_ready: F) -> Result<()>
where
    F: FnOnce(OsString) + Send + 'static,
{
    run_compositor_with_socket_notifier_and_launcher_flag(
        on_socket_ready,
        Arc::new(Mutex::new(false)),
    )
}

pub fn run_compositor_with_socket_notifier_and_launcher_flag<F>(
    on_socket_ready: F,
    launcher_open: Arc<Mutex<bool>>,
) -> Result<()>
where
    F: FnOnce(OsString) + Send + 'static,
{
    run_compositor_with_socket_notifier_and_shared_flags(
        on_socket_ready,
        launcher_open,
        Arc::new(Mutex::new(false)),
    )
}

pub fn run_compositor_with_socket_notifier_and_shared_flags<F>(
    on_socket_ready: F,
    launcher_open: Arc<Mutex<bool>>,
    settings_open: Arc<Mutex<bool>>,
) -> Result<()>
where
    F: FnOnce(OsString) + Send + 'static,
{
    let event_loop = Box::new(
        EventLoop::<CalloopData>::try_new().context("failed to create calloop event loop")?,
    );
    let event_loop = Box::leak(event_loop);
    let loop_handle = event_loop.handle();
    let loop_signal = event_loop.get_signal();

    let display = Display::<EdexState>::new().context("failed to create wayland display")?;
    let display_handle = display.handle();

    let compositor_state = SmithayCompositorState::new::<EdexState>(&display_handle);
    let xdg_shell_state = XdgShellState::new::<EdexState>(&display_handle);
    let shm_state = ShmState::new::<EdexState>(&display_handle, vec![]);
    let output_manager_state =
        OutputManagerState::new_with_xdg_output::<EdexState>(&display_handle);
    let data_device_state = DataDeviceState::new::<EdexState>(&display_handle);
    let layer_shell_state = WlrLayerShellState::new::<EdexState>(&display_handle);
    let mut seat_state = SeatState::new();
    let mut seat = seat_state.new_wl_seat(&display_handle, "seat0");
    seat.add_keyboard(Default::default(), 200, 25)
        .context("failed to create wl_keyboard")?;
    seat.add_pointer();

    let mut state = EdexState {
        display_handle: display_handle.clone(),
        clock: Clock::new(),
        space: Space::default(),
        compositor_state,
        xdg_shell_state,
        shm_state,
        output_manager_state,
        seat_state,
        data_device_state,
        layer_shell_state,
        seat,
        seat_name: "seat0".to_string(),
        socket_name: None,
        tiling: TilingLayout::new((1920, 1080).into()),
        focused_window: None,
        launcher_open,
        settings_open,
        lock_screen: false,
        loop_signal,
        loop_handle: loop_handle.clone(),
        pointer_location: (0.0, 0.0).into(),
    };

    let backend = init_udev_backend(&mut state, &loop_handle)?;
    for (index, output) in backend.outputs.iter().enumerate() {
        state.space.map_output(output, ((index as i32) * 1920, 0));
    }
    if let Some(size) = state.primary_output_size() {
        state.tiling.screen_size = size;
    }

    let socket = ListeningSocketSource::new_auto().context("failed to open wayland socket")?;
    let socket_name = socket.socket_name().to_os_string();
    state.socket_name = Some(socket_name.clone());
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    info!(socket = ?socket_name, "wayland socket ready");
    on_socket_ready(socket_name.clone());

    loop_handle
        .insert_source(socket, |client_stream, _, data| {
            if let Err(error) = data
                .display
                .handle()
                .insert_client(client_stream, Arc::new(ClientState::default()))
            {
                tracing::warn!(?error, "failed to insert wayland client");
            }
        })
        .context("failed to insert wayland socket source")?;

    let mut data = CalloopData {
        display,
        state,
        backend,
    };

    loop {
        event_loop
            .dispatch(None, &mut data)
            .context("calloop dispatch failed")?;
        data.display
            .dispatch_clients(&mut data.state)
            .context("wayland dispatch failed")?;
        data.display
            .flush_clients()
            .context("wayland flush failed")?;
    }
}

delegate_compositor!(EdexState);
delegate_xdg_shell!(EdexState);
delegate_layer_shell!(EdexState);
delegate_shm!(EdexState);
delegate_seat!(EdexState);
delegate_data_device!(EdexState);
delegate_output!(EdexState);
