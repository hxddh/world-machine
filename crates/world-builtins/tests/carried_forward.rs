//! Worlds saved by earlier releases open in this one and carry on, their
//! history kept exactly as it was. Each release adds the fixtures its own
//! Worlds leave behind, and they must keep opening from then on.

use world_host::WorldRegistry;
use world_persistence::WorldArchive;
use world_projection::ProjectionIntent::InvokeCommand;

fn registry() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

fn fixtures() -> Vec<(String, WorldArchive)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut all = Vec::new();
    for release in std::fs::read_dir(&root).unwrap() {
        let release = release.unwrap().path();
        for file in std::fs::read_dir(&release).unwrap() {
            let file = file.unwrap().path();
            let json = std::fs::read_to_string(&file).unwrap();
            let name = format!(
                "{}/{}",
                release.file_name().unwrap().to_string_lossy(),
                file.file_name().unwrap().to_string_lossy()
            );
            all.push((name, WorldArchive::from_json(&json).unwrap()));
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

#[test]
fn every_world_an_earlier_release_saved_opens_and_carries_on() {
    let registry = registry();
    let fixtures = fixtures();
    assert!(fixtures.len() >= 4, "{} fixtures", fixtures.len());
    for (name, saved) in fixtures {
        let current = registry
            .descriptor(&saved.pack.id)
            .unwrap_or_else(|| panic!("{name}: no Pack"))
            .pack
            .clone();
        assert!(
            registry.descriptor_for(&saved.pack).is_some(),
            "{name}: Home would say it needs a Pack"
        );
        let mut session = registry
            .open_archive(&saved)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let opened = session.snapshot();
        assert!(
            !opened.canvas.items.is_empty(),
            "{name}: nothing on the scene"
        );
        let pass = if saved.pack.id == tiny_society::TINY_SOCIETY_PACK_ID {
            "tiny-society.let-day-pass"
        } else {
            pocket_universe::NUDGE_COMMAND
        };
        for _ in 0..30 {
            let snapshot = session.snapshot();
            if let Some(answer) = snapshot
                .commands
                .iter()
                .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
            {
                let _ = session.handle(InvokeCommand(answer.id.clone()));
            }
            session
                .handle(InvokeCommand(pass.into()))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        }
        let carried = session.archive().unwrap().unwrap();
        assert_eq!(
            carried.pack, current,
            "{name}: saved again as this release's"
        );
        assert!(carried.events.len() > saved.events.len());
        assert_eq!(
            carried.events[..saved.events.len()],
            saved.events[..],
            "{name}: its history is kept as it was"
        );
        // And it opens again as it stands, the same World.
        let reopened = registry.open_archive(&carried).unwrap();
        assert_eq!(reopened.snapshot().title, session.snapshot().title);
        assert_eq!(reopened.archive().unwrap().unwrap(), carried);
    }
}
