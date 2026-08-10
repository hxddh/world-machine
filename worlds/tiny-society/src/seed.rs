use crate::model::*;
use society_basic::{CASH, JOB};
use std::error::Error;
use world_core::{Entity, Relation, WorldState};

pub(crate) fn seed_world() -> Result<WorldState, Box<dyn Error>> {
    let mut state = WorldState::default();

    for (id, name, cash, job, location) in [
        (JONAS, "Jonas", 60_i64, "fisher", HARBOR),
        (MARA, "Mara", 120, "baker", BAKERY),
        (LEO, "Leo", 180, "pub_owner", PUB),
        (EMMA, "Emma", 100, "teacher", SCHOOL),
        (MIA, "Mia", 35, "student", SCHOOL),
        (NOAH, "Noah", 220, "mayor", HARBOR),
        (EVAN, "Evan", 90, "carpenter", HARBOR),
        (SOFIA, "Sofia", 70, "shop_assistant", PUB),
    ] {
        state.seed_entity(
            Entity::new(id, "resident")
                .with_component("name", name)
                .with_component(CASH, cash)
                .with_component(JOB, job)
                .with_component("location", location)
                .with_component(MISSED_SHIFTS, 0_i64),
        )?;
    }

    for (id, name, cash) in [
        (HARBOR, "Harbor", 800_i64),
        (BAKERY, "Harbor Bakery", 500),
        (SCHOOL, "Island School", 1_000),
        (PUB, "Anchor Pub", 600),
    ] {
        state.seed_entity(
            Entity::new(id, "location")
                .with_component("name", name)
                .with_component(CASH, cash),
        )?;
    }

    state.seed_entity(
        Entity::new(JONAS_BOAT, "asset")
            .with_component("name", "Sea Finch")
            .with_component(CONDITION, "sound"),
    )?;
    state.seed_entity(
        Entity::new(WEDDING_ORDER, "order")
            .with_component("name", "Wedding bread order")
            .with_component(ORDER_STATUS, "pending")
            .with_component("value", 120_i64),
    )?;

    state.seed_relation(
        Relation::new(MARA_EMMA_FRIEND, "friend", MARA, EMMA).with_property("trust", 82_i64),
    )?;
    state.seed_relation(
        Relation::new(JONAS_LEO_TRUST, "trusts", JONAS, LEO).with_property("trust", 76_i64),
    )?;
    state.seed_relation(Relation::new(JONAS_BOAT_OWNER, "owns", JONAS, JONAS_BOAT))?;
    state.seed_relation(Relation::new(MARA_BAKERY_JOB, "works_at", MARA, BAKERY))?;
    state.seed_relation(Relation::new(LEO_PUB_JOB, "works_at", LEO, PUB))?;
    state.seed_relation(Relation::new(EMMA_SCHOOL_JOB, "works_at", EMMA, SCHOOL))?;
    state.seed_relation(Relation::new(JONAS_HARBOR_JOB, "works_at", JONAS, HARBOR))?;

    Ok(state)
}
