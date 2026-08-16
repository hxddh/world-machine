from pathlib import Path

path = Path("crates/world-pack-protocol/src/lib.rs")
text = path.read_text()
old = """        if self.protocol_version != PACK_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedProtocolVersion(
                self.protocol_version,
            ));
        }
"""
new = """        validate_protocol_version(self.protocol_version)?;
"""
count = text.count(old)
assert count == 1, f"manifest protocol validation: expected 1 match, found {count}"
path.write_text(text.replace(old, new, 1))
