# world-pack-catalog

Durable local installation catalog for external World Packs.

Installing a Pack is an explicit local approval step: World Machine validates the manifest without executing Pack code, canonicalizes its paths, and pins both the manifest and executable by SHA-256. Every enabled entry is revalidated before source assembly.

For content-pinned v1 installs, the approved program must be the direct process command; launcher-style runtime arguments are rejected because their referenced code would otherwise fall outside the pin. At launch, World Machine reads and hashes the executable from one file handle, materializes those exact verified bytes as a one-off launch image, executes that image, and removes it when the session ends. This is a local content-consistency boundary, not publisher signing or a defense against a malicious same-user process.

The catalog does not infer trust from a publisher and does not interpret Pack version strings. Multiple exact versions may remain enabled for durable World restoration, while one version per Pack id is explicitly marked active for new Worlds. Installing or activating a version is the only way to change that active selection; lexical or semantic version ordering never does.
