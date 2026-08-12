from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
old = '''#[derive(Clone, Debug, PartialEq)]
pub struct PackInstallPreview {
    pub source_path: PathBuf,
    pub kind: PackInstallKind,
    pub pack: WorldPackRef,
    pub title: String,
    pub description: String,
    pub runtime_name: String,
    pub program_bytes: u64,
    pub program_sha256: String,
    evidence: PackInstallEvidence,
}
'''
new = '''#[derive(Clone, Debug, PartialEq)]
pub struct PackInstallPreview {
    source_path: PathBuf,
    kind: PackInstallKind,
    pack: WorldPackRef,
    title: String,
    description: String,
    runtime_name: String,
    program_bytes: u64,
    program_sha256: String,
    evidence: PackInstallEvidence,
}

impl PackInstallPreview {
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn kind(&self) -> PackInstallKind {
        self.kind
    }

    pub fn pack(&self) -> &WorldPackRef {
        &self.pack
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn runtime_name(&self) -> &str {
        &self.runtime_name
    }

    pub fn program_bytes(&self) -> u64 {
        self.program_bytes
    }

    pub fn program_sha256(&self) -> &str {
        &self.program_sha256
    }
}
'''
if old not in s:
    raise SystemExit('preview struct marker missing')
s = s.replace(old, new, 1)
# Internal catalog accesses can stay as fields. Tests are in-crate and intentionally can inspect fields.
p.write_text(s)

p = Path('apps/world-machine-desktop/src/main.rs')
s = p.read_text()
s = s.replace('#[cfg(target_os = "macos")]\nuse world_pack_bundle::PACK_BUNDLE_SUFFIX;\n', '', 1)
s = s.replace('preview.pack.id, preview.pack.version', 'preview.pack().id, preview.pack().version')
s = s.replace('let format = preview.kind.label();', 'let format = preview.kind().label();')
s = s.replace('let size = format_program_size(preview.program_bytes);', 'let size = format_program_size(preview.program_bytes());')
s = s.replace('let source = preview.source_path.display().to_string();', 'let source = preview.source_path().display().to_string();')
s = s.replace('let pack = format!("{} @ {}", preview.pack.id, preview.pack.version);', 'let pack = format!("{} @ {}", preview.pack().id, preview.pack().version);')
s = s.replace('let runtime = preview.runtime_name.clone();', 'let runtime = preview.runtime_name().to_owned();')
s = s.replace('let sha = preview.program_sha256.clone();', 'let sha = preview.program_sha256().to_owned();')
s = s.replace('.child(div().text_lg().child(preview.title))', '.child(div().text_lg().child(preview.title().to_owned()))')
s = s.replace('.child(div().text_sm().text_color(rgb(0x666666)).child(preview.description))', '.child(\n                div()\n                    .text_sm()\n                    .text_color(rgb(0x666666))\n                    .child(preview.description().to_owned()),\n            )')
p.write_text(s)
