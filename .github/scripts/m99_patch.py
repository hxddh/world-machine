from pathlib import Path


def replace_once(text, old, new, label):
    assert old in text, f"{label} anchor changed"
    return text.replace(old, new, 1)


path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()

text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
use std::sync::Arc;''',
    '''#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};''',
    "main atomic import",
)

text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";

#[cfg(target_os = "macos")]
struct SharedDocumentState {''',
    '''#[cfg(target_os = "macos")]
const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";

#[cfg(target_os = "macos")]
static LIBRARY_MUTATION_REVISION: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
pub(crate) fn mark_library_mutated() {
    LIBRARY_MUTATION_REVISION.fetch_add(1, Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
fn library_mutation_revision() -> u64 {
    LIBRARY_MUTATION_REVISION.load(Ordering::Relaxed)
}

#[cfg(target_os = "macos")]
struct SharedDocumentState {''',
    "main mutation signal",
)

text = replace_once(
    text,
    '''        let mut document = self.document.borrow_mut();
        let registry = Arc::clone(&document.registry);
        let library = Arc::clone(&document.library);
        document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string())''',
    '''        let mut document = self.document.borrow_mut();
        let registry = Arc::clone(&document.registry);
        let library = Arc::clone(&document.library);
        let is_library_world = document.session.document_id().is_some();
        let result = document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string());
        if result.is_ok() && is_library_world {
            mark_library_mutated();
        }
        result''',
    "projection mutation mark",
)

old_listener = '''    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(200))
                .await;

            let Some(this) = this.upgrade() else {
                return;
            };
            let paths = system_open::drain_paths();
            if paths.is_empty() {
                continue;
            }
            this.update(cx, |this, cx| {
                for path in paths {
                    match path {
                        Ok(path) => this.open_external_path(path, cx),
                        Err(error) => {
                            this.status = Some(HomeStatus::error(format!(
                                "Could not open World file: {error}"
                            )));
                            cx.notify();
                        }
                    }
                }
            });
        })
        .detach();
    }'''
new_listener = '''    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut observed_library_revision = library_mutation_revision();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let Some(this) = this.upgrade() else {
                    return;
                };
                let paths = system_open::drain_paths();
                let revision = library_mutation_revision();
                let library_changed = revision != observed_library_revision;
                if library_changed {
                    observed_library_revision = revision;
                }
                if paths.is_empty() && !library_changed {
                    continue;
                }

                this.update(cx, |this, cx| {
                    for path in paths {
                        match path {
                            Ok(path) => this.open_external_path(path, cx),
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Could not open World file: {error}"
                                )));
                            }
                        }
                    }
                    if library_changed {
                        if let Err(error) = this.refresh_documents() {
                            this.status = Some(error);
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }'''
text = replace_once(text, old_listener, new_listener, "home listener")

test_marker = '''    #[test]
    fn home_status_tone_is_explicit_not_inferred_from_message_text() {'''
test_insert = '''    #[test]
    fn library_mutation_revision_advances_after_mark() {
        let before = library_mutation_revision();
        mark_library_mutated();
        assert!(library_mutation_revision() > before);
    }

    #[test]
    fn home_status_tone_is_explicit_not_inferred_from_message_text() {'''
text = replace_once(text, test_marker, test_insert, "main test marker")

path.write_text(text)

path = Path("apps/world-machine-desktop/src/world_fork.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use super::{
    sanitize_document_base, unique_document_id, DocumentStatus, SharedDocument, WorldDocumentView,
};''',
    '''use super::{
    mark_library_mutated, sanitize_document_base, unique_document_id, DocumentStatus, SharedDocument,
    WorldDocumentView,
};''',
    "fork import",
)
text = replace_once(
    text,
    '''        document
            .session
            .fork_to_library(document_id.clone(), None, library.as_ref())
            .map_err(|error| error.to_string())?;
    }

    // From this point on the fork is durable.''',
    '''        document
            .session
            .fork_to_library(document_id.clone(), None, library.as_ref())
            .map_err(|error| error.to_string())?;
    }
    mark_library_mutated();

    // From this point on the fork is durable.''',
    "fork durable mark",
)
text = replace_once(
    text,
    '''    let observer_warning =
        super::observer::catch_up(&mut session, registry.as_ref(), library.as_ref())
            .err()
            .map(|error| format!("observer clock initialization skipped: {error}"));''',
    '''    let observer_warning =
        match super::observer::catch_up(&mut session, registry.as_ref(), library.as_ref()) {
            Ok(Some(_)) => {
                mark_library_mutated();
                None
            }
            Ok(None) => None,
            Err(error) => Some(format!("observer clock initialization skipped: {error}")),
        };''',
    "fork catchup mark",
)
path.write_text(text)

path = Path("apps/world-machine-desktop/src/strategy_compare.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use super::{
    sanitize_document_base, unique_document_id, DocumentStatus, SharedDocument, WorldDocumentView,
};''',
    '''use super::{
    mark_library_mutated, sanitize_document_base, unique_document_id, DocumentStatus, SharedDocument,
    WorldDocumentView,
};''',
    "strategy import",
)
text = replace_once(
    text,
    '''        let summary = self
            .library
            .create_from_document(id, &future)
            .map_err(|error| error.to_string())?;
        let saved_id = summary.id;''',
    '''        let summary = self
            .library
            .create_from_document(id, &future)
            .map_err(|error| error.to_string())?;
        mark_library_mutated();
        let saved_id = summary.id;''',
    "strategy save mark",
)
path.write_text(text)

path = Path("apps/world-machine-desktop/src/world_fork/lineage.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use super::super::{observer, DocumentStatus, SharedDocument, WorldDocumentView};''',
    '''use super::super::{
    mark_library_mutated, observer, DocumentStatus, SharedDocument, WorldDocumentView,
};''',
    "lineage import",
)
text = replace_once(
    text,
    '''        let notice =
            match observer::catch_up(&mut session, self.registry.as_ref(), self.library.as_ref()) {
                Ok(Some(outcome)) => Some(format!(
                    "Advanced {} background period(s) · World time {}",
                    outcome.periods, outcome.world_time
                )),
                Ok(None) => None,
                Err(error) => Some(format!("Catch-up skipped: {error}")),
            };''',
    '''        let notice =
            match observer::catch_up(&mut session, self.registry.as_ref(), self.library.as_ref()) {
                Ok(Some(outcome)) => {
                    mark_library_mutated();
                    Some(format!(
                        "Advanced {} background period(s) · World time {}",
                        outcome.periods, outcome.world_time
                    ))
                }
                Ok(None) => None,
                Err(error) => Some(format!("Catch-up skipped: {error}")),
            };''',
    "lineage catchup mark",
)
path.write_text(text)
