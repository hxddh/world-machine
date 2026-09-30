use world_projection::{CanvasItemKind, ProjectionSnapshot};

/// A real harbour on its 358th day, as its Pack sends it (only what
/// the scene draws), for the painter's tests.
pub(crate) fn lived_harbour() -> ProjectionSnapshot {
    let json = include_str!("../../tests/fixtures/harbour-day-358.json");
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(json).expect("a wire snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// The painter draws everyone the World's day puts outdoors: at noon
/// most of the harbour is out and about, at eleven at night hardly
/// anyone is.
#[test]
fn a_lived_harbour_is_out_by_day_and_in_by_night() {
    let snapshot = lived_harbour();
    let residents = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .count();
    assert!(residents >= 10, "{residents}");
    let outdoors = |hour: u8| {
        let stage = super::stage_at(&snapshot, 1100.0, 848.0, super::Clock::at(hour));
        let living = super::living(
            &stage,
            &snapshot,
            0.0,
            crate::scene::daylight_at(hour as u32),
            &Default::default(),
            None,
        );
        let frame = super::frame(
            &snapshot,
            &stage,
            &living,
            super::Camera::whole(&stage),
            0.0,
            crate::scene::daylight_at(hour as u32),
            &Default::default(),
            1.0,
        );
        // A baby carried counts as out, in someone's arms.
        let drawn = frame.people.len()
            + frame
                .people
                .iter()
                .filter(|person| person.carrying.is_some())
                .count();
        drawn as f32 / residents as f32
    };
    assert!(outdoors(12) >= 0.6, "{} out at noon", outdoors(12));
    assert!(outdoors(10) >= 0.6);
    assert!(outdoors(23) <= 0.3, "{} out at night", outdoors(23));
}
