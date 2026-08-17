from pathlib import Path
path = Path("crates/world-query/src/lib.rs")
text = path.read_text()
count = text.count("EvidenceWhyNode")
if count != 3:
    raise SystemExit(f"unexpected EvidenceWhyNode occurrence count: {count}")
text = text.replace("EvidenceWhyNode", "EvidenceCausalNode")
path.write_text(text)
