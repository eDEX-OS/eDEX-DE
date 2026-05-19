use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tracing::warn;

use crate::grid::{Cell, CellAttributes, TermColor, TerminalGrid};

pub struct TerminalInstance {
    pub grid: Arc<Mutex<TerminalGrid>>,
    pty_master: Box<dyn MasterPty + Send>,
    pty_writer: Box<dyn Write + Send>,
    _child: Box<dyn portable_pty::Child + Send>,
}

impl TerminalInstance {
    pub fn new(cols: usize, rows: usize) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let grid = Arc::new(Mutex::new(TerminalGrid::new(cols, rows)));
        let grid_clone = Arc::clone(&grid);
        let mut reader = pair.master.try_clone_reader()?;

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut parser = vte::Parser::new();
            let mut performer = VtePerformer::new(grid_clone);

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => parser.advance(&mut performer, &buf[..n]),
                    Err(error) => {
                        warn!(?error, "terminal PTY reader stopped");
                        break;
                    }
                }
            }
        });

        let writer = pair.master.take_writer()?;

        Ok(Self {
            grid,
            pty_master: pair.master,
            pty_writer: writer,
            _child: child,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.pty_writer.write_all(data)?;
        self.pty_writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: usize, rows: usize) -> Result<()> {
        self.pty_master.resize(PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        if let Ok(mut grid) = self.grid.lock() {
            grid.resize(cols, rows);
        }
        Ok(())
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.grid
            .lock()
            .map(|grid| grid.visible_lines())
            .unwrap_or_default()
    }
}

struct VtePerformer {
    grid: Arc<Mutex<TerminalGrid>>,
    attrs: CellAttributes,
}

impl VtePerformer {
    fn new(grid: Arc<Mutex<TerminalGrid>>) -> Self {
        Self {
            grid,
            attrs: CellAttributes::default(),
        }
    }

    fn blank_row(cols: usize) -> Vec<Cell> {
        vec![Cell::default(); cols]
    }

    fn scroll_up(grid: &mut TerminalGrid) {
        if grid.rows == 0 {
            return;
        }
        if !grid.cells.is_empty() {
            let first = grid.cells.remove(0);
            grid.scrollback.push(first);
        }
        grid.cells.push(Self::blank_row(grid.cols));
        grid.cursor_row = grid.rows.saturating_sub(1);
    }

    fn newline(grid: &mut TerminalGrid) {
        grid.cursor_row += 1;
        if grid.cursor_row >= grid.rows {
            Self::scroll_up(grid);
        }
    }

    fn put_cell(&mut self, c: char) {
        if let Ok(mut grid) = self.grid.lock() {
            if grid.cols == 0 || grid.rows == 0 {
                return;
            }
            if grid.cursor_col >= grid.cols {
                grid.cursor_col = 0;
                Self::newline(&mut grid);
            }
            let row = grid.cursor_row.min(grid.rows.saturating_sub(1));
            let col = grid.cursor_col.min(grid.cols.saturating_sub(1));
            grid.cells[row][col] = Cell {
                ch: c,
                attrs: self.attrs.clone(),
            };
            grid.cursor_col += 1;
            if grid.cursor_col >= grid.cols {
                grid.cursor_col = 0;
                Self::newline(&mut grid);
            }
        }
    }

    fn erase_in_display(grid: &mut TerminalGrid, mode: u16) {
        match mode {
            0 => {
                let row = grid.cursor_row;
                let col = grid.cursor_col;
                if row < grid.rows {
                    for cell in grid.cells[row].iter_mut().skip(col) {
                        *cell = Cell::default();
                    }
                }
                for line in grid.cells.iter_mut().skip(row.saturating_add(1)) {
                    line.fill(Cell::default());
                }
            }
            1 => {
                let row = grid.cursor_row;
                let col = grid.cursor_col;
                for line in grid.cells.iter_mut().take(row) {
                    line.fill(Cell::default());
                }
                if row < grid.rows {
                    for cell in grid.cells[row].iter_mut().take(col.saturating_add(1)) {
                        *cell = Cell::default();
                    }
                }
            }
            2 | 3 => {
                for row in &mut grid.cells {
                    row.fill(Cell::default());
                }
                if mode == 3 {
                    grid.scrollback.clear();
                }
                grid.cursor_row = 0;
                grid.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn erase_in_line(grid: &mut TerminalGrid, mode: u16) {
        let row = grid.cursor_row;
        let col = grid.cursor_col;
        if row >= grid.rows {
            return;
        }

        match mode {
            0 => {
                for cell in grid.cells[row].iter_mut().skip(col) {
                    *cell = Cell::default();
                }
            }
            1 => {
                for cell in grid.cells[row].iter_mut().take(col.saturating_add(1)) {
                    *cell = Cell::default();
                }
            }
            2 => grid.cells[row].fill(Cell::default()),
            _ => {}
        }
    }

    fn set_cursor(grid: &mut TerminalGrid, row: usize, col: usize) {
        grid.cursor_row = row.min(grid.rows.saturating_sub(1));
        grid.cursor_col = col.min(grid.cols.saturating_sub(1));
    }

    fn apply_sgr(&mut self, params: &vte::Params) {
        let values: Vec<u16> = params.iter().map(|param| param[0]).collect();
        let values = if values.is_empty() { vec![0] } else { values };

        let mut iter = values.into_iter().peekable();
        while let Some(param) = iter.next() {
            match param {
                0 => self.attrs = CellAttributes::default(),
                1 => self.attrs.bold = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                5 => self.attrs.blink = true,
                7 => self.attrs.reverse = true,
                9 => self.attrs.strikethrough = true,
                22 => self.attrs.bold = false,
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                25 => self.attrs.blink = false,
                27 => self.attrs.reverse = false,
                29 => self.attrs.strikethrough = false,
                30..=37 => self.attrs.fg = TermColor::Ansi((param - 30) as u8),
                38 => {
                    if let Some(color) = parse_extended_color(&mut iter) {
                        self.attrs.fg = color;
                    }
                }
                39 => self.attrs.fg = TermColor::Default,
                40..=47 => self.attrs.bg = TermColor::Ansi((param - 40) as u8),
                48 => {
                    if let Some(color) = parse_extended_color(&mut iter) {
                        self.attrs.bg = color;
                    }
                }
                49 => self.attrs.bg = TermColor::Default,
                90..=97 => self.attrs.fg = TermColor::Ansi((param - 90 + 8) as u8),
                100..=107 => self.attrs.bg = TermColor::Ansi((param - 100 + 8) as u8),
                _ => {}
            }
        }
    }
}

fn parse_extended_color<I>(iter: &mut std::iter::Peekable<I>) -> Option<TermColor>
where
    I: Iterator<Item = u16>,
{
    match iter.next()? {
        2 => {
            let r = iter.next()? as u8;
            let g = iter.next()? as u8;
            let b = iter.next()? as u8;
            Some(TermColor::Rgb(r, g, b))
        }
        5 => iter.next().map(|idx| TermColor::Ansi(idx as u8)),
        _ => None,
    }
}

impl vte::Perform for VtePerformer {
    fn print(&mut self, c: char) {
        self.put_cell(c);
    }

    fn execute(&mut self, byte: u8) {
        if let Ok(mut grid) = self.grid.lock() {
            match byte {
                b'\n' => VtePerformer::newline(&mut grid),
                b'\r' => grid.cursor_col = 0,
                0x08 => grid.cursor_col = grid.cursor_col.saturating_sub(1),
                0x09 => {
                    let next_tab = ((grid.cursor_col / 8) + 1) * 8;
                    grid.cursor_col = next_tab.min(grid.cols.saturating_sub(1));
                }
                _ => {}
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action == 'm' {
            self.apply_sgr(params);
            return;
        }

        if let Ok(mut grid) = self.grid.lock() {
            let ps: Vec<u16> = params.iter().map(|param| param[0]).collect();
            let first = ps.first().copied().unwrap_or(1).max(1) as usize;

            match action {
                'A' => grid.cursor_row = grid.cursor_row.saturating_sub(first),
                'B' => {
                    grid.cursor_row = (grid.cursor_row + first).min(grid.rows.saturating_sub(1));
                }
                'C' => {
                    grid.cursor_col = (grid.cursor_col + first).min(grid.cols.saturating_sub(1));
                }
                'D' => grid.cursor_col = grid.cursor_col.saturating_sub(first),
                'G' => {
                    grid.cursor_col = ps.first().copied().unwrap_or(1).saturating_sub(1) as usize;
                    grid.cursor_col = grid.cursor_col.min(grid.cols.saturating_sub(1));
                }
                'H' | 'f' => {
                    let row = ps.first().copied().unwrap_or(1).saturating_sub(1) as usize;
                    let col = ps.get(1).copied().unwrap_or(1).saturating_sub(1) as usize;
                    VtePerformer::set_cursor(&mut grid, row, col);
                }
                'J' => VtePerformer::erase_in_display(&mut grid, ps.first().copied().unwrap_or(0)),
                'K' => VtePerformer::erase_in_line(&mut grid, ps.first().copied().unwrap_or(0)),
                'd' => {
                    let row = ps.first().copied().unwrap_or(1).saturating_sub(1) as usize;
                    grid.cursor_row = row.min(grid.rows.saturating_sub(1));
                }
                'h' => grid.cursor_visible = true,
                'l' => grid.cursor_visible = false,
                _ => {}
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        if byte == b'c' {
            if let Ok(mut grid) = self.grid.lock() {
                for row in &mut grid.cells {
                    row.fill(Cell::default());
                }
                grid.scrollback.clear();
                grid.cursor_row = 0;
                grid.cursor_col = 0;
                grid.cursor_visible = true;
                grid.title = "terminal".to_string();
            }
            self.attrs = CellAttributes::default();
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 && matches!(params[0], b"0" | b"2") {
            if let Ok(title) = std::str::from_utf8(params[1]) {
                if let Ok(mut grid) = self.grid.lock() {
                    grid.title = title.to_string();
                }
            }
        }
    }

    fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
}
