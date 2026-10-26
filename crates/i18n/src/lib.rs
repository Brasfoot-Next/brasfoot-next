//! User-facing text. Every string shown to the player lives in a Fluent catalog under
//! `locales/<lang>/*.ftl` and is looked up by key; code never holds literal UI text.

pub mod labels;

use std::collections::HashMap;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentArgs as Args;

pub const DEFAULT_LOCALE: &str = "pt-BR";

/// Catalogs are compiled into the binary; one entry per locale, each a list of .ftl sources.
const CATALOGS: &[(&str, &[&str])] = &[(
    "pt-BR",
    &[include_str!("../../../locales/pt-BR/main.ftl"), include_str!("../../../locales/pt-BR/countries.ftl"), include_str!("../../../locales/pt-BR/new-game.ftl"), include_str!("../../../locales/pt-BR/messages.ftl"), include_str!("../../../locales/pt-BR/awards.ftl"), include_str!("../../../locales/pt-BR/economy.ftl"), include_str!("../../../locales/pt-BR/staff.ftl"), include_str!("../../../locales/pt-BR/fans.ftl"), include_str!("../../../locales/pt-BR/sponsors.ftl"), include_str!("../../../locales/pt-BR/assets.ftl"), include_str!("../../../locales/pt-BR/satellites.ftl"), include_str!("../../../locales/pt-BR/super-mundial.ftl"), include_str!("../../../locales/pt-BR/youth-league.ftl"), include_str!("../../../locales/pt-BR/club.ftl"), include_str!("../../../locales/pt-BR/management.ftl")],
)];

pub struct Translator {
    bundle: FluentBundle<FluentResource>,
}

impl Translator {
    pub fn new(locale: &str) -> Self {
        let (id, sources) = CATALOGS
            .iter()
            .find(|(id, _)| *id == locale)
            .unwrap_or(&CATALOGS[0]);
        let lang: LanguageIdentifier = id.parse().expect("valid locale id");
        let mut bundle = FluentBundle::new(vec![lang]);
        // Isolation marks around arguments render as stray glyphs in egui.
        bundle.set_use_isolating(false);
        for source in *sources {
            let resource = FluentResource::try_new(source.to_string())
                .unwrap_or_else(|(_, errors)| panic!("invalid catalog {id}: {errors:?}"));
            bundle
                .add_resource(resource)
                .unwrap_or_else(|errors| panic!("duplicate keys in {id}: {errors:?}"));
        }
        Self { bundle }
    }

    /// Looks up `key`; a missing key shows the key itself so it is easy to spot on screen.
    pub fn text(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let Some(pattern) = self.bundle.get_message(key).and_then(|m| m.value()) else {
            return key.to_string();
        };
        let mut errors = vec![];
        self.bundle.format_pattern(pattern, args, &mut errors).into_owned()
    }
}

/// Builds Fluent arguments from `(name, value)` pairs.
pub fn args<'a, V: Into<FluentValue<'a>>>(pairs: impl IntoIterator<Item = (&'a str, V)>) -> FluentArgs<'a> {
    let mut args = FluentArgs::new();
    for (name, value) in pairs {
        args.set(name, value);
    }
    args
}

/// Every key of every catalog, for the completeness test.
pub fn keys(locale: &str) -> HashMap<String, ()> {
    let (_, sources) = CATALOGS.iter().find(|(id, _)| *id == locale).expect("known locale");
    sources
        .iter()
        .flat_map(|s| FluentResource::try_new(s.to_string()).ok())
        .flat_map(|r| {
            r.entries()
                .filter_map(|e| match e {
                    fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .map(|k| (k, ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_and_formats() {
        let t = Translator::new(DEFAULT_LOCALE);
        assert_eq!(t.text("app-title", None), "Brasfoot Next");
        assert_eq!(t.text("missing-key", None), "missing-key");
    }
}
