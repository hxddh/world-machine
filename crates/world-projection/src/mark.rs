//! The player's mark on a place: plots to build on, designs painted on a
//! flag, a sail, a sign or a quilt, and names given to boats, works and
//! newborns.
//!
//! All of it is presentation of what a World records, and the words an app
//! sends back: a design and a name travel as the argument of an ordinary
//! command (`<command>=<argument>`), which the World checks like any other
//! and records as an Event. Nothing here decides anything.

use crate::MarkShape;

/// How many cells a design has along each side.
pub const PATTERN_SIDE: usize = 16;
/// How many cells a design has.
pub const PATTERN_CELLS: usize = PATTERN_SIDE * PATTERN_SIDE;
/// The most colours one design may use.
pub const MOST_PATTERN_COLOURS: usize = 8;

/// The fixed palette every design is painted from: sixteen colours a
/// harbour, a colony or a rookery could dye cloth or paint wood with.
pub const PATTERN_PALETTE: [[u8; 3]; 16] = [
    [0xf6, 0xf1, 0xe4], // 0 chalk white
    [0x2b, 0x2a, 0x33], // 1 soot
    [0xc8, 0x3a, 0x32], // 2 signal red
    [0xe8, 0x8a, 0x3c], // 3 marigold
    [0xf2, 0xc9, 0x4c], // 4 buttercup
    [0x6d, 0xa3, 0x4d], // 5 meadow
    [0x2f, 0x6b, 0x4f], // 6 bottle green
    [0x4f, 0xa3, 0xc7], // 7 sky
    [0x25, 0x4e, 0x8c], // 8 harbour blue
    [0x1d, 0x2b, 0x4f], // 9 navy
    [0x8e, 0x5b, 0xa8], // a heather
    [0xe7, 0x9c, 0xb5], // b rose
    [0x8a, 0x5a, 0x3b], // c chestnut
    [0xd9, 0xb8, 0x8f], // d sand
    [0x9a, 0xa0, 0xa6], // e slate
    [0x5e, 0x3a, 0x2e], // f peat
];

/// What a design is painted on.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Wears {
    Flag,
    Sail,
    Sign,
    Quilt,
}

impl Wears {
    pub const ALL: [Wears; 4] = [Wears::Flag, Wears::Sail, Wears::Sign, Wears::Quilt];

    /// The word for it, as a World records it.
    pub fn id(self) -> &'static str {
        match self {
            Wears::Flag => "flag",
            Wears::Sail => "sail",
            Wears::Sign => "sign",
            Wears::Quilt => "quilt",
        }
    }

    pub fn from_id(id: &str) -> Option<Wears> {
        Wears::ALL.into_iter().find(|wears| wears.id() == id)
    }
}

/// A design as it is drawn: 256 cells, each a hex digit indexing
/// `palette`, row by row from the top left, and the colours themselves.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Pattern {
    pub cells: String,
    pub palette: Vec<[u8; 3]>,
}

impl Pattern {
    /// The colour of a cell, `x` across and `y` down.
    pub fn colour_at(&self, x: usize, y: usize) -> Option<[u8; 3]> {
        if x >= PATTERN_SIDE || y >= PATTERN_SIDE {
            return None;
        }
        let digit = self.cells.as_bytes().get(y * PATTERN_SIDE + x)?;
        let index = (*digit as char).to_digit(16)? as usize;
        self.palette.get(index).copied()
    }
}

/// A design as a World keeps it: 256 hex digits indexing its colours, and
/// its colours as places in [`PATTERN_PALETTE`]. Written as one short piece
/// of text, `<cells>:<colours>`, it is the argument of a design command and
/// what the World records.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Design {
    cells: String,
    colours: Vec<u8>,
}

/// Why a design or a name was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkError {
    /// Not 256 cells of hex digits.
    Cells,
    /// No colours, more than eight, a repeated one, or one not in the
    /// palette.
    Colours,
    /// A cell names a colour the design does not have.
    Cell,
    /// Empty, longer than 24 characters, or with a line break or other
    /// control character.
    Name,
}

