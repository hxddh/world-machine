from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
start = s.index('impl fmt::Display for CatalogError {')
end = s.index('#[cfg(test)]', start)
replacement = '''impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, path, message } => write!(f, "could not {operation} {}: {message}", path.display()),
            Self::Json(error) => write!(f, "could not decode Pack catalog: {error}"),
            Self::UnsupportedFormat(format) => write!(f, "unsupported Pack catalog format: {format}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported Pack catalog version: {version}"),
            Self::InvalidEntry(pack) => write!(f, "invalid installed Pack entry: {}@{}", pack.id, pack.version),
            Self::DuplicateEntry(pack) => write!(f, "duplicate installed Pack entry: {}@{}", pack.id, pack.version),
            Self::AlreadyInstalled(pack) => write!(f, "Pack is already installed: {}@{}", pack.id, pack.version),
            Self::NotInstalled(pack) => write!(f, "Pack is not installed: {}@{}", pack.id, pack.version),
            Self::InvalidStoredPath(pack) => write!(f, "installed Pack contains a non-absolute path: {}@{}", pack.id, pack.version),
            Self::PackIdentityChanged { expected, found } => write!(f, "installed Pack identity changed: expected {}@{}, found {}@{}", expected.id, expected.version, found.id, found.version),
            Self::CommandPathChanged { pack, expected, found } => write!(f, "installed Pack {}@{} executable path changed: expected {}, found {}", pack.id, pack.version, expected.display(), found.display()),
            Self::ContentChanged { pack, component, expected, found } => write!(f, "installed Pack {}@{} {component} content changed: expected sha256 {expected}, found {found}", pack.id, pack.version),
            Self::Process(error) => write!(f, "could not validate installed Pack: {error}"),
        }
    }
}

impl Error for CatalogError {}

'''
s = s[:start] + replacement + s[end:]
p.write_text(s)

bpath = Path('scripts/check-boundaries.sh')
b = bpath.read_text()
if 'PACK_CATALOG="$ROOT/crates/world-pack-catalog"' not in b:
    b = b.replace('PACK_PROCESS="$ROOT/crates/world-pack-process"\n', 'PACK_PROCESS="$ROOT/crates/world-pack-process"\nPACK_CATALOG="$ROOT/crates/world-pack-catalog"\n', 1)
    b = b.replace('pack_process_forbidden=("TinySociety"', 'pack_catalog_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")\npack_process_forbidden=("TinySociety"', 1)
    guard = '''if [[ -d "$PACK_CATALOG" ]]; then
  for token in "${pack_catalog_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_CATALOG" >/tmp/world-machine-pack-catalog-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-catalog:"
      cat /tmp/world-machine-pack-catalog-boundary-check
      failed=1
    fi
  done
fi

'''
    b = b.replace('if [[ -d "$PACK_PROCESS" ]]; then\n', guard + 'if [[ -d "$PACK_PROCESS" ]]; then\n', 1)
bpath.write_text(b)
