use smithay::{
    desktop::{Space, Window},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Rectangle, Size},
};

#[derive(Debug)]
pub struct TilingLayout {
    pub screen_size: Size<i32, Logical>,
    pub windows: Vec<Window>,
}

impl TilingLayout {
    pub fn new(screen_size: Size<i32, Logical>) -> Self {
        Self {
            screen_size,
            windows: Vec::new(),
        }
    }

    pub fn add_window(&mut self, space: &mut Space<Window>, window: Window) {
        self.windows.push(window);
        self.retile(space);
    }

    pub fn remove_window(&mut self, space: &mut Space<Window>, window: &Window) {
        self.windows.retain(|tracked| tracked != window);
        self.retile(space);
    }

    pub fn retile(&mut self, space: &mut Space<Window>) {
        let n = self.windows.len();
        if n == 0 {
            return;
        }

        if n == 1 {
            let geo = Rectangle::new((0, 0).into(), self.screen_size);
            let window = self.windows[0].clone();
            space.map_element(window.clone(), geo.loc, true);
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.size = Some(geo.size);
                    state.states.set(xdg_toplevel::State::Fullscreen);
                    state.states.set(xdg_toplevel::State::Activated);
                });
                toplevel.send_configure();
            }
            return;
        }

        let col_w = self.screen_size.w / n as i32;
        for (index, window) in self.windows.iter().enumerate() {
            let x = index as i32 * col_w;
            let w = if index + 1 == n {
                self.screen_size.w - x
            } else {
                col_w
            };
            let geo = Rectangle::new((x, 0).into(), (w, self.screen_size.h).into());
            space.map_element(window.clone(), geo.loc, index + 1 == n);
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.size = Some(geo.size);
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.states.set(xdg_toplevel::State::Activated);
                });
                toplevel.send_configure();
            }
        }
    }
}
