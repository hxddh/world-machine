//! The design canvas as the player works on it: a 16 by 16 grid, eight
//! colours chosen from the fixed palette, a pencil, a fill and an
//! eyedropper, undo and redo. Presentation state only; nothing reaches the
//! World until the player saves, and then only as a design command the
//! World checks like any other.

use crate::mark::{Motif, CELLS, PALETTE, SIDE, SLOTS, STARTING_COLOURS};

/// What a click or Space does on the grid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Tool {
    /// Paints the one square.
    #[default]
    Pencil,
    /// Paints every square of the same colour joined to it.
    Fill,
    /// Takes the square's colour as the one to paint with.
    Eyedropper,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Pencil, Tool::Fill, Tool::Eyedropper];

    /// Its name, with the key that picks it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil (P)",
            Self::Fill => "Fill (F)",
            Self::Eyedropper => "Eyedropper (I)",
        }
    }
}

/// One state of the canvas that undo goes back to.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Kept {
    cells: [u8; CELLS],
    colours: [u8; SLOTS],
}

/// A design being made.
#[derive(Clone, Debug, PartialEq)]
pub struct Sketch {
    /// Each square's colour, as one of the eight slots.
    pub cells: [u8; CELLS],
    /// Each slot's colour, as a place in the fixed palette.
    pub colours: [u8; SLOTS],
    /// The square the keys are on: column, row.
    pub cursor: (usize, usize),
    /// The slot painted with.
    pub slot: u8,
    pub tool: Tool,
    undo: Vec<Kept>,
    redo: Vec<Kept>,
    /// How it began, to tell whether anything changed.
    began: Kept,
}

