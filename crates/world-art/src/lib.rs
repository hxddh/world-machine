//! The art catalog: what World Machine's library of drawings knows about
//! each drawing besides how to paint it, held as data in one table
//! (`catalog.tsv`) that the app's painter (world-gpui) and the Systems that
//! site things (`systems/days`) both read.
//!
//! A row says where a drawing is at home (its family), how tall it stands
//! beside a resident (its rung on the art bible's ladder, §3), where it is
//! sited from the water to the hills (its zone, §2), and whether it is a
//! thing of gardens and fields. Before v0.28 each of those facts lived in
//! its own hand-written list in one of two crates, and adding a drawing
//! needed edits in three places.
//!
//! The vocabulary a Pack speaks to the app about its look is here too:
//! the [`Rung`] a Pack declares for its own drawings, and the [`Ground`] a
//! cluster of works stands on. Nothing here knows GPUI, a World or a Pack.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

/// Where a drawing is at home: anywhere, or in one kind of place.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Family {
    /// At home anywhere: a bench, a bandstand, a swing.
    Any,
    Harbour,
    Mars,
    Street,
    Ice,
}

impl Family {
    pub fn of_name(name: &str) -> Option<Self> {
        Some(match name {
            "any" => Family::Any,
            "harbour" => Family::Harbour,
            "mars" => Family::Mars,
            "street" => Family::Street,
            "ice" => Family::Ice,
            _ => return None,
        })
    }
}

/// A rung of the art bible's ladder: how tall something stands beside a
/// grown-up resident at the same depth, in **P** (that resident's height),
/// or for a long, low thing (a pier, a pond, a court) how wide it lies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rung {
    Tall(f32),
    Wide(f32),
}

impl Rung {
    /// Reads "tall 3.3" or "wide 4.0".
    pub fn of_text(text: &str) -> Option<Self> {
        let (kind, value) = text.split_once(' ')?;
        let value = value.trim().parse::<f32>().ok()?;
        if !(value.is_finite() && value > 0.0 && value <= 20.0) {
            return None;
        }
        match kind {
            "tall" => Some(Rung::Tall(value)),
            "wide" => Some(Rung::Wide(value)),
            _ => None,
        }
    }

    /// Whether a Pack's declared rung is one the app can stand by: a
    /// finite size between a pebble's and a tall tower's.
    pub fn is_sensible(self) -> bool {
        let value = match self {
            Rung::Tall(value) | Rung::Wide(value) => value,
        };
        value.is_finite() && value > 0.0 && value <= 20.0
    }
}

/// Where a thing belongs, from the water to the hills (the art bible's
/// siting zones): a pier never stands on a hill.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Zone {
    /// On the water line: pier, jetty, slipway, boathouse, moorings, the
    /// tower on the point.
    Water,
    /// On the quay, the spine people walk: stalls, carts, crates, benches,
    /// lamp posts.
    Quay,
    /// In the lanes: homes, shops, gardens.
    Lanes,
    /// On the green: the bandstand, maypole, well and fountain.
    Green,
    /// At the edges and up the hill: the windmill, telescope, chapel,
    /// orchard, beehives, lookout.
    Edge,
}

impl Zone {
    pub fn of_name(name: &str) -> Option<Option<Self>> {
        Some(Some(match name {
            "water" => Zone::Water,
            "quay" => Zone::Quay,
            "lanes" => Zone::Lanes,
            "green" => Zone::Green,
            "edge" => Zone::Edge,
            "-" => return Some(None),
            _ => return None,
        }))
    }
}

/// The ground a cluster of works and homes shares, as a Pack names it and
/// the app paints it. The set is closed: a ground the app cannot paint is
/// an error where the Pack's snapshot is read, never quietly plain grass.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Ground {
    Cobbles,
    Plaza,
    Garden,
    Green,
    Yard,
    Pad,
    Paving,
    Snow,
    Rock,
    /// Plain ground worn by feet: what a cluster stands on when its Pack
    /// says nothing more.
    #[default]
    Worn,
}

impl Ground {
    pub const ALL: [Ground; 10] = [
        Ground::Cobbles,
        Ground::Plaza,
        Ground::Garden,
        Ground::Green,
        Ground::Yard,
        Ground::Pad,
        Ground::Paving,
        Ground::Snow,
        Ground::Rock,
        Ground::Worn,
    ];

    /// The name a Pack sends.
    pub fn name(self) -> &'static str {
        match self {
            Ground::Cobbles => "cobbles",
            Ground::Plaza => "plaza",
            Ground::Garden => "garden",
            Ground::Green => "green",
            Ground::Yard => "yard",
            Ground::Pad => "pad",
            Ground::Paving => "paving",
            Ground::Snow => "snow",
            Ground::Rock => "rock",
            Ground::Worn => "worn",
        }
    }

    /// The ground a Pack's name stands for, if the app knows it.
    pub fn of_name(name: &str) -> Option<Self> {
        Ground::ALL.into_iter().find(|ground| ground.name() == name)
    }
}