impl std::fmt::Display for MarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MarkError::Cells => "A design is sixteen by sixteen squares",
            MarkError::Colours => "A design has one to eight colours from the palette",
            MarkError::Cell => "A square is painted a colour the design does not have",
            MarkError::Name => "A name is one to twenty-four letters on one line",
        })
    }
}

impl std::error::Error for MarkError {}

impl Design {
    /// A design from its cells (256 hex digits) and its colours (places in
    /// [`PATTERN_PALETTE`]), if it is a whole one.
    pub fn new(cells: &str, colours: &[u8]) -> Result<Design, MarkError> {
        if cells.len() != PATTERN_CELLS || !cells.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(MarkError::Cells);
        }
        let mut seen = [false; 16];
        if colours.is_empty()
            || colours.len() > MOST_PATTERN_COLOURS
            || colours.iter().any(|colour| {
                let colour = *colour as usize;
                colour >= PATTERN_PALETTE.len() || std::mem::replace(&mut seen[colour], true)
            })
        {
            return Err(MarkError::Colours);
        }
        let cells = cells.to_ascii_lowercase();
        if cells
            .chars()
            .any(|c| c.to_digit(16).unwrap_or(99) as usize >= colours.len())
        {
            return Err(MarkError::Cell);
        }
        Ok(Design {
            cells,
            colours: colours.to_vec(),
        })
    }

    /// A design written as `<cells>:<colours>`, colours as one hex digit
    /// each.
    pub fn parse(text: &str) -> Result<Design, MarkError> {
        let (cells, colours) = text.trim().split_once(':').ok_or(MarkError::Colours)?;
        let colours = colours
            .chars()
            .map(|c| c.to_digit(16).map(|d| d as u8).ok_or(MarkError::Colours))
            .collect::<Result<Vec<_>, _>>()?;
        Design::new(cells, &colours)
    }

    /// The design as one short piece of text: `<cells>:<colours>`.
    pub fn text(&self) -> String {
        let colours = self
            .colours
            .iter()
            .map(|c| char::from_digit(u32::from(*c), 16).unwrap_or('0'))
            .collect::<String>();
        format!("{}:{colours}", self.cells)
    }

    pub fn cells(&self) -> &str {
        &self.cells
    }

    /// Its colours, as places in [`PATTERN_PALETTE`].
    pub fn colours(&self) -> &[u8] {
        &self.colours
    }

    /// How it is drawn.
    pub fn pattern(&self) -> Pattern {
        Pattern {
            cells: self.cells.clone(),
            palette: self
                .colours
                .iter()
                .map(|c| PATTERN_PALETTE[*c as usize])
                .collect(),
        }
    }

    /// The design a drawn pattern is, when every colour it uses is one of
    /// the palette's.
    pub fn from_pattern(pattern: &Pattern) -> Result<Design, MarkError> {
        let colours = pattern
            .palette
            .iter()
            .map(|rgb| {
                PATTERN_PALETTE
                    .iter()
                    .position(|colour| colour == rgb)
                    .map(|at| at as u8)
                    .ok_or(MarkError::Colours)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Design::new(&pattern.cells, &colours)
    }
}

/// A name the player gives, made tidy: trimmed, and refused if it is
/// empty, longer than 24 characters, or holds a line break or any other
/// control character.
pub fn clean_name(name: &str) -> Result<String, MarkError> {
    let name = name.trim();
    let length = name.chars().count();
    if length == 0 || length > 24 || name.chars().any(char::is_control) {
        return Err(MarkError::Name);
    }
    Ok(name.to_string())
}

/// A command with its argument: `<command>=<argument>`. A design command
/// takes a design's text, a naming command a name.
pub fn command_with(command: &str, argument: &str) -> String {
    format!("{command}={argument}")
}

/// A command and its argument, if it has one.
pub fn command_argument(command: &str) -> (&str, Option<&str>) {
    match command.split_once('=') {
        Some((command, argument)) => (command, Some(argument)),
        None => (command, None),
    }
}

/// Something the player can paint a design on: the command a design is
/// sent with (`command_with(command, &design.text())`) and what it is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Designable {
    pub command: String,
    pub wears: Wears,
}

