use world_host::{HostError, WorldPackSource, WorldRegistration, WorldRegistry};

pub struct BuiltinWorlds;

impl WorldPackSource for BuiltinWorlds {
    fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError> {
        Ok(vec![
            tiny_society::tiny_society_registration(),
            future_archaeologist::future_archaeologist_registration(),
        ])
    }
}

pub fn registry() -> Result<WorldRegistry, HostError> {
    let mut registry = WorldRegistry::new();
    registry.install_source(&BuiltinWorlds)?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn builtin_source_exposes_both_benchmark_worlds() {
        let registrations = BuiltinWorlds.registrations().unwrap();
        let ids = registrations
            .iter()
            .map(|registration| registration.descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
        assert!(ids.contains(future_archaeologist::FUTURE_ARCHAEOLOGIST_PACK_ID));
    }

    #[test]
    fn catalog_lists_both_benchmark_worlds() {
        let registry = registry().unwrap();
        let ids = registry
            .descriptors()
            .into_iter()
            .map(|descriptor| descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
        assert!(ids.contains(future_archaeologist::FUTURE_ARCHAEOLOGIST_PACK_ID));
    }

    #[test]
    fn every_builtin_can_create_archive_and_reopen() {
        let registry = registry().unwrap();
        let ids = registry
            .descriptors()
            .into_iter()
            .map(|descriptor| descriptor.pack.id.clone())
            .collect::<Vec<_>>();

        for id in ids {
            let session = registry.create(&id).unwrap();
            let pack = session.pack();
            let archive = session.archive().unwrap().unwrap();
            let reopened = registry.open_archive(&archive).unwrap();

            assert_eq!(reopened.pack(), pack);
            assert_eq!(reopened.snapshot().title, session.snapshot().title);
        }
    }
}
