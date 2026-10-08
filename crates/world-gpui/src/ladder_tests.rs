//! The ladder's tests: every drawing of the library, the plain shapes
//! and the bible's own table, read off the paint.

use super::*;

const P: f32 = 60.0;

/// Paints `subject` sized for `P` and reads how tall it stands, in P.
fn height_in_p(subject: &Subject) -> f32 {
    let (w, h) = subject.sized(P, P * 3.0).expect("a rung");
    let (_, up) = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
        .expect("it paints something");
    up / P
}

fn width_in_p(subject: &Subject) -> f32 {
    let (w, h) = subject.sized(P, P * 3.0).expect("a rung");
    let (across, _) = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
        .expect("it paints something");
    across / P
}

fn holds(subject: &Subject, name: &str) -> Option<String> {
    let rung = subject.rung()?;
    let (got, want) = match rung {
        Rung::Tall(want) => (height_in_p(subject), want),
        Rung::Wide(want) => (width_in_p(subject), want),
    };
    ((got - want).abs() > want * 0.1 + 0.05)
        .then(|| format!("{name}: {got:.2} P, its rung is {rung:?}"))
}

#[test]
fn every_drawing_has_a_rung() {
    let missing = Art::all()
        .filter(|art| of_art(art.key()).is_none())
        .map(|art| art.key())
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "no rung for {missing:?}");
}

/// Reads every drawing's height in P as it is painted, as a thing and
/// as a building, and holds it to its rung.
#[test]
fn every_drawing_stands_on_its_rung() {
    let mut wrong = Vec::new();
    for art in Art::all() {
        let setting = art.family().setting();
        for subject in [
            Subject::Thing(MarkShape::Parcel, setting, Some(art)),
            Subject::Building(MarkShape::House, setting, Some(art)),
        ] {
            wrong.extend(holds(&subject, art.key()));
        }
    }
    use MarkShape as M;
    // A lamp in the harbour is a place (a lit house), never a thing.
    for shape in [
        M::House,
        M::Shop,
        M::Tower,
        M::Dome,
        M::Tree,
        M::Lamp,
        M::Bridge,
        M::Rover,
        M::Boat,
        M::Parcel,
        M::Stall,
        M::Bunting,
        M::Pier,
        M::Garden,
        M::Flag,
        M::Lantern,
        M::Tent,
        M::Bench,
        M::Sprouts,
        M::Well,
        M::Swing,
        M::Fountain,
        M::Signpost,
        M::Birdhouse,
        M::Planter,
        M::Statue,
        M::Postbox,
    ] {
        for setting in Setting::ALL {
            if (shape, setting) == (M::Lamp, Setting::Harbour) {
                continue;
            }
            wrong.extend(holds(
                &Subject::Thing(shape, setting, None),
                &format!("{shape:?} in {setting:?}"),
            ));
        }
        wrong.extend(holds(
            &Subject::Building(shape, Setting::Harbour, None),
            &format!("{shape:?} as a building"),
        ));
    }
    assert!(wrong.is_empty(), "off the ladder:\n{}", wrong.join("\n"));
}

/// The bible's own table, read off the paint.
#[test]
fn the_bibles_ladder_holds() {
    let thing = |key: &str| {
        let art = Art::from_key(key).unwrap();
        height_in_p(&Subject::Thing(
            MarkShape::Parcel,
            art.family().setting(),
            Some(art),
        ))
    };
    let building = |key: &str| {
        let art = Art::from_key(key).unwrap();
        height_in_p(&Subject::Building(
            MarkShape::House,
            art.family().setting(),
            Some(art),
        ))
    };
    let cottage = height_in_p(&Subject::Building(MarkShape::House, Setting::Harbour, None));
    assert!((cottage - 2.75).abs() < 0.3, "a cottage is {cottage:.2} P");
    for postbox in ["pillar-box", "lamp-box", "mailbox"] {
        let got = thing(postbox);
        assert!(got <= 0.8, "a postbox ({postbox}) is {got:.2} P");
    }
    let telescope = thing("telescope");
    assert!(
        telescope <= 0.6 * cottage,
        "a telescope is {telescope:.2} P to a cottage's {cottage:.2}"
    );
    for (key, want) in [
        ("park-bench", 0.8),
        ("wellhead", 1.0),
        ("market-stall", 1.3),
        ("lamp-post", 1.8),
        ("maypole", 3.5),
        ("rowing-boat", 0.5),
    ] {
        let got = thing(key);
        assert!(
            (got - want).abs() <= want * 0.12,
            "{key} is {got:.2} P, not {want}"
        );
    }
    for (key, want) in [("windmill", 5.0), ("lighthouse", 6.0), ("bookshop", 3.0)] {
        let got = building(key);
        assert!(
            (got - want).abs() <= want * 0.12,
            "{key} is {got:.2} P, not {want}"
        );
    }
}

/// The art catalog and the library are one: every row of the catalog has
/// a painter, and every painter a row.
#[test]
fn the_catalog_and_the_library_are_one() {
    let painted = Art::all()
        .map(|art| art.key())
        .collect::<std::collections::BTreeSet<_>>();
    let catalogued = world_art::entries()
        .iter()
        .map(|entry| entry.key)
        .collect::<std::collections::BTreeSet<_>>();
    let unpainted = catalogued.difference(&painted).collect::<Vec<_>>();
    let uncatalogued = painted.difference(&catalogued).collect::<Vec<_>>();
    assert!(
        unpainted.is_empty(),
        "catalogued with no painter: {unpainted:?}"
    );
    assert!(
        uncatalogued.is_empty(),
        "painted with no catalog row: {uncatalogued:?}"
    );
}

/// A Pack says how tall its own drawings stand: a drawing the app has
/// never heard of stands on the rung its Pack declares, and one that
/// declares none keeps the size its spot gives it.
#[test]
fn a_packs_drawing_stands_on_its_declared_rung() {
    use world_projection::{DrawPart, Ink};
    let parts = vec![
        DrawPart::rect(-0.4, 0.0, 0.8, 0.7, Ink::Wall),
        DrawPart::polygon(&[(-0.5, 0.7), (0.5, 0.7), (0.0, 1.0)], Ink::Roof),
    ];
    let made_up = Drawing::new("a-made-up-hall", 0.9, parts);
    assert_eq!(Subject::Drawing(&made_up).rung(), None);
    for tall in [1.5_f32, 4.0] {
        let declared = made_up.clone().standing(Rung::Tall(tall));
        let got = height_in_p(&Subject::Drawing(&declared));
        assert!(
            (got - tall).abs() <= tall * 0.1 + 0.05,
            "a drawing declared {tall} P stands {got:.2} P"
        );
    }
}
