use crate::{ActionRequest, ScheduleId};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledAction {
    pub id: ScheduleId,
    pub world_time: u64,
    pub request: ActionRequest,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ScheduleKey {
    world_time: u64,
    sequence: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scheduler {
    queue: BTreeMap<ScheduleKey, ScheduledAction>,
    next_id: u64,
    next_sequence: u64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
            next_id: 1,
            next_sequence: 1,
        }
    }

    pub fn schedule_at(&mut self, world_time: u64, request: ActionRequest) -> ScheduleId {
        let id = ScheduleId::new(self.next_id);
        let key = ScheduleKey {
            world_time,
            sequence: self.next_sequence,
        };
        self.next_id += 1;
        self.next_sequence += 1;
        self.queue.insert(
            key,
            ScheduledAction {
                id,
                world_time,
                request,
            },
        );
        id
    }

    pub fn pending(&self) -> impl Iterator<Item = &ScheduledAction> {
        self.queue.values()
    }

    pub fn get(&self, id: ScheduleId) -> Option<&ScheduledAction> {
        self.queue.values().find(|scheduled| scheduled.id == id)
    }

    pub(crate) fn next_due(&self, target_time: u64) -> Option<ScheduledAction> {
        self.queue
            .first_key_value()
            .filter(|(key, _)| key.world_time <= target_time)
            .map(|(_, scheduled)| scheduled.clone())
    }

    pub(crate) fn complete(&mut self, id: ScheduleId) -> bool {
        let key = self
            .queue
            .iter()
            .find_map(|(key, scheduled)| (scheduled.id == id).then_some(*key));
        match key {
            Some(key) => {
                self.queue.remove(&key);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_time_items_preserve_insertion_order() {
        let mut scheduler = Scheduler::new();
        let first = scheduler.schedule_at(10, ActionRequest::new("first"));
        let second = scheduler.schedule_at(10, ActionRequest::new("second"));

        let pending: Vec<_> = scheduler.pending().map(|item| item.id).collect();
        assert_eq!(pending, vec![first, second]);
    }
}
