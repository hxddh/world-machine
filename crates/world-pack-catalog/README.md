# world-pack-catalog

Durable local installation catalog for external World Packs.

Installing a Pack is an explicit local approval step: World Machine validates the manifest without executing Pack code, canonicalizes its paths, and pins both the manifest and executable by SHA-256. Every enabled entry is revalidated before source assembly, and the same content pin is checked again immediately before process launch.

The catalog does not infer trust from a publisher and does not interpret Pack version strings. Multiple exact versions may remain enabled for durable World restoration, while one version per Pack id is explicitly marked active for new Worlds.
