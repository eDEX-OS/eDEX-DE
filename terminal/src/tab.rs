use anyhow::Result;

use crate::instance::TerminalInstance;

pub struct TerminalTabs {
    pub tabs: Vec<TerminalInstance>,
    pub active: usize,
}

impl TerminalTabs {
    pub fn new(cols: usize, rows: usize) -> Result<Self> {
        let first = TerminalInstance::new(cols, rows)?;
        Ok(Self {
            tabs: vec![first],
            active: 0,
        })
    }

    pub fn active_mut(&mut self) -> &mut TerminalInstance {
        &mut self.tabs[self.active]
    }

    pub fn active_ref(&self) -> &TerminalInstance {
        &self.tabs[self.active]
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

    pub fn new_tab(&mut self, cols: usize, rows: usize) -> Result<()> {
        let inst = TerminalInstance::new(cols, rows)?;
        self.tabs.push(inst);
        self.active = self.tabs.len() - 1;
        Ok(())
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() || self.tabs.len() == 1 {
            return;
        }

        self.tabs.remove(idx);
        if self.active > idx {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    pub fn switch(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
        }
    }

    pub fn resize_all(&mut self, cols: usize, rows: usize) -> Result<()> {
        for tab in &mut self.tabs {
            tab.resize(cols, rows)?;
        }
        Ok(())
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.active_ref().visible_lines()
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.active_mut().write_input(data)
    }
}
