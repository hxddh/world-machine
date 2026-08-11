use world_core::{EntityId, RelationId};

pub const JONAS: EntityId = EntityId::new(1);
pub const MARA: EntityId = EntityId::new(2);
pub const LEO: EntityId = EntityId::new(3);
pub const EMMA: EntityId = EntityId::new(4);
pub const MIA: EntityId = EntityId::new(5);
pub const NOAH: EntityId = EntityId::new(6);
pub const EVAN: EntityId = EntityId::new(7);
pub const SOFIA: EntityId = EntityId::new(8);

pub const HARBOR: EntityId = EntityId::new(101);
pub const BAKERY: EntityId = EntityId::new(102);
pub const SCHOOL: EntityId = EntityId::new(103);
pub const PUB: EntityId = EntityId::new(104);

pub const JONAS_BOAT: EntityId = EntityId::new(201);
pub const WEDDING_ORDER: EntityId = EntityId::new(202);
pub(crate) const MAINLAND_MARKET: EntityId = EntityId::new(301);

pub(crate) const MARA_EMMA_FRIEND: RelationId = RelationId::new(601);
pub(crate) const JONAS_LEO_TRUST: RelationId = RelationId::new(602);
pub(crate) const JONAS_BOAT_OWNER: RelationId = RelationId::new(603);
pub(crate) const MARA_BAKERY_JOB: RelationId = RelationId::new(701);
pub(crate) const LEO_PUB_JOB: RelationId = RelationId::new(702);
pub(crate) const EMMA_SCHOOL_JOB: RelationId = RelationId::new(703);
pub(crate) const JONAS_HARBOR_JOB: RelationId = RelationId::new(704);
pub(crate) const TEMP_BAKERY_JOB: RelationId = RelationId::new(705);

pub(crate) const CONDITION: &str = "condition";
pub(crate) const INCOME_STATUS: &str = "income_status";
pub(crate) const LOAN_STATUS: &str = "loan_status";
pub(crate) const MISSED_SHIFTS: &str = "missed_shifts";
pub(crate) const OPERATING_STATUS: &str = "operating_status";
pub(crate) const ORDER_STATUS: &str = "status";
pub(crate) const SUPPORT_STATUS: &str = "support_status";
pub(crate) const WEATHER: &str = "weather";
