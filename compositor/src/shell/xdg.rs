use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat},
            Client,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, Size},
    wayland::{
        compositor::CompositorClientState,
        shell::xdg::{Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler},
    },
};

use crate::state::EdexState;

impl XdgShellHandler for EdexState {
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface.clone());
        self.space.map_element(window, (0, 0), true);
        self.refresh_layout();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        _positioner: PositionerState,
        token: u32,
    ) {
        surface.send_repositioned(token);
    }

    fn ack_configure(
        &mut self,
        _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        _configure: Configure,
    ) {
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _output: Option<WlOutput>) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Fullscreen);
            state.states.set(xdg_toplevel::State::Activated);
            if let Some(size) = self.primary_output_size() {
                state.size = Some(size);
            }
        });
        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
        });
        surface.send_configure();
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Maximized);
            if let Some(size) = self.primary_output_size() {
                state.size = Some(size);
            }
        });
        surface.send_configure();
    }

    fn move_request(&mut self, _surface: ToplevelSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
    }

    fn resize_request(
        &mut self,
        _surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial,
        _edges: xdg_toplevel::ResizeEdge,
    ) {
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let target = self
            .space
            .elements()
            .find(|window| {
                window
                    .toplevel()
                    .map(|toplevel| toplevel.wl_surface() == surface.wl_surface())
                    .unwrap_or(false)
            })
            .cloned();
        if let Some(window) = target {
            self.space.unmap_elem(&window);
            self.refresh_layout();
        }
    }
}

pub fn apply_window_layout(state: &mut EdexState) {
    let Some(output_geometry) = state.primary_output_geometry() else {
        return;
    };

    let windows = state.space.elements().cloned().collect::<Vec<_>>();
    let window_count = windows.len();

    for (index, window) in windows.iter().enumerate() {
        let geometry = layout_geometry(output_geometry, window_count, index);
        state
            .space
            .map_element(window.clone(), geometry.loc, index + 1 == window_count);
        if let Some(toplevel) = window.toplevel() {
            toplevel.with_pending_state(|pending| {
                pending.size = Some(geometry.size);
                pending.states.set(xdg_toplevel::State::Activated);
                if window_count == 1 {
                    pending.states.set(xdg_toplevel::State::Fullscreen);
                } else {
                    pending.states.unset(xdg_toplevel::State::Fullscreen);
                }
            });
            toplevel.send_configure();
        }
    }
}

fn layout_geometry(
    output_geometry: Rectangle<i32, Logical>,
    window_count: usize,
    index: usize,
) -> Rectangle<i32, Logical> {
    if window_count <= 1 {
        return output_geometry;
    }

    let cols = (window_count as f64).sqrt().ceil() as i32;
    let rows = ((window_count as f64) / cols as f64).ceil() as i32;
    let col = index as i32 % cols;
    let row = index as i32 / cols;
    let tile_width = (output_geometry.size.w / cols).max(1);
    let tile_height = (output_geometry.size.h / rows).max(1);

    Rectangle::new(
        Point::from((
            output_geometry.loc.x + col * tile_width,
            output_geometry.loc.y + row * tile_height,
        )),
        Size::from((tile_width, tile_height)),
    )
}

pub fn client_compositor_state(client: &Client) -> &CompositorClientState {
    &client
        .get_data::<crate::state::ClientState>()
        .expect("missing client state")
        .compositor_state
}
