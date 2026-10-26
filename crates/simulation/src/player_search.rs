//! Port of `best.af` (the player search filter of `a.gx`) and the order its table starts in
//! (`b.D` with `b.E`).

use brasfoot_model::ids::PlayerId;
use brasfoot_model::player::attribute;
use brasfoot_model::saved::Player;
use brasfoot_model::world::World;

/// `af.mR`: the seven value bands of the "Valor" combo, inclusive. The last starts at 10 000 000,
/// which the band before already holds; kept as the original has it.
pub const VALUE_BANDS: [(i32, i32); 7] =
    [(1, 100_999), (101_000, 500_999), (501_000, 1_000_000), (1_000_001, 3_000_000), (3_000_001, 5_000_000), (5_000_001, 10_000_000), (10_000_000, 1_000_000_000)];

/// The filter. Every `None` is the original's `-1` (any).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerFilter {
    /// `nome`: a prefix of the name, without accents (`setNome` folds it with `D.f`).
    pub name: String,
    /// `md`: position 0..4.
    pub position: Option<i32>,
    /// `me`: side 0 right, 1 left.
    pub side: Option<i32>,
    /// `mf`, `mm`, `mn`: strength range (classic mode).
    pub strength: Option<(i32, i32)>,
    /// `mg`, `mo`, `mp`
    pub age: Option<(i32, i32)>,
    /// `af.b(name, min, max)`: ranges by `attribute::*` index (individual skills mode).
    pub attributes: [Option<(i32, i32)>; attribute::COUNT],
    /// `mh`: an index of [`VALUE_BANDS`].
    pub value_band: Option<usize>,
    /// `mi`, `mj`
    pub trait1: Option<i32>,
    pub trait2: Option<i32>,
    /// `mk`: the nationality, a country id.
    pub nationality: Option<i32>,
    /// `ml`: an index of the game's league countries (`SR.N()`), where the player's club is.
    pub playing_in: Option<usize>,
    /// `mL` star, `mM` world class, `mN` listed for loan, `mO` listed for sale.
    pub star: bool,
    pub world_class: bool,
    pub for_loan: bool,
    pub for_sale: bool,
}

/// `D.f(String)`: Latin-1 and Latin Extended-A letters to their base letter, per UTF-16 unit.
pub fn fold_accents(text: &str) -> Vec<u16> {
    const TABLE: &str = "AAAAAAACEEEEIIIIDNOOOOO×ØUUUUYIßaaaaaaaceeeeiiiionooooo÷ouuuuyþyAaAaAaCcCcCcCcDdDdEeEeEeEeEeGgGgGgGgHhHhIiIiIiIiIiJjJjKkkLlLlLlLlLlNnNnNnnNnOoOoOoOoRrRrRrSsSsSsSsTtTtTtUuUuUuUuUuUuWwYyYZzZzZzF";
    let table: Vec<u16> = TABLE.encode_utf16().collect();
    text.encode_utf16().map(|c| if (192..=383).contains(&c) { table[(c - 192) as usize] } else { c }).collect()
}

/// `String.equalsIgnoreCase` on UTF-16 units.
fn equals_ignore_case(a: &[u16], b: &[u16]) -> bool {
    let fold = |u: u16| -> (u32, u32) {
        let c = char::from_u32(u as u32).unwrap_or('\u{fffd}');
        let upper = c.to_uppercase().next().unwrap_or(c);
        let lower = upper.to_lowercase().next().unwrap_or(upper);
        (upper as u32, lower as u32)
    };
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| x == y || fold(x).0 == fold(y).0 || fold(x).1 == fold(y).1)
}

impl PlayerFilter {
    /// `af.i(F)`
    pub fn matches(&self, world: &World, p: &Player) -> bool {
        let within = |range: Option<(i32, i32)>, v: i32| range.is_none_or(|(lo, hi)| v >= lo && v <= hi);
        if self.position.is_some_and(|x| p.position != x) || self.side.is_some_and(|x| p.side != x) {
            return false;
        }
        if !within(self.strength, p.strength) || !within(self.age, p.age) {
            return false;
        }
        for (i, range) in self.attributes.iter().enumerate() {
            if !within(*range, p.attributes[i]) {
                return false;
            }
        }
        if !within(self.value_band.map(|b| VALUE_BANDS[b]), p.market_value) {
            return false;
        }
        if self.trait1.is_some_and(|x| p.trait1 != x) || self.trait2.is_some_and(|x| p.trait2 != x) {
            return false;
        }
        if self.nationality.is_some_and(|x| p.country != x) {
            return false;
        }
        // a player without a club, or a country index past the list, passes
        if let (Some(i), Some(club)) = (self.playing_in, p.club)
            && let Some(&country) = world.game.league_countries.get(i)
            && world.club(club).country != world.country(country).country
        {
            return false;
        }
        if (self.star && !p.star) || (self.world_class && !p.world_class) || (self.for_loan && !p.listed_for_loan) || (self.for_sale && !p.listed_for_sale) {
            return false;
        }
        let wanted = fold_accents(&self.name);
        if wanted.is_empty() {
            return true;
        }
        let name = fold_accents(&p.name);
        wanted.len() <= name.len() && equals_ignore_case(&wanted, &name[..wanted.len()])
    }

    /// `af.D(false)`: the game's players (`SR.O()`) that pass, sorted as `b.D` shows them first
    /// (`b.E`: position up, strength down; a stable sort keeps the rest in list order).
    pub fn search(&self, world: &World) -> Vec<PlayerId> {
        let mut found: Vec<PlayerId> = world.game.players.iter().copied().filter(|&id| self.matches(world, world.player(id))).collect();
        found.sort_by(|&a, &b| {
            let (x, y) = (world.player(a), world.player(b));
            x.position.cmp(&y.position).then(y.strength.cmp(&x.strength))
        });
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_accents_like_the_game() {
        assert_eq!(String::from_utf16_lossy(&fold_accents("João Ñúñez Çağrı")), "Joao Nunez Cagri");
    }

    #[test]
    fn compares_ignoring_case() {
        assert!(equals_ignore_case(&fold_accents("jOã"), &fold_accents("Joa")));
        assert!(!equals_ignore_case(&fold_accents("jox"), &fold_accents("Joa")));
    }
}