/// One drawing of the library.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub key: &'static str,
    pub family: Family,
    pub rung: Rung,
    /// Where it is sited; none sites it by its shape.
    pub zone: Option<Zone>,
    /// A thing of gardens and fields: never taller than a grown-up.
    pub field: bool,
}

/// The table itself.
const TABLE: &str = include_str!("catalog.tsv");

/// Reads the table. A row that cannot be read is a mistake in the table,
/// and stops everything with its line number: the catalog is the app's own
/// data, so it is never half-read.
fn read(table: &'static str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for (number, line) in table.lines().enumerate() {
        let number = number + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        let [key, family, rung, zone, flags] = columns[..] else {
            return Err(format!(
                "catalog line {number}: five columns, tab-separated"
            ));
        };
        let family = Family::of_name(family)
            .ok_or_else(|| format!("catalog line {number}: no family {family:?}"))?;
        let rung = Rung::of_text(rung)
            .ok_or_else(|| format!("catalog line {number}: no rung {rung:?}"))?;
        let zone = Zone::of_name(zone)
            .ok_or_else(|| format!("catalog line {number}: no zone {zone:?}"))?;
        let mut field = false;
        for flag in flags.split(',') {
            match flag {
                "field" => field = true,
                "-" => {}
                other => return Err(format!("catalog line {number}: no flag {other:?}")),
            }
        }
        if key.is_empty() || key.contains(char::is_whitespace) {
            return Err(format!("catalog line {number}: a key is one word"));
        }
        entries.push(Entry {
            key,
            family,
            rung,
            zone,
            field,
        });
    }
    Ok(entries)
}

struct Catalog {
    entries: Vec<Entry>,
    by_key: HashMap<&'static str, usize>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let entries = read(TABLE).unwrap_or_else(|error| panic!("{error}"));
        let by_key = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.key, index))
            .collect();
        Catalog { entries, by_key }
    })
}

/// Every drawing in the catalog, in the table's order.
pub fn entries() -> &'static [Entry] {
    &catalog().entries
}

/// The catalog's row for a library key.
pub fn entry(key: &str) -> Option<&'static Entry> {
    let catalog = catalog();
    catalog
        .by_key
        .get(key)
        .map(|index| &catalog.entries[*index])
}

/// How tall a library drawing stands, or how wide it lies.
pub fn rung(key: &str) -> Option<Rung> {
    entry(key).map(|entry| entry.rung)
}

/// Where a library drawing is sited, if its row says; otherwise by its
/// shape, which the caller knows.
pub fn zone(key: &str) -> Option<Zone> {
    entry(key).and_then(|entry| entry.zone)
}

/// Whether a library drawing is a thing of gardens and fields.
pub fn is_field(key: &str) -> bool {
    entry(key).is_some_and(|entry| entry.field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_reads_whole_with_one_row_a_key() {
        let entries = entries();
        assert!(entries.len() > 400, "{} rows", entries.len());
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            assert!(seen.insert(entry.key), "{} has two rows", entry.key);
        }
    }

    #[test]
    fn a_bad_row_is_refused_with_its_line() {
        for (table, says) in [
            ("bench\tany\ttall 0.8\tquay\n", "five columns"),
            ("bench\tanywhere\ttall 0.8\tquay\t-\n", "no family"),
            ("bench\tany\ttall\tquay\t-\n", "no rung"),
            ("bench\tany\ttall -1\tquay\t-\n", "no rung"),
            ("bench\tany\ttall 0.8\tpond\t-\n", "no zone"),
            ("bench\tany\ttall 0.8\tquay\tshiny\n", "no flag"),
        ] {
            let error = read(table).expect_err(table);
            assert!(error.contains("line 1") && error.contains(says), "{error}");
        }
    }

    #[test]
    fn things_of_fields_stand_no_taller_than_a_grown_up() {
        for entry in entries().iter().filter(|entry| entry.field) {
            if let Rung::Tall(tall) = entry.rung {
                assert!(tall <= 1.0, "{} stands {tall} P in a field", entry.key);
            }
        }
    }

    #[test]
    fn every_ground_reads_back_from_its_name() {
        for ground in Ground::ALL {
            assert_eq!(Ground::of_name(ground.name()), Some(ground));
        }
        assert_eq!(Ground::of_name("lava"), None);
    }

    #[test]
    fn rungs_read_from_text() {
        assert_eq!(Rung::of_text("tall 3.3"), Some(Rung::Tall(3.3)));
        assert_eq!(Rung::of_text("wide 4"), Some(Rung::Wide(4.0)));
        assert_eq!(Rung::of_text("tall NaN"), None);
        assert!(!Rung::Tall(f32::INFINITY).is_sensible());
    }
}