/// How many steps undo remembers.
const UNDO_DEPTH: usize = 200;

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Sketch {
    /// A blank design in chalk white, the pencil in hand, the second colour
    /// chosen.
    pub fn new() -> Self {
        let kept = Kept {
            cells: [0; CELLS],
            colours: STARTING_COLOURS,
        };
        Self {
            cells: kept.cells,
            colours: kept.colours,
            cursor: (SIDE / 2, SIDE / 2),
            slot: 1,
            tool: Tool::Pencil,
            undo: Vec::new(),
            redo: Vec::new(),
            began: kept,
        }
    }

    /// The design something already wears, to work on further: its colours
    /// in the first slots, and the starting colours after them.
    pub fn from_motif(motif: &Motif) -> Self {
        let mut sketch = Self::new();
        let mut colours = Vec::<u8>::new();
        let mut slot_of = Vec::with_capacity(motif.palette.len());
        for rgb in &motif.palette {
            let place = nearest(*rgb);
            let slot = match colours.iter().position(|c| *c == place) {
                Some(slot) => slot,
                None if colours.len() < SLOTS => {
                    colours.push(place);
                    colours.len() - 1
                }
                None => 0,
            };
            slot_of.push(slot as u8);
        }
        for place in STARTING_COLOURS {
            if colours.len() < SLOTS && !colours.contains(&place) {
                colours.push(place);
            }
        }
        for (slot, place) in colours.into_iter().enumerate().take(SLOTS) {
            sketch.colours[slot] = place;
        }
        for (cell, index) in sketch.cells.iter_mut().zip(motif.cells) {
            *cell = slot_of.get(index as usize).copied().unwrap_or(0);
        }
        sketch.began = sketch.kept();
        sketch
    }

    fn kept(&self) -> Kept {
        Kept {
            cells: self.cells,
            colours: self.colours,
        }
    }

    /// Whether anything has changed since it was opened.
    pub fn changed(&self) -> bool {
        self.kept() != self.began
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Remembers the canvas as it is, before a change: a stroke, a fill, a
    /// clear.
    pub fn begin(&mut self) {
        let kept = self.kept();
        if self.undo.last() != Some(&kept) {
            self.undo.push(kept);
            if self.undo.len() > UNDO_DEPTH {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some(kept) = self.undo.pop() {
            self.redo.push(self.kept());
            self.cells = kept.cells;
            self.colours = kept.colours;
        }
    }

    pub fn redo(&mut self) {
        if let Some(kept) = self.redo.pop() {
            self.undo.push(self.kept());
            self.cells = kept.cells;
            self.colours = kept.colours;
        }
    }

    /// Everything back to the first colour.
    pub fn clear(&mut self) {
        if self.cells.iter().all(|cell| *cell == 0) {
            return;
        }
        self.begin();
        self.cells = [0; CELLS];
    }

    /// The colour of the square in `column` and `row`, as a slot.
    pub fn at(&self, column: usize, row: usize) -> u8 {
        self.cells[row.min(SIDE - 1) * SIDE + column.min(SIDE - 1)]
    }

    /// Paints one square with the chosen colour, as part of a stroke
    /// already begun.
    pub fn put(&mut self, column: usize, row: usize) {
        if column < SIDE && row < SIDE {
            self.cells[row * SIDE + column] = self.slot;
        }
    }

    /// Paints every square of the same colour joined to (`column`, `row`)
    /// with the chosen colour.
    pub fn fill(&mut self, column: usize, row: usize) {
        if column >= SIDE || row >= SIDE {
            return;
        }
        let from = self.at(column, row);
        if from == self.slot {
            return;
        }
        self.begin();
        let mut open = vec![(column, row)];
        while let Some((x, y)) = open.pop() {
            if self.cells[y * SIDE + x] != from {
                continue;
            }
            self.cells[y * SIDE + x] = self.slot;
            if x > 0 {
                open.push((x - 1, y));
            }
            if x + 1 < SIDE {
                open.push((x + 1, y));
            }
            if y > 0 {
                open.push((x, y - 1));
            }
            if y + 1 < SIDE {
                open.push((x, y + 1));
            }
        }
    }

    /// What the tool in hand does at a square, as a click or Space: the
    /// pencil begins a stroke there, the fill fills, the eyedropper takes
    /// the colour and hands back the pencil.
    pub fn apply(&mut self, column: usize, row: usize) {
        if column >= SIDE || row >= SIDE {
            return;
        }
        self.cursor = (column, row);
        match self.tool {
            Tool::Pencil => {
                if self.at(column, row) != self.slot {
                    self.begin();
                    self.put(column, row);
                }
            }
            Tool::Fill => self.fill(column, row),
            Tool::Eyedropper => {
                self.slot = self.at(column, row);
                self.tool = Tool::Pencil;
            }
        }
    }

    /// Moves the keys' square by (`dx`, `dy`), staying on the grid.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let clamp = |at: usize, by: i32| (at as i32 + by).clamp(0, SIDE as i32 - 1) as usize;
        self.cursor = (clamp(self.cursor.0, dx), clamp(self.cursor.1, dy));
    }

    /// Chooses the colour in slot `slot` (0 to 7) to paint with.
    pub fn choose(&mut self, slot: usize) {
        if slot < SLOTS {
            self.slot = slot as u8;
            if self.tool == Tool::Eyedropper {
                self.tool = Tool::Pencil;
            }
        }
    }

    /// Mixes the chosen slot to the palette's colour `place`: every square
    /// painted with it changes too.
    pub fn mix(&mut self, place: u8) {
        let slot = self.slot as usize;
        if (place as usize) < PALETTE.len() && self.colours[slot] != place {
            self.begin();
            self.colours[slot] = place;
        }
    }

    /// The design as the painter and the World read it: only the colours
    /// it uses (a colour in two slots counted once), in the order first
    /// used.
    pub fn motif(&self) -> Motif {
        let mut places = Vec::<u8>::new();
        let mut index_of = [0_u8; SLOTS];
        let mut seen = [false; SLOTS];
        for cell in self.cells {
            let slot = cell as usize % SLOTS;
            if seen[slot] {
                continue;
            }
            seen[slot] = true;
            let place = self.colours[slot];
            index_of[slot] = match places.iter().position(|p| *p == place) {
                Some(at) => at as u8,
                None => {
                    places.push(place);
                    (places.len() - 1) as u8
                }
            };
        }
        let mut cells = [0_u8; CELLS];
        for (out, cell) in cells.iter_mut().zip(self.cells) {
            *out = index_of[cell as usize % SLOTS];
        }
        Motif {
            cells,
            palette: places.iter().map(|p| PALETTE[*p as usize]).collect(),
        }
    }
}

/// The palette's place nearest a colour.
fn nearest(rgb: [u8; 3]) -> u8 {
    PALETTE
        .iter()
        .enumerate()
        .min_by_key(|(_, colour)| {
            colour
                .iter()
                .zip(rgb)
                .map(|(a, b)| (*a as i32 - b as i32).pow(2))
                .sum::<i32>()
        })
        .map(|(at, _)| at as u8)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_and_a_fill_undo_one_step_at_a_time() {
        let mut sketch = Sketch::new();
        assert!(!sketch.changed());
        sketch.apply(0, 0);
        sketch.put(1, 0);
        sketch.put(2, 0);
        assert_eq!(sketch.at(2, 0), 1);
        sketch.choose(2);
        sketch.tool = Tool::Fill;
        sketch.apply(8, 8);
        assert_eq!(sketch.at(15, 15), 2);
        assert_eq!(sketch.at(1, 0), 1, "a fill stops at another colour");
        sketch.undo();
        assert_eq!(sketch.at(15, 15), 0);
        assert_eq!(sketch.at(2, 0), 1);
        sketch.undo();
        assert_eq!(sketch.at(2, 0), 0, "the whole stroke is one step");
        assert!(!sketch.changed());
        sketch.redo();
        sketch.redo();
        assert_eq!(sketch.at(15, 15), 2);
        sketch.clear();
        assert_eq!(sketch.at(15, 15), 0);
        sketch.undo();
        assert_eq!(sketch.at(15, 15), 2);
    }

    #[test]
    fn the_eyedropper_takes_a_colour_and_hands_back_the_pencil() {
        let mut sketch = Sketch::new();
        sketch.choose(5);
        sketch.apply(3, 3);
        sketch.choose(0);
        sketch.tool = Tool::Eyedropper;
        sketch.apply(3, 3);
        assert_eq!((sketch.slot, sketch.tool), (5, Tool::Pencil));
    }

    #[test]
    fn the_cursor_stays_on_the_grid() {
        let mut sketch = Sketch::new();
        sketch.move_cursor(-40, 3);
        assert_eq!(sketch.cursor, (0, 11));
        sketch.move_cursor(99, 99);
        assert_eq!(sketch.cursor, (15, 15));
    }

    #[test]
    fn a_design_keeps_only_the_colours_it_uses_and_comes_back_the_same() {
        let mut sketch = Sketch::new();
        sketch.choose(3);
        sketch.apply(0, 0);
        sketch.choose(6);
        sketch.apply(1, 0);
        // Two slots mixed to the same colour count once.
        sketch.choose(7);
        sketch.mix(sketch.colours[3]);
        sketch.apply(2, 0);
        let motif = sketch.motif();
        assert_eq!(motif.palette.len(), 3, "{:?}", motif.palette);
        assert_eq!(motif.colour(0, 0), motif.colour(2, 0));
        let design = world_projection::Design::from_pattern(&motif.pattern()).expect("a design");
        let again = Sketch::from_motif(&Motif::of(&design.pattern()).unwrap());
        assert_eq!(again.motif(), motif);
        assert!(!again.changed());
    }
}
