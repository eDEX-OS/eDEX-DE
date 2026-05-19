use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_seat},
            Client,
        },
    },
    utils::Serial,
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
        self.tiling.add_window(&mut self.space, window.clone());
        self.focus_window(window);
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
            .tiling
            .windows
            .iter()
            .find(|window| {
                window
                    .toplevel()
                    .map(|toplevel| toplevel.wl_surface() == surface.wl_surface())
                    .unwrap_or(false)
            })
            .cloned();
        if let Some(window) = target {
            self.space.unmap_elem(&window);
            self.tiling.remove_window(&mut self.space, &window);
            if self.focused_window.as_ref() == Some(&window) {
                self.focused_window = self.tiling.windows.last().cloned();
            }
            if let Some(focused) = self.focused_window.clone() {
                self.focus_window(focused);
            }
            self.refresh_layout();
        }
    }
}

pub fn apply_window_layout(state: &mut EdexState) {
    state.refresh_layout();
}

pub fn client_compositor_state(client: &Client) -> &CompositorClientState {
    &client
        .get_data::<crate::state::ClientState>()
        .expect("missing client state")
        .compositor_state
}
