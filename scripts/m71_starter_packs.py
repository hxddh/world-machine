from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


main_path = ROOT / "apps/world-machine-desktop/src/main.rs"
text = main_path.read_text()
text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
''',
    '''#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::{Component, Path, PathBuf};
''',
    "filesystem imports",
)
text = replace_once(
    text,
    '''use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog, PackInstallPreview};
''',
    '''use world_pack_catalog::{
    CatalogError, InstalledPack, PackAvailability, PackCatalog, PackInstallPreview,
};
''',
    "catalog error import",
)
text = replace_once(
    text,
    '''const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";
''',
    '''const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";
#[cfg(target_os = "macos")]
const STARTER_PACKS_OVERRIDE_ENV: &str = "WORLD_MACHINE_STARTER_PACKS_DIR";
''',
    "starter override constant",
)
text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
struct WorldMachineHome {
''',
    '''#[cfg(target_os = "macos")]
#[derive(Clone)]
struct StarterPackCard {
    title: String,
    description: String,
    pack: WorldPackRef,
    preview: Option<PackInstallPreview>,
}

#[cfg(target_os = "macos")]
impl StarterPackCard {
    fn available(preview: PackInstallPreview) -> Self {
        Self {
            title: preview.title().to_owned(),
            description: preview.description().to_owned(),
            pack: preview.pack().clone(),
            preview: Some(preview),
        }
    }

    fn installed(pack: InstalledPack) -> Self {
        Self {
            title: pack.title,
            description: pack.description,
            pack: pack.pack,
            preview: None,
        }
    }
}

#[cfg(target_os = "macos")]
struct WorldMachineHome {
''',
    "starter card model",
)
text = replace_once(
    text,
    '''    pending_pack_install: Option<PackInstallPreview>,
    probing_packs: Vec<WorldPackRef>,
''',
    '''    pending_pack_install: Option<PackInstallPreview>,
    starter_packs: Vec<StarterPackCard>,
    probing_packs: Vec<WorldPackRef>,
''',
    "starter home field",
)
text = replace_once(
    text,
    '''    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    fn install_pack(&mut self, cx: &mut Context<Self>) {
''',
    '''    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    fn reload_starter_packs(&mut self) -> Result<(), String> {
        self.starter_packs = load_starter_packs(self.pack_catalog.as_ref())?;
        Ok(())
    }

    fn review_starter_pack(&mut self, preview: PackInstallPreview, cx: &mut Context<Self>) {
        self.status = Some(format!(
            "Review bundled Starter Pack {} @ {} before trusting its executable bytes",
            preview.pack().id,
            preview.pack().version
        ));
        self.pending_pack_install = Some(preview);
        cx.notify();
    }

    fn install_pack(&mut self, cx: &mut Context<Self>) {
''',
    "starter review methods",
)
text = replace_once(
    text,
    '''            Ok(installed) => {
                self.start_pack_probe(installed.pack, true, cx);
            }
''',
    '''            Ok(installed) => {
                let _ = self.reload_starter_packs();
                self.start_pack_probe(installed.pack, true, cx);
            }
''',
    "refresh starters after install",
)
card_anchor = '''    fn installed_pack_card(&self, pack: InstalledPack, cx: &mut Context<Self>) -> impl IntoElement {
'''
starter_card = '''    fn starter_pack_card(
        &self,
        starter: StarterPackCard,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_label = format!("{} @ {}", starter.pack.id, starter.pack.version);
        let mut actions = div().flex().gap_2();
        let state = if let Some(preview) = starter.preview.clone() {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "review-starter-pack-{}-{}",
                        starter.pack.id, starter.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xd9d9d3))
                    .text_sm()
                    .child("Review & Install")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_starter_pack(preview.clone(), cx)
                    })),
            );
            "Bundled starter · Not installed"
        } else {
            "Bundled starter · Already installed · manage under Installed Packs"
        };

        div()
            .id(SharedString::from(format!(
                "starter-pack-{}-{}",
                starter.pack.id, starter.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(starter.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child(starter.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8a8a82))
                            .child(format!("{pack_label} · {state}")),
                    ),
            )
            .child(actions)
    }

'''
text = replace_once(text, card_anchor, starter_card + card_anchor, "starter card UI")
text = replace_once(
    text,
    '''        let installed_packs = self
            .pack_catalog
''',
    '''        let starter_packs = self.starter_packs.clone();
        let mut starters = div().w_full().flex().flex_col().gap_3();
        for starter in starter_packs {
            starters = starters.child(self.starter_pack_card(starter, cx));
        }

        let installed_packs = self
            .pack_catalog
''',
    "render starter collection",
)
text = replace_once(
    text,
    '''        body = body
            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("Installed Packs"))
            .child(installed)
            .child(div().text_sm().child("New World"))
            .child(available);
''',
    '''        body = body
            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("Installed Packs"))
            .child(installed);
        if !self.starter_packs.is_empty() {
            body = body
                .child(div().text_sm().child("Starter Packs"))
                .child(starters);
        }
        body = body
            .child(div().text_sm().child("New World"))
            .child(available);
''',
    "render starter section",
)
function_anchor = '''#[cfg(target_os = "macos")]
fn build_registry(catalog: Option<&PackCatalog>) -> Result<world_host::WorldRegistry, String> {
'''
starter_functions = r'''#[cfg(target_os = "macos")]
fn starter_pack_dir_from_executable(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    if macos.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return None;
    }
    let contents = macos.parent()?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return None;
    }
    Some(contents.join("Resources").join("StarterPacks"))
}

#[cfg(target_os = "macos")]
fn discover_starter_pack_dir() -> std::io::Result<Option<PathBuf>> {
    if let Some(path) = env::var_os(STARTER_PACKS_OVERRIDE_ENV) {
        return Ok(Some(PathBuf::from(path)));
    }
    let executable = env::current_exe()?;
    Ok(starter_pack_dir_from_executable(&executable))
}

#[cfg(target_os = "macos")]
fn starter_pack_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.try_exists().map_err(|error| {
        format!("Could not inspect Starter Packs directory {}: {error}", root.display())
    })? {
        return Ok(Vec::new());
    }
    let index = root.join("index.txt");
    if !index.try_exists().map_err(|error| {
        format!("Could not inspect Starter Packs index {}: {error}", index.display())
    })? {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&index)
        .map_err(|error| format!("Could not read Starter Packs index {}: {error}", index.display()))?;
    let mut names = std::collections::BTreeSet::new();
    let mut paths = Vec::new();
    for (line_index, raw) in contents.lines().enumerate() {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        let relative = Path::new(name);
        let mut components = relative.components();
        let valid_name = matches!(components.next(), Some(Component::Normal(_)))
            && components.next().is_none()
            && name.ends_with(".worldpack");
        if !valid_name {
            return Err(format!(
                "Starter Packs index {} line {} is not a single .worldpack file name: {name:?}",
                index.display(),
                line_index + 1
            ));
        }
        if !names.insert(name.to_owned()) {
            return Err(format!(
                "Starter Packs index {} contains duplicate entry {name:?}",
                index.display()
            ));
        }
        let path = root.join(name);
        if !path.is_file() {
            return Err(format!(
                "Starter Pack listed by {} is missing or not a file: {}",
                index.display(),
                path.display()
            ));
        }
        paths.push(path);
    }
    Ok(paths)
}

#[cfg(target_os = "macos")]
fn load_starter_packs(catalog: Option<&PackCatalog>) -> Result<Vec<StarterPackCard>, String> {
    let Some(catalog) = catalog else {
        return Ok(Vec::new());
    };
    let Some(root) = discover_starter_pack_dir().map_err(|error| {
        format!("Could not locate bundled Starter Packs: {error}")
    })? else {
        return Ok(Vec::new());
    };

    let mut starters = Vec::new();
    for path in starter_pack_paths(&root)? {
        match catalog.inspect_install(&path) {
            Ok(preview) => starters.push(StarterPackCard::available(preview)),
            Err(CatalogError::AlreadyInstalled(pack)) => {
                let installed = catalog.entry(&pack).cloned().ok_or_else(|| {
                    format!(
                        "Starter Pack {} @ {} reported as installed but has no catalog entry",
                        pack.id, pack.version
                    )
                })?;
                starters.push(StarterPackCard::installed(installed));
            }
            Err(error) => {
                return Err(format!(
                    "Could not inspect bundled Starter Pack {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Ok(starters)
}

'''
text = replace_once(text, function_anchor, starter_functions + function_anchor, "starter discovery functions")
text = replace_once(
    text,
    '''    let (documents, lineage, library_status) = match library.list() {
''',
    '''    let (starter_packs, starter_status) = match load_starter_packs(pack_catalog.as_ref()) {
        Ok(starters) => (starters, None),
        Err(error) => (Vec::new(), Some(error)),
    };
    let (documents, lineage, library_status) = match library.list() {
''',
    "startup starter load",
)
text = replace_once(
    text,
    '''    let status = pack_status.or(library_status);
''',
    '''    let status = pack_status.or(starter_status).or(library_status);
''',
    "startup starter status",
)
text = replace_once(
    text,
    '''                pending_pack_install: None,
                probing_packs: Vec::new(),
''',
    '''                pending_pack_install: None,
                starter_packs,
                probing_packs: Vec::new(),
''',
    "home starter initialization",
)

