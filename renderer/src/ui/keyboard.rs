use super::panels::Rectangle;

#[derive(Clone, Debug)]
pub struct KeyDef {
    pub label: &'static str,
    pub width_units: u8,
}

#[derive(Clone, Debug)]
pub struct HexKeyboard {
    pub rows: Vec<Vec<KeyDef>>,
    pub hover_key: Option<(usize, usize)>,
    pub pressed_key: Option<(usize, usize)>,
}

impl HexKeyboard {
    pub fn new() -> Self {
        Self {
            rows: vec![
                vec![
                    KeyDef {
                        label: "`",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "1",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "2",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "3",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "4",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "5",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "6",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "7",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "8",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "9",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "0",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "-",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "=",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "Backspace",
                        width_units: 2,
                    },
                ],
                vec![
                    KeyDef {
                        label: "Tab",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Q",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "W",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "E",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "R",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "T",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "Y",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "U",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "I",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "O",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "P",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "[",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "]",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "\\",
                        width_units: 1,
                    },
                ],
                vec![
                    KeyDef {
                        label: "CapsLk",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "A",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "S",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "D",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "F",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "G",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "H",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "J",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "K",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "L",
                        width_units: 1,
                    },
                    KeyDef {
                        label: ";",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "'",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "Enter",
                        width_units: 2,
                    },
                ],
                vec![
                    KeyDef {
                        label: "Shift",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Z",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "X",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "C",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "V",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "B",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "N",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "M",
                        width_units: 1,
                    },
                    KeyDef {
                        label: ",",
                        width_units: 1,
                    },
                    KeyDef {
                        label: ".",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "/",
                        width_units: 1,
                    },
                    KeyDef {
                        label: "Shift",
                        width_units: 2,
                    },
                ],
                vec![
                    KeyDef {
                        label: "Ctrl",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Meta",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Alt",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Space",
                        width_units: 6,
                    },
                    KeyDef {
                        label: "Alt",
                        width_units: 2,
                    },
                    KeyDef {
                        label: "Ctrl",
                        width_units: 2,
                    },
                ],
            ],
            hover_key: None,
            pressed_key: None,
        }
    }

    pub fn key_rect(&self, panel: &Rectangle, row: usize, col: usize) -> Rectangle {
        let outer_padding = 14.0;
        let gap = 8.0;
        let row_count = self.rows.len().max(1) as f32;
        let inner_height = (panel.height as f32 - outer_padding * 2.0).max(1.0);
        let key_height = ((inner_height - gap * (row_count - 1.0)) / row_count).max(18.0);
        let row_defs = &self.rows[row];
        let total_units = row_defs
            .iter()
            .map(|key| key.width_units as u32)
            .sum::<u32>() as f32;
        let key_count = row_defs.len() as f32;
        let inner_width = (panel.width as f32 - outer_padding * 2.0).max(1.0);
        let unit_width = ((inner_width - gap * (key_count - 1.0)) / total_units.max(1.0)).max(12.0);
        let row_offset = if row % 2 == 1 { unit_width * 0.35 } else { 0.0 };

        let x_units = row_defs
            .iter()
            .take(col)
            .map(|key| key.width_units as f32)
            .sum::<f32>();
        let x =
            panel.x as f32 + outer_padding + row_offset + x_units * unit_width + col as f32 * gap;
        let y = panel.y as f32 + outer_padding + row as f32 * (key_height + gap);
        let width = row_defs[col].width_units as f32 * unit_width
            + gap * (row_defs[col].width_units as f32 - 1.0);

        Rectangle::new(
            x.round().max(0.0) as u32,
            y.round().max(0.0) as u32,
            width.round().max(1.0) as u32,
            key_height.round().max(1.0) as u32,
        )
    }
}

impl Default for HexKeyboard {
    fn default() -> Self {
        Self::new()
    }
}
