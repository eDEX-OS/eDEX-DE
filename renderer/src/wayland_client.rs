use anyhow::{anyhow, Context, Result};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_surface},
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};
use xkbcommon::xkb;

const KEYCODE_OFFSET: u32 = 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct KeyboardModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Clone, Debug, Default)]
pub struct KeyEvent {
    pub keysym: u32,
    pub text: String,
    pub pressed: bool,
    pub modifiers: KeyboardModifiers,
}

struct KeyboardContext {
    context: xkb::Context,
    keymap: Option<xkb::Keymap>,
    state: Option<xkb::State>,
    modifiers: KeyboardModifiers,
    keyboard_bound: bool,
}

impl Default for KeyboardContext {
    fn default() -> Self {
        Self {
            context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            keymap: None,
            state: None,
            modifiers: KeyboardModifiers::default(),
            keyboard_bound: false,
        }
    }
}

impl KeyboardContext {
    fn refresh_modifiers(&mut self) {
        if let Some(state) = self.state.as_ref() {
            self.modifiers = KeyboardModifiers {
                ctrl: state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE),
                alt: state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE),
                shift: state.mod_name_is_active(xkb::MOD_NAME_SHIFT, xkb::STATE_MODS_EFFECTIVE),
                logo: state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE),
            };
        }
    }
}

#[derive(Default)]
struct WaylandState {
    configured_size: Option<(u32, u32)>,
    closed: bool,
    keyboard: KeyboardContext,
    key_events: Vec<KeyEvent>,
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

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.contains(wl_seat::Capability::Keyboard)
                && !state.keyboard.keyboard_bound
            {
                seat.get_keyboard(qh, ());
                state.keyboard.keyboard_bound = true;
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap {
                format: WEnum::Value(wl_keyboard::KeymapFormat::XkbV1),
                fd,
                size,
            } => {
                if let Ok(Some(keymap)) = unsafe {
                    xkb::Keymap::new_from_fd(
                        &state.keyboard.context,
                        fd,
                        size as usize,
                        xkb::KEYMAP_FORMAT_TEXT_V1,
                        xkb::COMPILE_NO_FLAGS,
                    )
                } {
                    let xkb_state = xkb::State::new(&keymap);
                    state.keyboard.keymap = Some(keymap);
                    state.keyboard.state = Some(xkb_state);
                    state.keyboard.refresh_modifiers();
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                if let Some(xkb_state) = state.keyboard.state.as_mut() {
                    xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                    state.keyboard.refresh_modifiers();
                }
            }
            wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                if let Some(xkb_state) = state.keyboard.state.as_ref() {
                    let keycode = key + KEYCODE_OFFSET;
                    let pressed = matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed));
                    let text = if pressed {
                        xkb_state.key_get_utf8(keycode.into())
                    } else {
                        String::new()
                    };
                    let keysym = xkb_state.key_get_one_sym(keycode.into()).raw();
                    state.key_events.push(KeyEvent {
                        keysym,
                        text,
                        pressed,
                        modifiers: state.keyboard.modifiers,
                    });
                }
            }
            _ => {}
        }
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

pub struct LayerShellClient {
    connection: Connection,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    surface: wl_surface::WlSurface,
    _compositor: wl_compositor::WlCompositor,
    _seat: wl_seat::WlSeat,
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
        let seat = globals
            .bind::<wl_seat::WlSeat, _, _>(&qh, 1..=9, ())
            .context("wl_seat not available")?;
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
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
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
            _seat: seat,
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

    pub fn drain_key_events(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.state.key_events)
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
