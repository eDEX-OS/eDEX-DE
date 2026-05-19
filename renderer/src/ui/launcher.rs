#[derive(Clone, Debug, Default)]
pub struct LauncherState {
    pub visible: bool,
    pub query: String,
    pub results: Vec<LauncherResult>,
    pub selected: usize,
}

#[derive(Clone, Debug)]
pub struct LauncherResult {
    pub name: String,
    pub comment: Option<String>,
    pub icon: Option<String>,
}

impl LauncherState {
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }
}
