mod actions;
mod host;
mod model;
mod persistence;
mod projection;
mod seed;

use std::error::Error;
use world_core::{ActionRegistry, ActionRequest, EventId, World};
use world_projection::ProjectionSnapshot;

pub use host::future_archaeologist_registration;
pub use model::{
    ASTERION, CALENDAR_FRAGMENT, DELETED_MESSAGE, ELIAS, MIRA, PLATFORM_12, PLATFORM_PHOTO,
    PROJECT_COPY_LOG, TAXI_RECEIPT, TERMINAL, WIFI_LOG,
};
pub use persistence::{
    future_archaeologist_pack_ref, FUTURE_ARCHAEOLOGIST_PACK_ID, FUTURE_ARCHAEOLOGIST_PACK_VERSION,
};

pub const RECOVER_MESSAGE_COMMAND: &str = "future-archaeologist.recover-deleted-message";

pub struct FutureArchaeologist {
    world: World,
    actions: ActionRegistry,
}

impl FutureArchaeologist {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let actions = build_action_registry()?;
        let world = World::from_history(seed::baseline()?, &seed::truth_events())?;
        Ok(Self { world, actions })
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        projection::snapshot(&self.world)
    }

    pub fn invoke_projection_command(
        &mut self,
        command_id: &str,
    ) -> Result<EventId, Box<dyn Error>> {
        match command_id {
            RECOVER_MESSAGE_COMMAND => {
                let event = self.world.execute(
                    &self.actions,
                    &ActionRequest::new("recover_deleted_message")
                        .actor(TERMINAL)
                        .caused_by(model::MESSAGE_DELETED),
                )?;
                Ok(event.id)
            }
            _ => Err(
                std::io::Error::other(format!("unknown projection command: {command_id}")).into(),
            ),
        }
    }
}

fn build_action_registry() -> Result<ActionRegistry, Box<dyn Error>> {
    let mut actions = ActionRegistry::new();
    actions::register(&mut actions)?;
    Ok(actions)
}

#[cfg(test)]
mod tests;
