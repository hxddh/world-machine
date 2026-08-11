use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_persistence::{WorldArchive, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveIntegritySummary {
    pub event_count: usize,
    pub pending_count: usize,
    pub latest_event_time: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveIntegrityError {
    UnsupportedFormat(String),
    UnsupportedVersion(u32),
    InvalidPack,
    DuplicateEventId(u64),
    EmptyEventKind(u64),
    EventBeyondWorldTime {
        event_id: u64,
        event_time: u64,
        world_time: u64,
    },
    EventTimeRegression {
        event_id: u64,
        previous_time: u64,
        event_time: u64,
    },
    MissingEventCause {
        event_id: u64,
        cause_id: u64,
    },
    NonHistoricalEventCause {
        event_id: u64,
        cause_id: u64,
    },
    EmptyPendingAction {
        index: usize,
    },
    PendingInPast {
        index: usize,
        scheduled_time: u64,
        world_time: u64,
    },
    MissingPendingCause {
        index: usize,
        cause_id: u64,
    },
}

impl fmt::Display for ArchiveIntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat(format) => {
                write!(f, "unsupported world archive format: {format}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported world archive version: {version}")
            }
            Self::InvalidPack => write!(f, "world archive Pack id and version must be non-empty"),
            Self::DuplicateEventId(event_id) => write!(f, "duplicate event id: #{event_id}"),
            Self::EmptyEventKind(event_id) => write!(f, "event #{event_id} has an empty kind"),
            Self::EventBeyondWorldTime {
                event_id,
                event_time,
                world_time,
            } => write!(
                f,
                "event #{event_id} occurs at t={event_time} after archive world time {world_time}"
            ),
            Self::EventTimeRegression {
                event_id,
                previous_time,
                event_time,
            } => write!(
                f,
                "event time regresses at #{event_id}: {previous_time} -> {event_time}"
            ),
            Self::MissingEventCause { event_id, cause_id } => {
                write!(f, "event #{event_id} references missing cause #{cause_id}")
            }
            Self::NonHistoricalEventCause { event_id, cause_id } => write!(
                f,
                "event #{event_id} cause #{cause_id} is not an earlier event"
            ),
            Self::EmptyPendingAction { index } => {
                write!(f, "pending action #{index} has an empty action name")
            }
            Self::PendingInPast {
                index,
                scheduled_time,
                world_time,
            } => write!(
                f,
                "pending action #{index} is scheduled in the past: {scheduled_time} < {world_time}"
            ),
            Self::MissingPendingCause { index, cause_id } => write!(
                f,
                "pending action #{index} references missing cause #{cause_id}"
            ),
        }
    }
}

impl Error for ArchiveIntegrityError {}

pub fn check_archive(
    archive: &WorldArchive,
) -> Result<ArchiveIntegritySummary, ArchiveIntegrityError> {
    check_header(archive)?;

    let mut positions = BTreeMap::new();
    for (position, event) in archive.events.iter().enumerate() {
        if positions.insert(event.id, position).is_some() {
            return Err(ArchiveIntegrityError::DuplicateEventId(event.id));
        }
    }

    let mut previous_time = None;
    for (position, event) in archive.events.iter().enumerate() {
        if event.kind.trim().is_empty() {
            return Err(ArchiveIntegrityError::EmptyEventKind(event.id));
        }
        if event.world_time > archive.world_time {
            return Err(ArchiveIntegrityError::EventBeyondWorldTime {
                event_id: event.id,
                event_time: event.world_time,
                world_time: archive.world_time,
            });
        }
        if let Some(previous_time) = previous_time {
            if event.world_time < previous_time {
                return Err(ArchiveIntegrityError::EventTimeRegression {
                    event_id: event.id,
                    previous_time,
                    event_time: event.world_time,
                });
            }
        }
        previous_time = Some(event.world_time);

        for cause_id in &event.caused_by {
            match positions.get(cause_id) {
                None => {
                    return Err(ArchiveIntegrityError::MissingEventCause {
                        event_id: event.id,
                        cause_id: *cause_id,
                    });
                }
                Some(cause_position) if *cause_position >= position => {
                    return Err(ArchiveIntegrityError::NonHistoricalEventCause {
                        event_id: event.id,
                        cause_id: *cause_id,
                    });
                }
                Some(_) => {}
            }
        }
    }

    for (offset, pending) in archive.pending.iter().enumerate() {
        let index = offset + 1;
        if pending.request.action.trim().is_empty() {
            return Err(ArchiveIntegrityError::EmptyPendingAction { index });
        }
        if pending.world_time < archive.world_time {
            return Err(ArchiveIntegrityError::PendingInPast {
                index,
                scheduled_time: pending.world_time,
                world_time: archive.world_time,
            });
        }
        for cause_id in &pending.request.caused_by {
            if !positions.contains_key(cause_id) {
                return Err(ArchiveIntegrityError::MissingPendingCause {
                    index,
                    cause_id: *cause_id,
                });
            }
        }
    }

    Ok(ArchiveIntegritySummary {
        event_count: archive.events.len(),
        pending_count: archive.pending.len(),
        latest_event_time: archive.events.last().map(|event| event.world_time),
    })
}

