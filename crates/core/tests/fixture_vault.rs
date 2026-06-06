use lattice_core::Vault;
use std::path::Path;

fn vault() -> Vault {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/vault");
    Vault::open_in_memory(&root).unwrap()
}

#[test]
fn aiignore_excludes_secret() {
    let all = vault().query(&[], None).unwrap();
    assert!(
        !all.iter().any(|n| n.path == "secret.md"),
        "secret.md must be ignored"
    );
}

#[test]
fn missing_note_is_a_broken_link() {
    let bl = vault().broken_links(None, 1000).unwrap();
    assert!(bl.iter().any(|e| e.raw_target == "missing-note"));
}

#[test]
fn guide_has_expected_backlinks() {
    let bl = vault().backlinks("docs/guide.md").unwrap();
    // api links to guide; docs/README links to guide
    assert!(bl.iter().any(|e| e.src == "docs/api.md"));
    assert!(bl.iter().any(|e| e.src == "docs/README.md"));
}

#[test]
fn frontmatter_query_finds_active_specs() {
    let r = vault()
        .query(&[("type", "spec"), ("status", "active")], None)
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].path, "docs/guide.md");
}

fn walkdir(p: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = vec![];
    for e in std::fs::read_dir(p).unwrap() {
        let path = e.unwrap().path();
        if path.is_dir() {
            out.push(path.clone());
            out.extend(walkdir(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn rename_repairs_fixture_backlinks_in_memory() {
    use std::fs;
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/vault");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for entry in walkdir(&src) {
        let rel = entry.strip_prefix(&src).unwrap();
        let dest = root.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&dest).unwrap();
        } else {
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::copy(&entry, &dest).unwrap();
        }
    }
    let mut vault = Vault::open_in_memory(root).unwrap();
    let plan = vault
        .rename("docs/guide.md", "docs/manual.md", true)
        .unwrap();
    assert!(plan.applied);
    assert!(vault
        .backlinks("docs/manual.md")
        .unwrap()
        .iter()
        .any(|e| e.src == "docs/api.md"));
}
