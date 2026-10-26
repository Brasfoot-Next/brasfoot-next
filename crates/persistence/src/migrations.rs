//! The upgrades of older `.bfn` payloads, run on the CBOR value of a saved `World` before it is
//! deserialized (see [`crate::native::MIGRATIONS`]).

use ciborium::Value;

fn field<'a>(map: &'a Value, name: &str) -> Option<&'a Value> {
    map.as_map()?.iter().find(|(k, _)| k.as_text() == Some(name)).map(|(_, v)| v)
}

fn field_mut<'a>(map: &'a mut Value, name: &str) -> Option<&'a mut Value> {
    map.as_map_mut()?.iter_mut().find(|(k, _)| k.as_text() == Some(name)).map(|(_, v)| v)
}

fn set(map: &mut Value, name: &str, value: i64) {
    let value = Value::Integer(value.into());
    match field_mut(map, name) {
        Some(v) => *v = value,
        None => {
            if let Some(entries) = map.as_map_mut() {
                entries.push((Value::Text(name.to_owned()), value));
            }
        }
    }
}

fn index(value: &Value) -> Option<usize> {
    value.as_integer().and_then(|i| usize::try_from(i).ok())
}

fn ids(game: Option<&Value>, name: &str) -> Vec<usize> {
    game.and_then(|g| field(g, name)).and_then(Value::as_array).map(|a| a.iter().filter_map(index).collect()).unwrap_or_default()
}

/// Version 2 keeps the numbers `g.b.Ai()` gives the players when the game saves
/// (`Player::save_number`, `save_list`) and the ones the transfer history records
/// (`Transfer::player_number`, `player_list`). A version 1 save gets the numbers its next save
/// would have given, so its history relinks to the same players on load. A player in neither
/// list gets -1, as his number from an older save is gone; an imported save's transfers point
/// only at listed players (the import relinks them like `ap.me()`), so none of them is lost.
pub fn add_save_numbers(world: &mut Value) {
    let game = field(world, "game");
    let lists = [(1, ids(game, "players")), (2, ids(game, "youth_players"))];
    let count = field(world, "players").and_then(Value::as_array).map_or(0, Vec::len);
    let mut numbers = vec![(-1, -1); count];
    let mut number = 0;
    for (list, players) in &lists {
        for &p in players {
            if let Some(slot) = numbers.get_mut(p) {
                *slot = (number, *list);
            }
            number += 1;
        }
    }
    if let Some(players) = field_mut(world, "players").and_then(Value::as_array_mut) {
        for (player, &(number, list)) in players.iter_mut().zip(&numbers) {
            set(player, "save_number", number);
            set(player, "save_list", list);
        }
    }
    let transfers = field_mut(world, "game").and_then(|g| field_mut(g, "transfers")).and_then(Value::as_array_mut);
    for t in transfers.into_iter().flatten() {
        let (number, list) = field(t, "player").and_then(index).and_then(|p| numbers.get(p).copied()).unwrap_or((-1, -1));
        set(t, "player_number", number);
        set(t, "player_list", list);
    }
}

fn rename(map: &mut Value, old: &str, new: &str) {
    if let Some(entries) = map.as_map_mut() {
        for (k, _) in entries.iter_mut() {
            if k.as_text() == Some(old) {
                *k = Value::Text(new.to_owned());
            }
        }
    }
}

