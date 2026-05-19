use anyhow::{anyhow, Context, Result};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_compositor, wl_registry, wl_surface},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

#[derive(Debug, Default)]
struct WaylandState {
    configured_size: Option<(u32, u32)>,
    closed: bool,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                proxy.ack_configure(serial);
                state.configured_size = Some((width, height));
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.closed = true;
            }
            _ => {}
        }
    }
}

delegate_noop!(WaylandState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandState: ignore wl_surface::WlSurface);
delegate_noop!(WaylandState: ignore ZwlrLayerShellV1);

#[derive(Debug)]
pub struct LayerShellClient {
    connection: Connection,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    surface: wl_surface::WlSurface,
    _compositor: wl_compositor::WlCompositor,
    _layer_shell: ZwlrLayerShellV1,
    _layer_surface: ZwlrLayerSurfaceV1,
}

impl LayerShellClient {
    pub fn connect_from_env() -> Result<Self> {
        let connection =
            Connection::connect_to_env().context("failed to connect to Wayland compositor")?;
        let (globals, event_queue) = registry_queue_init::<WaylandState>(&connection)
            .context("failed to initialize Wayland registry")?;
        let qh = event_queue.handle();

        let compositor = globals
            .bind::<wl_compositor::WlCompositor, _, _>(&qh, 4..=5, ())
            .context("wl_compositor not available")?;
        let layer_shell = globals
            .bind::<ZwlrLayerShellV1, _, _>(&qh, 1..=4, ())
            .context("zwlr_layer_shell_v1 not available")?;

        let surface = compositor.create_surface(&qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            Layer::Background,
            "eDEX-DE".to_string(),
            &qh,
            (),
        );
        layer_surface.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, 0);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        surface.commit();
        connection
            .flush()
            .context("failed to flush Wayland setup requests")?;

        let mut client = Self {
            connection,
            event_queue,
            state: WaylandState::default(),
            surface,
            _compositor: compositor,
            _layer_shell: layer_shell,
            _layer_surface: layer_surface,
        };
        client.roundtrip()?;
        Ok(client)
    }

    pub fn roundtrip(&mut self) -> Result<usize> {
        let dispatched = self
            .event_queue
            .roundtrip(&mut self.state)
            .context("Wayland roundtrip failed")?;
        self.ensure_open()?;
        Ok(dispatched)
    }

    pub fn dispatch_pending(&mut self) -> Result<usize> {
        let dispatched = self
            .event_queue
            .dispatch_pending(&mut self.state)
            .context("Wayland dispatch failed")?;
        self.ensure_open()?;
        Ok(dispatched)
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }

    pub fn configured_size(&self) -> Option<(u32, u32)> {
        self.state
            .configured_size
            .filter(|(width, height)| *width > 0 && *height > 0)
    }

    fn ensure_open(&self) -> Result<()> {
        if self.state.closed {
            Err(anyhow!("layer-shell surface was closed by compositor"))
        } else {
            Ok(())
        }
    }
}
