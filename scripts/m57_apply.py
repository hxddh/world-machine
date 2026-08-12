from pathlib import Path

server = Path('crates/world-pack-server/src/lib.rs')
s = server.read_text()
s = s.replace('use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};', 'use std::io::{self, BufRead, BufWriter, Read, Write};', 1)
s = s.replace('use std::path::PathBuf;\n', '', 1)
s = s.replace('    use std::io::Cursor;\n', '    use std::io::Cursor;\n    use std::path::PathBuf;\n', 1)
server.write_text(s)

boundary = Path('scripts/check-boundaries.sh')
b = boundary.read_text()
if 'PACK_SERVER="$ROOT/crates/world-pack-server"' not in b:
    b = b.replace(
        'PACK_CATALOG="$ROOT/crates/world-pack-catalog"\n',
        'PACK_CATALOG="$ROOT/crates/world-pack-catalog"\nPACK_SERVER="$ROOT/crates/world-pack-server"\n',
        1,
    )
    b = b.replace(
        'pack_catalog_forbidden=("TinySociety"',
        'pack_server_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")\npack_catalog_forbidden=("TinySociety"',
        1,
    )
    marker = 'if [[ -d "$PACK_CATALOG" ]]; then\n'
    guard = '''if [[ -d "$PACK_SERVER" ]]; then
  for token in "${pack_server_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_SERVER" >/tmp/world-machine-pack-server-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-server:"
      cat /tmp/world-machine-pack-server-boundary-check
      failed=1
    fi
  done
fi

'''
    b = b.replace(marker, guard + marker, 1)
boundary.write_text(b)
