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
        let mut resident = Entity::new(id, "resident")
            .with_component("name", name)
            .with_component(CASH, cash)
            .with_component(JOB, job)
            .with_component("location", location)
            .with_component(MISSED_SHIFTS, 0_i64);
        if id == JONAS {
            resident = resident.with_component(SUPPORT_STATUS, "none");
        }
        state.seed_entity(resident)?;
    }

    for (id, name, cash) in [
        (HARBOR, "Harbor", 800_i64),
        (BAKERY, "Harbor Bakery", 500),
        (SCHOOL, "Island School", 1_000),
        (PUB, "Anchor Pub", 600),
    ] {
        let mut entity = Entity::new(id, "location")
            .with_component("name", name)
            .with_component(CASH, cash);
        // A business says whether it is open. The harbour is not a business.
        if matches!(id, BAKERY | PUB | SCHOOL) {
            entity = entity.with_component(OPERATING_STATUS, "open");
        }
        state.seed_entity(entity)?;
    }

    state.seed_entity(
        Entity::new(MAINLAND_MARKET, "external_market")
            .with_component("name", "Mainland Fish Market")
            .with_component(CASH, 10_000_i64),
    )?;
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
    // Sofia has been the Anchor Pub's shop assistant since the first morning
    // and the World never recorded it, so she never worked a shift, was never
    // paid, and could not be laid off when the pub ran dry.
    state.seed_relation(Relation::new(SOFIA_PUB_JOB, "works_at", SOFIA, PUB))?;

    Ok(state)
}
