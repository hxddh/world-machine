from pathlib import Path

p = Path('scripts/m56_managed_store.py')
s = p.read_text()
start = s.index('# Add display arms after InvalidStoredPath.')
end = s.index('# Existing install persistence assertion gets managed semantics.', start)
replacement = '''# Add display arms after InvalidStoredPath.
needle = '            Self::InvalidStoredPath(pack) => write!(f, "installed Pack contains a non-absolute path: {}@{}", pack.id, pack.version),\\n'
addition = needle + \'''            Self::InvalidManagedPath(pack) => write!(
                f,
                "managed Pack paths do not match the catalog-owned store: {}@{}",
                pack.id, pack.version
            ),
            Self::ManagedDestinationExists(pack) => write!(
                f,
                "managed Pack destination already exists for {}@{}",
                pack.id, pack.version
            ),
\'''
if needle not in s:
    raise SystemExit('error display marker not found')
s = s.replace(needle, addition, 1)

'''
s = s[:start] + replacement + s[end:]
p.write_text(s)
