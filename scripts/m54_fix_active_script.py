from pathlib import Path

p = Path('scripts/m54_active_patch.py')
s = p.read_text()
start = s.index("needle = '            Self::NotInstalled(pack)")
end = s.index("s = s.replace('        assert!(installed.enabled);", start)
replacement = '''needle = '            Self::NotInstalled(pack) => write!(f, "Pack is not installed: {}@{}", pack.id, pack.version),\\n'
addition = needle + \'''            Self::DisabledCannotActivate(pack) => write!(
                f,
                "disabled Pack cannot become active: {}@{}",
                pack.id, pack.version
            ),
            Self::ActivePackRequiresReplacement(pack) => write!(
                f,
                "active Pack {}@{} has another enabled version; activate its replacement first",
                pack.id, pack.version
            ),
            Self::InvalidActiveSelection(id) => write!(
                f,
                "Pack catalog must select exactly one active enabled version for {id}"
            ),
\'''
if needle not in s:
    raise SystemExit('display marker not found')
s = s.replace(needle, addition, 1)

'''
s = s[:start] + replacement + s[end:]
p.write_text(s)
