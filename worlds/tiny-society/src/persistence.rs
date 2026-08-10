use crate::{behaviors, build_action_registry, projection, seed, TinySociety, TinySocietyBranch};
use std::error::Error;
use world_core::{BehaviorRegistry, World};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::ProjectionSnapshot;

pub const TINY_SOCIETY_PACK_ID: &str = "world-machine.tiny-society";
pub const TINY_SOCIETY_PACK_VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisitCursor {
    pub event_count: usize,
}

impl VisitCursor {
    pub const fn new(event_count: usize) -> Self {
        Self { event_count }
    }
}

pub fn tiny_society_pack_ref() -> WorldPackRef {
    WorldPackRef::new(TINY_SOCIETY_PACK_ID, TINY_SOCIETY_PACK_VERSION)
}

impl TinySociety {
    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(tiny_society_pack_ref(), &self.world)
    }

    pub fn archive_json(&self) -> Result<String, PersistenceError> {
        self.archive()?.to_json_pretty()
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        let baseline = seed::seed_world()?;
        let world = archive.restore(&tiny_society_pack_ref(), baseline)?;
        restored_simulation(world)
    }

    pub fn resume_json(json: &str) -> Result<Self, Box<dyn Error>> {
        let archive = WorldArchive::from_json(json)?;
        Self::resume_archive(&archive)
    }

    pub fn visit_cursor(&self) -> VisitCursor {
        VisitCursor::new(self.world.events().len())
    }

    pub fn projection_snapshot_since(&self, cursor: VisitCursor) -> ProjectionSnapshot {
        projection::snapshot_since(&self.world, Some(cursor.event_count))
    }
}

impl TinySocietyBranch {
    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(tiny_society_pack_ref(), &self.world)
    }

    pub fn archive_json(&self) -> Result<String, PersistenceError> {
        self.archive()?.to_json_pretty()
    }

    pub fn visit_cursor(&self) -> VisitCursor {
        VisitCursor::new(self.world.events().len())
    }

    pub fn projection_snapshot_since(&self, cursor: VisitCursor) -> ProjectionSnapshot {
        projection::snapshot_since(&self.world, Some(cursor.event_count))
    }
}

fn restored_simulation(world: World) -> Result<TinySociety, Box<dyn Error>> {
    let actions = build_action_registry()?;
    let mut behaviors = BehaviorRegistry::new();
    behaviors::register(&mut behaviors)?;

    Ok(TinySociety {
        world,
        actions,
        behaviors,
    })
}
