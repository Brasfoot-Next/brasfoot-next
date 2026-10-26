//! Where a trophy's picture lives in a pack. The game keeps naming trophies as the original does
//! (`tr_nacional_ALE_d2`, saved in careers, see `brasfoot_simulation::trophy`); a pack stores
//! them by kind and FIFA code instead:
//!
//! ```text
//! trophies/league/ger.png      tr_nacional_ALE      (also the first division's)
//! trophies/league/ger-2.png    tr_nacional_ALE_d2
//! trophies/league/default.png  tr_nacionalgenerico
//! trophies/cup/bra.png         tr_copa_BRA          (cup/default.png: tr_copa)
//! trophies/super-cup/bra.png   tr_supercopa_BRA     (super-cup/default.png: tr_supercopa_generico)
//! trophies/state/sp.png        tr_estadual_SP       (state/sp-2.png, state/default.png)
//! trophies/regional/<slug>.png, trophies/continental/<slug>.png
//! ```

use crate::codes;

/// The regional cups (`f.F`'s names).
pub const REGIONAL: [(&str, &str); 4] =
    [("tr_riosaopaulo", "rio-sao-paulo"), ("tr_sulminas", "sul-minas"), ("tr_copanordeste", "copa-do-nordeste"), ("tr_copaverde", "copa-verde")];

/// The continental and world competitions (`best.at.mA()` of each class).
pub const CONTINENTAL: [(&str, &str); 22] = [
    ("tr_copaafrica", "africa-cup-of-nations"),
    ("tr_copaamerica", "copa-america"),
    ("tr_recopaeuropa", "european-super-cup"),
    ("tr_copaasia", "asian-cup"),
    // the original's spelling
    ("tr_recopasulamaericana", "recopa-sudamericana"),
    ("tr_copaofc", "ofc-nations-cup"),
    ("tr_copaouro", "gold-cup"),
    ("tr_sulamericana", "copa-sudamericana"),
    ("tr_conference", "conference-league"),
    ("tr_finalissima", "finalissima"),
    ("tr_liganacoes", "nations-league"),
    ("tr_liganacoesc", "nations-league-c"),
    ("tr_eurocopa", "euro"),
    ("tr_copamundo", "world-cup"),
    ("tr_libertadores", "libertadores"),
    ("tr_ligacaf", "caf-champions-league"),
    ("tr_ligaafc", "afc-champions-league"),
    ("tr_ligacampeoes", "champions-league"),
    ("tr_ligaconcacaf", "concacaf-champions-cup"),
    ("tr_ligaeuropa", "europa-league"),
    ("tr_ligaofc", "ofc-champions-league"),
    ("tr_mundial", "intercontinental-cup"),
];

/// The pack path of a trophy named the original's way. Lenient about case (`TR_nacional_tur`,
/// `_D1`), as the original is on Windows.
pub fn pack_path(stem: &str) -> Option<String> {
    let lower = stem.to_lowercase();
    let path = |kind: &str, name: &str| Some(format!("trophies/{kind}/{name}.png"));
    match lower.as_str() {
        "tr_nacionalgenerico" => return path("league", "default"),
        "tr_copa" => return path("cup", "default"),
        "tr_supercopa_generico" => return path("super-cup", "default"),
        "tr_estadualgenerico" => return path("state", "default"),
        _ => {}
    }
    if let Some((_, slug)) = REGIONAL.iter().find(|(s, _)| *s == lower) {
        return path("regional", slug);
    }
    if let Some((_, slug)) = CONTINENTAL.iter().find(|(s, _)| *s == lower) {
        return path("continental", slug);
    }
    let (kind, rest) = [("tr_nacional_", "league"), ("tr_copa_", "cup"), ("tr_supercopa_", "super-cup"), ("tr_estadual_", "state")]
        .into_iter()
        .find_map(|(prefix, kind)| lower.strip_prefix(prefix).map(|rest| (kind, rest)))?;
    let (code, division) = match rest.split_once("_d") {
        Some((code, d)) if kind == "league" || kind == "state" => (code, Some(d.parse::<i32>().ok().filter(|d| (1..=4).contains(d))?)),
        Some(_) => return None,
        None => (rest, None),
    };
    let name = if kind == "state" {
        codes::state_id(&code.to_uppercase())?;
        code.to_owned()
    } else {
        let id = brasfoot_model::countries::COUNTRIES.iter().find(|c| c.code.eq_ignore_ascii_case(code))?.id;
        codes::country_code(id)?.to_lowercase()
    };
    match division {
        Some(d) => path(kind, &format!("{name}-{d}")),
        None => path(kind, &name),
    }
}

#[cfg(test)]
mod tests {
    use super::pack_path;

    #[test]
    fn original_names_map_to_pack_paths() {
        assert_eq!(pack_path("tr_nacional_ALE_d2").as_deref(), Some("trophies/league/ger-2.png"));
        assert_eq!(pack_path("tr_nacional_ING").as_deref(), Some("trophies/league/eng.png"));
        assert_eq!(pack_path("TR_nacional_tur").as_deref(), Some("trophies/league/tur.png"));
        assert_eq!(pack_path("tr_estadual_RS_D1").as_deref(), Some("trophies/state/rs-1.png"));
        assert_eq!(pack_path("tr_copa_HOL").as_deref(), Some("trophies/cup/ned.png"));
        assert_eq!(pack_path("tr_supercopa_generico").as_deref(), Some("trophies/super-cup/default.png"));
        assert_eq!(pack_path("tr_recopasulamaericana").as_deref(), Some("trophies/continental/recopa-sudamericana.png"));
        assert_eq!(pack_path("tr_liganacoesC").as_deref(), Some("trophies/continental/nations-league-c.png"));
        // not a country of the game, not a division
        assert_eq!(pack_path("tr_copa_NZE"), None);
        assert_eq!(pack_path("tr_nacional_RUS1"), None);
    }
}