/// Something the player can name: the command a name is sent with
/// (`command_with(command, name)`), and names someone proposes, if anyone
/// does (a newborn's parents).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Naming {
    pub command: String,
    pub proposals: Vec<String>,
}

/// How the town built something the player chose: its colour, which way
/// it faces, and whether it joins what stands either side of it (a wall
/// run on, a path between).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Variant {
    pub colour: [u8; 3],
    /// Faces the other way.
    pub flip: bool,
    pub join_left: bool,
    pub join_right: bool,
}

/// A plot along a path: somewhere free the player can build, and what
/// could stand there.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plot {
    /// Stable for as long as the place is.
    pub id: String,
    /// Where along the panorama it lies, in panorama units.
    pub px: f32,
    /// Which depth row it lies in, as the place's layout counts them from
    /// the back.
    pub row: u8,
    /// The district it lies in.
    pub district: String,
    pub offers: Vec<PlotOffer>,
}

/// Something that could stand on a plot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotOffer {
    /// The command that builds it there, which the World checks like any
    /// deed.
    pub command: String,
    pub label: String,
    pub shape: MarkShape,
    pub cost: Option<String>,
    /// Why it cannot be built now, if it cannot.
    pub unavailable: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stripes() -> Design {
        let cells = (0..PATTERN_CELLS)
            .map(|at| {
                if (at / PATTERN_SIDE).is_multiple_of(2) {
                    '0'
                } else {
                    '1'
                }
            })
            .collect::<String>();
        Design::new(&cells, &[8, 0]).unwrap()
    }

    #[test]
    fn a_design_is_its_text_and_back() {
        let design = stripes();
        let text = design.text();
        assert_eq!(text.len(), 256 + 3);
        assert_eq!(Design::parse(&text).unwrap(), design);
        let pattern = design.pattern();
        assert_eq!(pattern.colour_at(0, 0), Some(PATTERN_PALETTE[8]));
        assert_eq!(pattern.colour_at(3, 1), Some(PATTERN_PALETTE[0]));
        assert_eq!(Design::from_pattern(&pattern).unwrap(), design);
    }

    #[test]
    fn a_broken_design_is_refused() {
        let cells = "0".repeat(256);
        assert_eq!(Design::new(&cells[1..], &[1]), Err(MarkError::Cells));
        assert_eq!(Design::new(&cells, &[]), Err(MarkError::Colours));
        assert_eq!(Design::new(&cells, &[1, 1]), Err(MarkError::Colours));
        assert_eq!(Design::new(&cells, &[16]), Err(MarkError::Colours));
        assert_eq!(
            Design::new(&cells, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            Err(MarkError::Colours)
        );
        let two = format!("2{}", &cells[1..]);
        assert_eq!(Design::new(&two, &[0, 1]), Err(MarkError::Cell));
        assert!(Design::parse("nonsense").is_err());
    }

    #[test]
    fn names_are_tidy() {
        assert_eq!(clean_name("  Stormy Petrel ").unwrap(), "Stormy Petrel");
        assert_eq!(clean_name("小海燕").unwrap(), "小海燕");
        assert!(clean_name("").is_err());
        assert!(clean_name("   ").is_err());
        assert!(clean_name("two\nlines").is_err());
        assert!(clean_name("tab\there").is_err());
        assert!(clean_name(&"x".repeat(25)).is_err());
        assert!(clean_name(&"é".repeat(24)).is_ok());
        let (command, name) = command_argument("pack.name.7=Ann = Bea");
        assert_eq!((command, name), ("pack.name.7", Some("Ann = Bea")));
        assert_eq!(command_with("pack.name.7", "Ann"), "pack.name.7=Ann");
    }
}