/// Version 3 names what version 2 left numbered or misnamed (the first imports called the
/// Finalíssima, the Sudamericana and the stadium works otherwise, swapped the Europa and the
/// Conference League, and named the world club cup's fields out of order): the game's World Cup and continental cups
/// seasons (`bl`..`br`, `IZ`, `Ja`, `Jb`), its next club id (`bg`), its qualifiers (`aL`..`aQ`,
/// and their classes), and a season record's move up or down (`nO`). The records of the
/// players' competitions keep the number the game saved (`H.ft`) beside the link: a version 2
/// save had the link only, so the number becomes its competition's (`at.ei`). The Euro's and
/// the World Cup's qualified (`YM`) hold countries now: a version 2 save could only have them
/// empty.
pub fn name_national_tournaments(world: &mut Value) {
    const GAME: [(&str, &str); 16] = [
        ("value_bl", "world_cup_season"),
        ("value_bm", "euro_season"),
        ("value_bn", "copa_america_season"),
        ("value_bo", "africa_cup_season"),
        ("value_bp", "asian_cup_season"),
        ("value_bq", "oceania_cup_season"),
        ("value_br", "gold_cup_season"),
        ("value_iz", "finalissima_season"),
        ("value_ja", "nations_league_m_season"),
        ("value_jb", "nations_league_n_season"),
        ("value_bg", "next_club_id"),
        ("qualifiers_n", "south_american_qualifiers"),
        ("qualifiers_i", "african_qualifiers"),
        ("qualifiers_playoff", "asian_qualifiers"),
        ("qualifiers_m", "oceania_qualifiers"),
        ("qualifiers_k", "concacaf_qualifiers"),
    ];
    const CLASSES: [(&str, &str); 5] = [
        ("QualifiersN", "SouthAmericanQualifiers"),
        ("QualifiersI", "AfricanQualifiers"),
        ("QualifiersPlayoff", "AsianQualifiers"),
        ("QualifiersM", "OceaniaQualifiers"),
        ("QualifiersK", "ConcacafQualifiers"),
    ];
    if let Some(game) = field_mut(world, "game") {
        for (old, new) in GAME {
            rename(game, old, new);
        }
        // the names of the first imports: IX, aY and aw, and the second tiers of Europe swapped
        // (`IY` was europa_league and `aX` conference_league)
        if field(game, "continental_cup_h").is_some() {
            rename(game, "europa_league", "__conference_league");
            rename(game, "conference_league", "europa_league");
            rename(game, "__conference_league", "conference_league");
            rename(game, "continental_cup_h", "copa_sudamericana");
        }
        rename(game, "club_world_cup", "finalissima");
        rename(game, "list_aw", "stadium_works");
    }
    let mut legacy_ids = Vec::new();
    if let Some(competitions) = field_mut(world, "competitions").and_then(Value::as_array_mut) {
        for c in competitions.iter_mut() {
            let kind = field(c, "kind").and_then(Value::as_integer).and_then(|i| i64::try_from(i).ok()).unwrap_or(-1);
            if let Some(class) = field_mut(c, "class")
                && let Some(name) = class.as_text()
            {
                let new = match (name, kind) {
                    // f.x (kind 6) and f.J (kind 12) had each other's names
                    ("ConferenceLeague", 6) => Some("EuropaLeague"),
                    ("EuropaLeague", 12) => Some("ConferenceLeague"),
                    ("ClubWorldCup", _) => Some("Finalissima"),
                    ("ContinentalCupH", _) => Some("CopaSudamericana"),
                    _ => CLASSES.iter().find(|(old, _)| *old == name).map(|(_, new)| *new),
                };
                if let Some(new) = new {
                    *class = Value::Text(new.to_owned());
                }
            }
            // f.A's fields, by their place in the class (ZM..ZT)
            for (old, new) in [
                ("semi_final_1", "quarter_finals"),
                ("semi_final_2", "semi_finals"),
                ("libertadores_champion", "europe_champion"),
                ("champions_league_champion", "south_america_champion"),
                ("semi_finalist_1", "africa_champion"),
                ("semi_finalist_2", "asia_champion"),
                ("finalist_1", "oceania_champion"),
                ("finalist_2", "concacaf_champion"),
            ] {
                rename(c, old, new);
            }
            if let Some(ym) = field_mut(c, "list_ym") {
                *ym = Value::Array(Vec::new());
            }
            legacy_ids.push(field(c, "legacy_id").and_then(Value::as_integer).and_then(|i| i64::try_from(i).ok()).unwrap_or(-1));
        }
    }
    if let Some(clubs) = field_mut(world, "clubs").and_then(Value::as_array_mut) {
        for club in clubs.iter_mut() {
            if let Some(records) = field_mut(club, "competition_records").and_then(Value::as_array_mut) {
                for r in records.iter_mut() {
                    rename(r, "value_no", "league_move");
                }
            }
        }
    }
    if let Some(players) = field_mut(world, "players").and_then(Value::as_array_mut) {
        for player in players.iter_mut() {
            if let Some(stats) = field_mut(player, "competition_stats").and_then(Value::as_array_mut) {
                for h in stats.iter_mut() {
                    let number = field(h, "competition_id").and_then(index).and_then(|c| legacy_ids.get(c).copied()).filter(|&n| n >= 0).unwrap_or(-1);
                    set(h, "competition_number", number);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: Vec<(&str, Value)>) -> Value {
        Value::Map(entries.into_iter().map(|(k, v)| (Value::Text(k.to_owned()), v)).collect())
    }

    fn int(i: i64) -> Value {
        Value::Integer(i.into())
    }

    /// A version 2 save of the first imports: numbered fields, the Europa and the Conference
    /// League swapped, a record linked to a competition without its number.
    #[test]
    fn version_2_names_and_numbers_come_through() {
        let mut world = map(vec![
            ("game", map(vec![("value_iz", int(4)), ("europa_league", int(1)), ("conference_league", int(0)), ("continental_cup_h", int(2))])),
            (
                "competitions",
                Value::Array(vec![
                    map(vec![("class", Value::Text("ConferenceLeague".into())), ("kind", int(6)), ("legacy_id", int(3))]),
                    map(vec![("class", Value::Text("EuropaLeague".into())), ("kind", int(12)), ("legacy_id", int(-1))]),
                    map(vec![("class", Value::Text("ContinentalCupH".into())), ("kind", int(6)), ("legacy_id", int(5))]),
                ]),
            ),
            ("players", Value::Array(vec![map(vec![("competition_stats", Value::Array(vec![map(vec![("competition_id", int(0))])]))])])),
        ]);
        name_national_tournaments(&mut world);
        let game = field(&world, "game").unwrap();
        assert_eq!(field(game, "finalissima_season").and_then(Value::as_integer), Some(4.into()));
        assert_eq!(field(game, "europa_league").and_then(Value::as_integer), Some(0.into()));
        assert_eq!(field(game, "conference_league").and_then(Value::as_integer), Some(1.into()));
        assert_eq!(field(game, "copa_sudamericana").and_then(Value::as_integer), Some(2.into()));
        let classes: Vec<&str> = field(&world, "competitions").unwrap().as_array().unwrap().iter().map(|c| field(c, "class").unwrap().as_text().unwrap()).collect();
        assert_eq!(classes, ["EuropaLeague", "ConferenceLeague", "CopaSudamericana"]);
        let player = &field(&world, "players").unwrap().as_array().unwrap()[0];
        let h = &field(player, "competition_stats").unwrap().as_array().unwrap()[0];
        assert_eq!(field(h, "competition_number").and_then(Value::as_integer), Some(3.into()));
    }
}