text += r'''

#[cfg(all(test, target_os = "macos"))]
mod starter_pack_tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "world-machine-starter-packs-{label}-{}-{nonce}",
            process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn app_executable_maps_only_to_signed_resources_layout() {
        let executable = Path::new(
            "/Applications/World Machine.app/Contents/MacOS/world-machine-desktop",
        );
        assert_eq!(
            starter_pack_dir_from_executable(executable),
            Some(PathBuf::from(
                "/Applications/World Machine.app/Contents/Resources/StarterPacks"
            ))
        );
        assert_eq!(
            starter_pack_dir_from_executable(Path::new("/tmp/world-machine-desktop")),
            None
        );
    }

    #[test]
    fn starter_pack_index_is_explicit_ordered_and_does_not_scan() {
        let root = temp_dir("explicit");
        fs::write(root.join("first.worldpack"), b"first").unwrap();
        fs::write(root.join("second.worldpack"), b"second").unwrap();
        fs::write(root.join("not-indexed.worldpack"), b"ignored").unwrap();
        fs::write(
            root.join("index.txt"),
            "second.worldpack\nfirst.worldpack\n",
        )
        .unwrap();

        assert_eq!(
            starter_pack_paths(&root).unwrap(),
            vec![root.join("second.worldpack"), root.join("first.worldpack")]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn starter_pack_index_rejects_traversal_nested_absolute_non_bundle_and_duplicates() {
        let root = temp_dir("invalid");
        for invalid in [
            "../escape.worldpack\n",
            "nested/pack.worldpack\n",
            "/tmp/absolute.worldpack\n",
            "not-a-pack.txt\n",
            "same.worldpack\nsame.worldpack\n",
        ] {
            fs::write(root.join("index.txt"), invalid).unwrap();
            let error = starter_pack_paths(&root).expect_err("unsafe index entry must fail");
            assert!(error.contains("Starter Packs index"));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_starter_pack_index_is_a_normal_unbundled_dev_state() {
        let root = temp_dir("missing-index");
        assert!(starter_pack_paths(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
'''

