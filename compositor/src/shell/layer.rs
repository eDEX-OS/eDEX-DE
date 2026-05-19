use smithay::{
    desktop::{layer_map_for_output, LayerSurface},
    reexports::wayland_server::protocol::wl_output::WlOutput,
    wayland::shell::wlr_layer::{Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler},
};

use crate::state::EdexState;

impl WlrLayerShellHandler for EdexState {
    fn shell_state(&mut self) -> &mut smithay::wayland::shell::wlr_layer::WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        wl_output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let desktop_surface = LayerSurface::new(surface, namespace);

        // Assign the surface to the requested output, or the first available output.
        let output = wl_output
            .as_ref()
            .and_then(|o| self.space.outputs().find(|out| out.owns(o)).cloned())
            .or_else(|| self.space.outputs().next().cloned());

        if let Some(output) = output {
            let mut map = layer_map_for_output(&output);
            if let Err(e) = map.map_layer(&desktop_surface) {
                tracing::warn!("failed to map layer surface: {}", e);
            }
            map.arrange();
        } else {
            tracing::warn!("new_layer_surface: no output available to assign surface");
        }

        let _ = desktop_surface.layer_surface().send_pending_configure();
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        // Remove the surface from whichever output it was on.
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            let mut map = layer_map_for_output(&output);
            let found = map
                .layers()
                .find(|l| l.layer_surface() == &surface)
                .cloned();
            if let Some(layer) = found {
                map.unmap_layer(&layer);
                map.arrange();
                break;
            }
        }
    }
}
