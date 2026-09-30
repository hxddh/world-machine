//! Views a reader works out from a World, kept with it between readings.

use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

type View = Arc<dyn Any + Send + Sync>;

/// A World's derived views, one of each type. They are the reader's, not
/// the World's: they take no part in comparing Worlds, are never saved or
/// replayed, and are forgotten when the World goes back to a checkpoint. A
/// copy of a World starts with the views it had and keeps its own after.
#[derive(Default)]
pub(crate) struct DerivedViews(Mutex<BTreeMap<TypeId, View>>);

impl DerivedViews {
    fn views(&self) -> std::sync::MutexGuard<'_, BTreeMap<TypeId, View>> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub(crate) fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let view = self.views().get(&TypeId::of::<T>()).cloned()?;
        view.downcast::<T>().ok()
    }

    pub(crate) fn keep<T: Any + Send + Sync>(&self, view: Arc<T>) {
        self.views().insert(TypeId::of::<T>(), view);
    }

    pub(crate) fn forget(&mut self) {
        self.0
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner())
            .clear();
    }
}

impl Clone for DerivedViews {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.views().clone()))
    }
}

impl PartialEq for DerivedViews {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl fmt::Debug for DerivedViews {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DerivedViews")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft,
        StateChange, Value, World, WorldState,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const COUNTER: EntityId = EntityId::new(1);

    struct Count;

    impl Action for Count {
        fn name(&self) -> &'static str {
            "count"
        }

        fn evaluate(
            &self,
            state: &WorldState,
            _request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let n = match state.entity(COUNTER).and_then(|e| e.component("n")) {
                Some(Value::Integer(n)) => *n,
                _ => 0,
            };
            let mut draft = EventDraft::new("counted");
            draft.changes = vec![StateChange::SetComponent {
                entity: COUNTER,
                key: "n".into(),
                value: Value::Integer(n + 1),
            }];
            Ok(draft)
        }
    }

    fn world() -> (World, ActionRegistry) {
        let mut state = WorldState::default();
        state.seed_entity(Entity::new(COUNTER, "counter")).unwrap();
        let mut actions = ActionRegistry::new();
        actions.register(Count).unwrap();
        (World::new(state), actions)
    }

    struct Seen(usize);

    #[test]
    fn a_view_is_worked_out_once_for_as_long_as_the_world_stands_so() {
        let (mut world, actions) = world();
        let made = AtomicUsize::new(0);
        let look = |world: &World| {
            world
                .as_it_stands(|| Seen(made.fetch_add(1, Ordering::SeqCst) + 1))
                .0
        };
        assert_eq!(look(&world), 1);
        assert_eq!(look(&world), 1, "kept while nothing changes");
        world
            .execute(&actions, &ActionRequest::new("count"))
            .unwrap();
        assert_eq!(look(&world), 2, "an event moves the World on");
        world.advance_to(&actions, 5).unwrap();
        assert_eq!(look(&world), 3, "so does its clock");
        world.schedule_at(9, ActionRequest::new("count")).unwrap();
        assert_eq!(look(&world), 4, "and its schedule");
        assert_eq!(look(&world), 4);
    }

    #[test]
    fn views_are_forgotten_on_going_back_and_each_copy_keeps_its_own() {
        let (mut world, actions) = world();
        let keep = |world: &World, n: usize| world.derived::<Seen>(|_| Arc::new(Seen(n))).0;
        let kept = |world: &World| {
            world
                .derived::<Seen>(|kept| kept.unwrap_or(Arc::new(Seen(0))))
                .0
        };
        keep(&world, 7);
        let copy = world.clone();
        assert_eq!(kept(&copy), 7, "a copy starts with the views it had");
        keep(&copy, 8);
        assert_eq!(kept(&world), 7, "and keeps its own after");
        let checkpoint = world.checkpoint();
        world
            .execute(&actions, &ActionRequest::new("count"))
            .unwrap();
        world.rollback(checkpoint);
        assert_eq!(kept(&world), 0, "going back forgets every view");
        keep(&world, 9);
        assert_eq!(kept(&world.sketch(10)), 0, "a sketch starts with none");
        let other = world.clone();
        keep(&other, 1);
        assert_eq!(world, other, "views take no part in comparing Worlds");
    }
}