main_path.write_text(text)

build_path = ROOT / "apps/world-machine-desktop/macos/build-app.sh"
build = build_path.read_text()
build = replace_once(
    build,
    '''    release)
        cargo build -p world-machine-desktop --release
        PROFILE_DIR="release"
        ;;
    debug)
        cargo build -p world-machine-desktop
        PROFILE_DIR="debug"
''',
    '''    release)
        cargo build -p world-machine-desktop -p pocket-universe-pack -p micro-company-pack --release
        PROFILE_DIR="release"
        ;;
    debug)
        cargo build -p world-machine-desktop -p pocket-universe-pack -p micro-company-pack
        PROFILE_DIR="debug"
''',
    "build starter pack binaries",
)
build = replace_once(
    build,
    '''BINARY_PATH="$TARGET_DIR/$PROFILE_DIR/$BINARY_NAME"
if [[ ! -x "$BINARY_PATH" ]]; then
    echo "built binary is missing or not executable: $BINARY_PATH" >&2
    exit 1
fi

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_DIR/Contents/MacOS/$BINARY_NAME"
sed "s/@VERSION@/$VERSION/g" "$PLIST_TEMPLATE" > "$APP_DIR/Contents/Info.plist"
''',
    '''BINARY_PATH="$TARGET_DIR/$PROFILE_DIR/$BINARY_NAME"
POCKET_PACK_BINARY="$TARGET_DIR/$PROFILE_DIR/pocket-universe-pack"
MICRO_COMPANY_PACK_BINARY="$TARGET_DIR/$PROFILE_DIR/micro-company-pack"
for BUILT_BINARY in "$BINARY_PATH" "$POCKET_PACK_BINARY" "$MICRO_COMPANY_PACK_BINARY"; do
    if [[ ! -x "$BUILT_BINARY" ]]; then
        echo "built binary is missing or not executable: $BUILT_BINARY" >&2
        exit 1
    fi
done

rm -rf "$APP_DIR"
STARTER_PACKS_DIR="$APP_DIR/Contents/Resources/StarterPacks"
mkdir -p "$APP_DIR/Contents/MacOS" "$STARTER_PACKS_DIR"
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_DIR/Contents/MacOS/$BINARY_NAME"
"$POCKET_PACK_BINARY" --write-bundle "$STARTER_PACKS_DIR/pocket-universe.worldpack"
"$MICRO_COMPANY_PACK_BINARY" --write-bundle "$STARTER_PACKS_DIR/micro-company.worldpack"
printf '%s\n' \
    'pocket-universe.worldpack' \
    'micro-company.worldpack' \
    > "$STARTER_PACKS_DIR/index.txt"
sed "s/@VERSION@/$VERSION/g" "$PLIST_TEMPLATE" > "$APP_DIR/Contents/Info.plist"
''',
    "bundle starter pack resources",
)
build = replace_once(
    build,
    '''plutil -lint "$APP_DIR/Contents/Info.plist"

python3 - "$APP_DIR/Contents/Info.plist" <<'PY'
''',
    '''plutil -lint "$APP_DIR/Contents/Info.plist"

python3 - "$STARTER_PACKS_DIR" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
expected = ["pocket-universe.worldpack", "micro-company.worldpack"]
lines = [line.strip() for line in (root / "index.txt").read_text().splitlines() if line.strip()]
assert lines == expected, (lines, expected)
for name in expected:
    path = root / name
    assert path.is_file(), path
    assert path.stat().st_size > 0, path
assert sorted(path.name for path in root.iterdir()) == sorted(expected + ["index.txt"])
PY

python3 - "$APP_DIR/Contents/Info.plist" <<'PY'
''',
    "validate starter resources",
)
build_path.write_text(build)
