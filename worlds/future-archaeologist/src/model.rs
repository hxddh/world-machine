use world_core::{EntityId, EventId, RelationId};

pub const TERMINAL: EntityId = EntityId::new(1);
pub const MIRA: EntityId = EntityId::new(2);
pub const ELIAS: EntityId = EntityId::new(3);
pub const PLATFORM_12: EntityId = EntityId::new(4);
pub const ASTERION: EntityId = EntityId::new(5);

pub const CALENDAR_FRAGMENT: EntityId = EntityId::new(10);
pub const TAXI_RECEIPT: EntityId = EntityId::new(11);
pub const PLATFORM_PHOTO: EntityId = EntityId::new(12);
pub const WIFI_LOG: EntityId = EntityId::new(13);
pub const PROJECT_COPY_LOG: EntityId = EntityId::new(14);
pub const DELETED_MESSAGE: EntityId = EntityId::new(15);

pub const ASTERION_EMPLOYS_MIRA: RelationId = RelationId::new(100);
pub const ASTERION_EMPLOYS_ELIAS: RelationId = RelationId::new(101);
pub const TERMINAL_OWNED_BY_ELIAS: RelationId = RelationId::new(102);

pub const MEETING_SCHEDULED: EventId = EventId::new(1);
pub const TAXI_RIDE_RECORDED: EventId = EventId::new(2);
pub const PHOTO_CAPTURED: EventId = EventId::new(3);
pub const PLATFORM_ACCESSED: EventId = EventId::new(4);
pub const PROTOTYPE_COPIED: EventId = EventId::new(5);
pub const WARNING_MESSAGE_SENT: EventId = EventId::new(6);
pub const MESSAGE_DELETED: EventId = EventId::new(7);

pub const VISIBLE: &str = "visible";
pub const EVENT_REF: &str = "event_ref";
pub const ARTIFACT_KIND: &str = "artifact_kind";
pub const SOURCE: &str = "source";
pub const SUMMARY: &str = "summary";
pub const TIMESTAMP: &str = "timestamp";
