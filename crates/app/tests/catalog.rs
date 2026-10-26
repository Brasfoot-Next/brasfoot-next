//! Every catalog key the app asks for by name exists in pt-BR.

use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(std::fs::read_to_string(path).unwrap());
        }
    }
}

#[test]
fn every_key_is_in_the_catalog() {
    let keys = brasfoot_i18n::keys(brasfoot_i18n::DEFAULT_LOCALE);
    let mut texts = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut texts);
    let mut missing = Vec::new();
    for text in &texts {
        for (i, _) in text.match_indices("t.text(\"") {
            let rest = &text[i + 8..];
            let key = &rest[..rest.find('"').unwrap()];
            if !keys.contains_key(key) {
                missing.push(key.to_owned());
            }
        }
        // keys written as string literals in lists: "area-name" literals with a known prefix
        for prefix in ["\"squad-column-", "\"tables-column-", "\"calendar-column-", "\"toolbar-", "\"player-column-", "\"home-next-"] {
            for (i, _) in text.match_indices(prefix) {
                let rest = &text[i + 1..];
                let key = &rest[..rest.find('"').unwrap()];
                if !keys.contains_key(key) {
                    missing.push(key.to_owned());
                }
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "missing in locales/pt-BR: {missing:?}");
}
