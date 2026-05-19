#[derive(Clone, Debug, Default)]
pub struct CellAttributes {
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

#[derive(Clone, Debug, Default)]
pub enum TermColor {
    #[default]
    Default,
    Ansi(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    pub attrs: CellAttributes,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            attrs: CellAttributes::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_visible: bool,
    pub scrollback: Vec<Vec<Cell>>,
    pub scroll_offset: usize,
    pub title: String,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let empty_row = vec![Cell::default(); cols];
        Self {
            cols,
            rows,
            cells: vec![empty_row; rows],
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            scrollback: Vec::new(),
            scroll_offset: 0,
            title: String::from("terminal"),
        }
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.cells.resize(rows, vec![Cell::default(); cols]);
        for row in &mut self.cells {
            row.resize(cols, Cell::default());
        }
        if self.cursor_row >= rows {
            self.cursor_row = rows.saturating_sub(1);
        }
        if self.cursor_col >= cols {
            self.cursor_col = cols.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalGrid;

    #[test]
    fn visible_lines_trim_trailing_spaces() {
        let mut grid = TerminalGrid::new(4, 1);
        grid.cells[0][0].ch = 'o';
        grid.cells[0][1].ch = 'k';
        assert_eq!(grid.visible_lines(), vec!["ok".to_string()]);
    }
}
