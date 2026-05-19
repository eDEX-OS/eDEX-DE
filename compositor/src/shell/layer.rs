use smithay::{
    reexports::wayland_server::protocol::wl_output::WlOutput,
    wayland::shell::wlr_layer::{Layer, LayerSurface, WlrLayerShellHandler},
};

use crate::state::EdexState;

impl WlrLayerShellHandler for EdexState {
    fn shell_state(&mut self) -> &mut smithay::wayland::shell::wlr_layer::WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        _output: Option<WlOutput>,
        _layer: Layer,
        _namespace: String,
    ) {
        let _ = surface.send_pending_configure();
    }

    fn layer_destroyed(&mut self, _surface: LayerSurface) {}
}