fn check_header(archive: &WorldArchive) -> Result<(), ArchiveIntegrityError> {
    if archive.format != WORLD_ARCHIVE_FORMAT {
        return Err(ArchiveIntegrityError::UnsupportedFormat(
            archive.format.clone(),
        ));
    }
    if archive.format_version != WORLD_ARCHIVE_VERSION {
        return Err(ArchiveIntegrityError::UnsupportedVersion(
            archive.format_version,
        ));
    }
    if archive.pack.id.trim().is_empty() || archive.pack.version.trim().is_empty() {
        return Err(ArchiveIntegrityError::InvalidPack);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use world_persistence::{
        ArchivedActionRequest, ArchivedEvent, ArchivedScheduledAction, WorldPackRef,
    };

    #[test]
    fn accepts_well_formed_archive() {
        let mut archive = archive(5);
        archive.events = vec![event(1, "storm", 1, vec![]), event(2, "damage", 2, vec![1])];
        archive.pending = vec![pending(7, "repair", vec![2])];

        let summary = check_archive(&archive).unwrap();

        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.pending_count, 1);
        assert_eq!(summary.latest_event_time, Some(2));
    }

    #[test]
    fn rejects_duplicate_event_ids() {
        let mut archive = archive(2);
        archive.events = vec![event(1, "one", 1, vec![]), event(1, "again", 2, vec![])];

        assert_eq!(
            check_archive(&archive).unwrap_err(),
            ArchiveIntegrityError::DuplicateEventId(1)
        );
    }

    #[test]
    fn rejects_event_time_regression_or_event_beyond_world_time() {
        let mut regression = archive(3);
        regression.events = vec![event(1, "one", 2, vec![]), event(2, "two", 1, vec![1])];
        assert!(matches!(
            check_archive(&regression),
            Err(ArchiveIntegrityError::EventTimeRegression { .. })
        ));

        let mut future = archive(2);
        future.events = vec![event(1, "one", 3, vec![])];
        assert!(matches!(
            check_archive(&future),
            Err(ArchiveIntegrityError::EventBeyondWorldTime { .. })
        ));
    }

    #[test]
    fn rejects_missing_or_nonhistorical_event_causes() {
        let mut missing = archive(2);
        missing.events = vec![event(1, "one", 1, vec![]), event(2, "two", 2, vec![99])];
        assert_eq!(
            check_archive(&missing).unwrap_err(),
            ArchiveIntegrityError::MissingEventCause {
                event_id: 2,
                cause_id: 99,
            }
        );

        let mut future = archive(2);
        future.events = vec![event(1, "one", 1, vec![2]), event(2, "two", 2, vec![])];
        assert_eq!(
            check_archive(&future).unwrap_err(),
            ArchiveIntegrityError::NonHistoricalEventCause {
                event_id: 1,
                cause_id: 2,
            }
        );
    }

    #[test]
    fn rejects_invalid_pending_actions() {
        let mut past = archive(5);
        past.events = vec![event(1, "one", 1, vec![])];
        past.pending = vec![pending(4, "repair", vec![1])];
        assert!(matches!(
            check_archive(&past),
            Err(ArchiveIntegrityError::PendingInPast { .. })
        ));

        let mut missing = archive(5);
        missing.events = vec![event(1, "one", 1, vec![])];
        missing.pending = vec![pending(6, "repair", vec![99])];
        assert_eq!(
            check_archive(&missing).unwrap_err(),
            ArchiveIntegrityError::MissingPendingCause {
                index: 1,
                cause_id: 99,
            }
        );

        let mut empty = archive(5);
        empty.pending = vec![pending(6, "   ", vec![])];
        assert_eq!(
            check_archive(&empty).unwrap_err(),
            ArchiveIntegrityError::EmptyPendingAction { index: 1 }
        );
    }

    #[test]
    fn rejects_invalid_header_or_empty_event_kind() {
        let mut wrong_format = archive(0);
        wrong_format.format = "other-format".into();
        assert!(matches!(
            check_archive(&wrong_format),
            Err(ArchiveIntegrityError::UnsupportedFormat(_))
        ));

        let mut empty_kind = archive(1);
        empty_kind.events = vec![event(1, "  ", 1, vec![])];
        assert_eq!(
            check_archive(&empty_kind).unwrap_err(),
            ArchiveIntegrityError::EmptyEventKind(1)
        );
    }

    fn archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.test", "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn event(id: u64, kind: &str, world_time: u64, caused_by: Vec<u64>) -> ArchivedEvent {
        ArchivedEvent {
            id,
            kind: kind.into(),
            world_time,
            actor: None,
            targets: Vec::new(),
            caused_by,
            payload: BTreeMap::new(),
            changes: Vec::new(),
        }
    }

    fn pending(world_time: u64, action: &str, caused_by: Vec<u64>) -> ArchivedScheduledAction {
        ArchivedScheduledAction {
            world_time,
            request: ArchivedActionRequest {
                actor: None,
                action: action.into(),
                args: BTreeMap::new(),
                caused_by,
            },
        }
    }
}
