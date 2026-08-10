use crate::{build_action_registry, seed, FutureArchaeologist};
use std::error::Error;
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};

pub const FUTURE_ARCHAEOLOGIST_PACK_ID: &str = "world-machine.future-archaeologist";
pub const FUTURE_ARCHAEOLOGIST_PACK_VERSION: &str = "0.1.0";

pub fn future_archaeologist_pack_ref() -> WorldPackRef {
    WorldPackRef::new(
        FUTURE_ARCHAEOLOGIST_PACK_ID,
        FUTURE_ARCHAEOLOGIST_PACK_VERSION,
    )
}

impl FutureArchaeologist {
    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(future_archaeologist_pack_ref(), &self.world)
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        let world = archive.restore(&future_archaeologist_pack_ref(), seed::baseline()?)?;
        Ok(Self {
            world,
            actions: build_action_registry()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DELETED_MESSAGE, RECOVER_MESSAGE_COMMAND};
    use world_projection::SelectionId;

    #[test]
    fn archive_round_trip_preserves_recovered_evidence() {
        let mut world = FutureArchaeologist::new().unwrap();
        world
            .invoke_projection_command(RECOVER_MESSAGE_COMMAND)
            .unwrap();
        let archive = world.archive().unwrap();

        let restored = FutureArchaeologist::resume_archive(&archive).unwrap();
        let snapshot = restored.projection_snapshot();

        assert!(snapshot.commands.is_empty());
        assert!(snapshot
            .collection
            .items
            .iter()
            .any(|item| item.id == SelectionId::Entity(DELETED_MESSAGE)));
    }
}
