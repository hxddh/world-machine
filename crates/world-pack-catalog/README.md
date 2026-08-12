# world-pack-catalog

Durable local installation catalog for external World Packs.

Installing a Pack is an explicit local approval step: World Machine validates the selected manifest without executing Pack code, then materializes the approved direct executable and a rewritten manifest into the catalog-owned `Installed/<opaque identity>/` store. The source manifest/executable are import inputs only; they can be moved or deleted after installation. New catalog entries pin the managed manifest and executable by SHA-256, and every enabled entry is revalidated before source assembly.

For content-pinned v1 installs, the approved program must be the direct process command; launcher-style runtime arguments are rejected because their referenced code would otherwise fall outside the pin. At launch, World Machine reads and hashes the managed executable from one file handle, materializes those exact verified bytes as a one-off launch image, executes that image, and removes it when the session ends. This is a local content-consistency boundary, not publisher signing or a defense against a malicious same-user process.

Uninstall removes a managed copy only after the catalog entry has been durably removed, and cleanup is constrained to the exact catalog-owned identity directory. Legacy catalog entries created before managed storage remain readable as unmanaged entries and are never treated as deletion targets.

The catalog does not infer trust from a publisher and does not interpret Pack version strings. Multiple exact versions may remain enabled for durable World restoration, while one version per Pack id is explicitly marked active for new Worlds. Installing or activating a version is the only way to change that active selection; lexical or semantic version ordering never does.
