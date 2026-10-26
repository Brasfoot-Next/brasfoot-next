// Port of the creation of players: the name generator (F.au, M.au), the youth player of a club
// (F.a(ah, int, F, int, String, Boolean) with W and F(W, ah)), his strength and attributes
// (F.fI, F.j), development (F.ga), estimated potential (F.fZ), salary (F.fJ), market value
// (F.fK) and contract (F.a(long, boolean)).

use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::player::attribute;
use brasfoot_model::rng::java_round;
use brasfoot_model::saved::Player;

use crate::Sim;
use crate::tables::TOP_LEAGUE_COUNTRIES;

/// `String.length()`: UTF-16 units.
fn java_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `M.q(String)`: the words of a name (`split("\\s+")`).
fn word_count(name: &str) -> usize {
    if name.is_empty() {
        return 0;
    }
    let is_space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r');
    let mut parts: Vec<&str> = name.split(is_space).collect();
    // Java's split joins runs of separators and drops trailing empty strings
    let leading = parts.first().is_some_and(|p| p.is_empty());
    parts.retain(|p| !p.is_empty());
    parts.len() + usize::from(leading && !parts.is_empty())
}

/// `F.at(int)`: the two traits a new player of a position may get.
const TRAITS: [&[[i32; 2]]; 5] = [
    &[[0, 3], [0, 1], [2, 0], [1, 2], [3, 1], [0, 2]],
    &[[6, 10], [6, 13], [10, 11], [10, 13], [10, 6], [10, 9], [6, 11]],
    &[[7, 10], [7, 12], [7, 5], [10, 13], [7, 13], [7, 10], [7, 5], [7, 13], [7, 12], [7, 9], [7, 10], [5, 12]],
    &[
        [4, 11], [4, 9], [9, 11], [11, 9], [4, 8], [4, 13], [7, 10], [7, 11], [7, 5], [7, 13], [10, 13], [10, 11], [9, 4], [10, 12],
        [4, 11], [8, 11], [7, 9], [11, 13], [7, 11],
    ],
    &[[9, 5], [13, 9], [9, 5], [8, 9], [9, 13], [9, 5], [9, 8], [5, 13], [8, 11], [9, 11], [9, 12], [13, 8]],
];

impl Sim<'_> {
    /// `F.at(int)`: a pair of traits that fit the position, drawn with the game's shared
    /// `Random` (`aq.cu`).
    pub fn draw_traits(&mut self, position: i32) -> [i32; 2] {
        let traits = TRAITS[position as usize];
        traits[self.runtime.shared_int(traits.len() as i32) as usize]
    }

    /// `F.au(int)` (`M.au(int)`): a name from the country's lists. None when the game's lists
    /// are not available (the original then tries `F.av`, whose list is always empty).
    pub fn draw_name(&mut self, country: i32) -> Option<String> {
        self.runtime.checkpoint("best.F.au");
        let code = countries::country(country).map(|c| c.code).unwrap_or("");
        let rng = &mut self.runtime.rng;
        let names = self.runtime.names.list(country, code, false).to_vec();
        if names.is_empty() {
            return None;
        }
        let mut i = rng.fresh_int(names.len() as i32);
        if names.len() >= 1000 && rng.fresh_int(2) == 0 {
            i = rng.fresh_int(500);
        }
        if i == 0 {
            i = 1;
        }
        let mut name = names[i as usize].clone();
        match word_count(&name) {
            1 => {
                let surnames = self.runtime.names.list(country, code, true).to_vec();
                if surnames.len() > 2 {
                    let mut j = self.runtime.rng.fresh_int(surnames.len() as i32);
                    if j == 0 {
                        j = 1;
                    }
                    let surname = &surnames[j as usize];
                    if name != *surname {
                        name = format!("{name} {surname}");
                    }
                }
            }
            2 => {
                let extend = self.runtime.rng.fresh_int(2) == 0;
                // the second word comes from the first-name list
                if java_len(&name) <= 12 && extend && names.len() > 2 {
                    let mut j = self.runtime.rng.fresh_int(names.len() as i32);
                    if j == 0 {
                        j = 1;
                    }
                    let other = &names[j as usize];
                    if name != *other && java_len(other) <= 6 {
                        name = format!("{name} {other}");
                    }
                }
            }
            _ => {}
        }
        Some(name)
    }

    /// `F.a(ah, int, F, int, String, Boolean)` with no player given: a youth player of the club.
    /// `pending` (the season's intake) keeps him in `f.bA` until the youth list is updated.
    pub fn new_youth_player(&mut self, club: ClubId, position: i32, name: Option<String>, pending: bool) -> PlayerId {
        self.make_youth_player(club, position, None, name, pending)
    }

    /// `F.a(ah, int, F, int, String, Boolean)`: with `existing`, that player is remade as a new
    /// youth player of the club (the original reuses the object and leaves the lists alone).
    pub fn make_youth_player(&mut self, club: ClubId, position: i32, existing: Option<PlayerId>, name: Option<String>, pending: bool) -> PlayerId {
        self.runtime.checkpoint("best.F.a(Lbest/ah;ILbest/F;ILjava/lang/String;Ljava/lang/Boolean;)Lbest/F;");
        let id = match existing {
            Some(p) => p,
            None => self.create_template_player(club, pending),
        };
        self.fill_youth_player(id, club, position, name)
    }

    /// `new W()`, `new F(W, ah)` and the lists `F.a` adds the new player to.
    fn create_template_player(&mut self, club: ClubId, pending: bool) -> PlayerId {
        // new W(): its side is drawn at once
        let template_side = self.runtime.rng.fresh_int(2);
        // F(W, ah): the template's country 0 and empty name draw a name for country 0
        let mut p = Player { name: self.draw_name(0).unwrap_or_default(), country: 0, club: Some(club), ..Default::default() };
        p.position = 0;
        p.set_side(template_side);
        p.set_star(false);
        p.set_potential(1);
        p.status = 0;
        p.trait1 = 0;
        p.trait2 = 0;
        p.youth = false;
        p.set_age(20);
        p.compute_style();
        let id = PlayerId(self.world.players.len() as u32);
        self.world.players.push(p);
        if pending {
            self.runtime.youth_intake.push(id);
        } else {
            self.world.game.youth_players.push(id);
        }
        self.world.club_mut(club).youth_squad.push(id);
        id
    }

    /// The rest of `F.a(ah, int, F, int, String, Boolean)`: who the youth player is.
    fn fill_youth_player(&mut self, id: PlayerId, club: ClubId, position: i32, name: Option<String>) -> PlayerId {
        let c = self.world.club(club);
        let (level, reputation, club_country) = (c.level, c.reputation, c.country);
        let rng = &mut self.runtime.rng;
        let draw = rng.fresh_int(100) + 1;
        let potential = youth_potential(level, reputation, draw);
        let age = rng.fresh_int(4) + 16;
        let draw2 = rng.fresh_int(100) + 1;
        let mut pos = match draw2 {
            ..=10 => 0,
            ..=30 => 1,
            ..=50 => 2,
            ..=80 => 3,
            _ => 4,
        };
        if position >= 0 {
            pos = position;
        }
        let t = self.draw_traits(pos);
        let mut country = club_country;
        // the second draw decides a foreign youth player, as in the original
        if draw2 == 1 && level >= 18 {
            let pick = self.runtime.rng.fresh_int(6);
            if club_country == 29 {
                country = match pick {
                    0 => 11,
                    1 => 43,
                    2 => 195,
                    3 => 150,
                    4 => 84,
                    _ => self.runtime.rng.fresh_int(200),
                };
            } else if matches!(club_country, 3 | 154 | 85 | 104 | 72) {
                country = match pick {
                    0 => 3,
                    1 => 154,
                    2 => 85,
                    3 => 104,
                    4 => 174,
                    _ => 72,
                };
            }
        }
        let mut potential = potential;
        if country != club_country {
            potential = self.runtime.rng.fresh_int(4) + 7;
        }
        let name = match name {
            Some(n) => n,
            None => self.draw_name(country).unwrap_or_default(),
        };
        let side = self.runtime.rng.fresh_int(2);
        let p = self.world.player_mut(id);
        p.youth = true;
        p.club = Some(club);
        p.potential = potential;
        p.star = draw == 1;
        p.age = age;
        p.position = pos;
        p.trait1 = t[0];
        p.trait2 = t[1];
        p.country = country;
        p.name = name;
        p.side = side;
        p.status = 0;
        p.compute_style();
        self.draw_development(id);
        self.initial_strength(id);
        self.compute_salary(id);
        self.draw_estimated_potential(id);
        id
    }

    /// `F.ga()`: development by age and potential.
    fn draw_development(&mut self, p: PlayerId) {
        self.runtime.checkpoint("best.F.ga");
        let extra = self.runtime.rng.fresh_int(5) + 1;
        let player = self.world.player_mut(p);
        let base = match player.age {
            16 => 15,
            17 => 35,
            18 => 55,
            19 => 70,
            20 => 75,
            _ => 0,
        };
        let mut development = base + extra + player.potential;
        if development < 1 {
            development = 1;
        }
        if development > 100 {
            development = 95;
        }
        player.development = development;
    }

    /// `F.fZ()`: the potential the scouts see.
    fn draw_estimated_potential(&mut self, p: PlayerId) {
        self.runtime.checkpoint("best.F.fZ");
        let draw = self.runtime.rng.fresh_int(100) + 1;
        let player = self.world.player_mut(p);
        let estimate = if draw <= 15 {
            player.potential
        } else if draw <= 60 {
            player.potential - 1
        } else {
            player.potential + 1
        };
        player.estimated_potential = estimate.clamp(1, 10);
    }

    /// `F.fI()`: the strength of a new player from his club's division, level and country.
    pub(crate) fn initial_strength(&mut self, p: PlayerId) {
        self.runtime.checkpoint("best.F.fI");
        let Some(club) = self.world.player(p).club else { return };
        let c = self.world.club(club);
        let (division, level, country) = (c.division, c.level, c.country);
        let (mut strength, attribute_base) = if c.league_club {
            match division {
                1 => (20, 7),
                2 => (15, 3),
                3 => (5, 1),
                _ => (1, 1),
            }
        } else {
            match c.reputation {
                1..=3 => (5, 1),
                4 => (15, 4),
                5 => (22, 7),
                _ => (1, 1),
            }
        };
        let scale = match level {
            ..=15 => level,
            16 => 17,
            17 => 18,
            18 => 19,
            19 => 21,
            20 => 25,
            21 => 26,
            22 => 27,
            23 => 28,
            24 => 29,
            25 => 30,
            _ => level,
        };
        let attribute_level = if scale > 4 { scale - 4 } else { scale };
        strength += scale;
        strength += self.runtime.rng.fresh_int(3);
        let player = self.world.player(p);
        if player.status == 1 {
            strength = strength + 8 + self.runtime.rng.fresh_int(2);
        }
        let player = self.world.player(p);
        if player.star || player.world_class {
            strength = strength + 9 + self.runtime.rng.fresh_int(3);
        }
        if self.world.player(p).youth {
            strength -= 23;
            if strength < 5 {
                strength = 10;
            }
            self.draw_development(p);
            self.draw_estimated_potential(p);
        }
        let country_level = countries::country(country).map(|c| c.level).unwrap_or(0);
        let scaled = |s: i32, f: f64| java_round(s as f64 * f) as i32;
        if country_level <= 13 {
            strength = if level <= 5 {
                scaled(strength, 0.4)
            } else if level < 10 {
                scaled(strength, 0.65)
            } else {
                scaled(strength, 0.75)
            };
        } else if level < 10 {
            strength = if level < 3 {
                scaled(strength, 0.5)
            } else if level < 5 {
                scaled(strength, 0.6)
            } else {
                scaled(strength, 0.7)
            };
        }
        strength = strength.min(100);
        let days = self.runtime.rng.fresh_int(30) + 210;
        self.extend_contract(p, days as i64, true);
        self.world.player_mut(p).strength = strength;
        if self.world.game.individual_skills {
            self.draw_attributes(p, attribute_level, attribute_base);
        }
        self.compute_market_value(p);
        self.compute_salary(p);
    }

    /// `K.r(int, int)`: made-up players of a country short of them (3 keepers, then 4, 4, 5 and 4
    /// of the other positions), without a club (`new F(int)`, in `f.ai`).
    pub(crate) fn made_up_players(&mut self, country: i32, level: i32) {
        self.runtime.checkpoint("best.K.r");
        const COUNT: [i32; 5] = [3, 4, 4, 5, 4];
        for (position, &n) in COUNT.iter().enumerate() {
            for _ in 0..n {
                // new F(int): the country and a name of it
                let name = self.draw_name(country).unwrap_or_default();
                let id = PlayerId(self.world.players.len() as u32);
                self.world.players.push(Player { country, name, ..Default::default() });
                self.world.game.generated_players.push(id);
                self.made_up_player(id, level, position as i32);
            }
        }
    }

    /// `K.a(int, F, int)`: the made-up player's strength by the country's level, a contract of
    /// 180 days, attributes (with the individual skills), traits, potential, age and side.
    fn made_up_player(&mut self, id: PlayerId, level: i32, position: i32) {
        let base = if level <= 15 {
            level
        } else {
            match level {
                16 => 17,
                17 => 18,
                18 => 19,
                19 => 21,
                20 => 25,
                21 => 26,
                22 => 27,
                23 => 28,
                24 => 29,
                25 => 30,
                _ => 0,
            }
        };
        self.world.player_mut(id).position = position;
        let strength = base - 5 + self.runtime.rng.fresh_int(8);
        let season = self.world.season();
        let p = self.world.player_mut(id);
        p.strength = strength;
        p.youth = false;
        p.signed_season = season;
        p.strength_progress = 0.0;
        self.extend_contract(id, 180, true);
        if self.world.game.individual_skills {
            let extra = if level >= 20 {
                7
            } else if level >= 19 {
                4
            } else {
                1
            };
            let attribute_base = if base > 4 { base - 4 } else { base };
            self.draw_attributes(id, attribute_base, extra);
        }
        let t = self.draw_traits(position);
        let potential = self.runtime.rng.fresh_int(4) + 7;
        let age = self.runtime.rng.fresh_int(12) + 18;
        let side = self.runtime.rng.fresh_int(2);
        let p = self.world.player_mut(id);
        p.trait1 = t[0];
        p.trait2 = t[1];
        p.set_potential(potential);
        p.set_age(age);
        p.set_side(side);
        p.status = 0;
        p.compute_style();
        self.compute_market_value(id);
        self.compute_salary(id);
    }

    /// `F.a(long, boolean)`: a contract for so many days from today (or from its end).
    pub fn extend_contract(&mut self, p: PlayerId, days: i64, from_today: bool) {
        let start = if from_today { self.world.today_millis() } else { self.world.player(p).contract_until };
        self.world.player_mut(p).contract_until = start + days * 86_400_000;
        // F.aB(int) and F.fr are transient (the contract screen's state)
    }

    /// `F.j(int, int)`: the attributes of a new player.
    pub fn draw_attributes(&mut self, p: PlayerId, base: i32, extra: i32) {
        self.runtime.checkpoint("best.F.j(II)V");
        use attribute::*;
        let player = self.world.player(p).clone();
        let (strength, t1, t2, style) = (player.strength, player.trait1, player.trait2, player.style);
        let has = |t: i32| t1 == t || t2 == t;
        let third = base / 3;
        let r = &mut self.runtime.rng;
        let mut a = [0i32; 7];
        let round = |x: f64| java_round(x) as i32;
        match player.position {
            0 => {
                a[GOALKEEPING] = strength + r.fresh_int(2);
                a[SPEED] = base + r.fresh_int(7);
                a[TECHNIQUE] = base + r.fresh_int(4);
                a[PASSING] = base + r.fresh_int(4);
                a[DEFENDING] = extra + r.fresh_int(3);
                a[PLAYMAKING] = extra + r.fresh_int(3);
                a[FINISHING] = extra + r.fresh_int(3);
                if t1 == 0 || t1 == 3 {
                    a[TECHNIQUE] = a[TECHNIQUE] + 2 + r.fresh_int(5);
                }
                if t2 == 0 || t2 == 3 {
                    a[TECHNIQUE] += r.fresh_int(2);
                }
                if t1 == 2 {
                    a[SPEED] = a[SPEED] + 2 + r.fresh_int(5);
                }
                if t2 == 2 {
                    a[SPEED] += r.fresh_int(2);
                }
                if t1 == 1 {
                    a[GOALKEEPING] = a[GOALKEEPING] + 1 + r.fresh_int(3);
                }
                if t2 == 1 {
                    a[GOALKEEPING] += r.fresh_int(2);
                }
            }
            1 | 3 => {
                let full_back = player.position == 1;
                if style == 0 {
                    a[DEFENDING] = round(strength as f64 * if full_back { 0.8 } else { 0.7 }) + r.fresh_int(6);
                    a[FINISHING] = if full_back { extra } else { base } + r.fresh_int(4);
                    a[PASSING] = base + r.fresh_int(3);
                    a[TECHNIQUE] = base + r.fresh_int(7);
                    a[PLAYMAKING] = if full_back { extra } else { base } + r.fresh_int(5);
                    a[SPEED] = base + extra + r.fresh_int(6);
                } else {
                    if full_back {
                        a[PLAYMAKING] = round(strength as f64 * 0.5) + r.fresh_int(5);
                        a[FINISHING] = base + extra + r.fresh_int(4);
                        a[PASSING] = base + third + r.fresh_int(3);
                    } else {
                        a[PLAYMAKING] = strength + r.fresh_int(2);
                        a[FINISHING] = base + third + r.fresh_int(4);
                        a[PASSING] = base + extra + r.fresh_int(3);
                    }
                    a[TECHNIQUE] = base + third + r.fresh_int(7);
                    a[DEFENDING] = base + r.fresh_int(4);
                    a[SPEED] = base + if full_back { extra } else { third } + r.fresh_int(4);
                }
                a[GOALKEEPING] = 1 + r.fresh_int(4);
                if has(4) {
                    a[PLAYMAKING] = a[PLAYMAKING] + extra + r.fresh_int(5);
                    a[PASSING] = a[PASSING] + extra + r.fresh_int(5);
                }
                if has(5) {
                    a[FINISHING] = a[FINISHING] + 2 + r.fresh_int(3);
                    a[DEFENDING] = a[DEFENDING] + 2 + r.fresh_int(3);
                }
                if has(6) {
                    a[PASSING] = a[PASSING] + 2 + r.fresh_int(3);
                }
                if has(7) {
                    a[DEFENDING] = a[DEFENDING] + extra + r.fresh_int(3);
                }
                if has(8) {
                    a[TECHNIQUE] = a[TECHNIQUE] + extra + r.fresh_int(3);
                }
                if has(9) {
                    a[FINISHING] = a[FINISHING] + extra + r.fresh_int(3);
                }
                if has(10) {
                    a[DEFENDING] = a[DEFENDING] + if full_back { extra } else { 3 } + r.fresh_int(if full_back { 5 } else { 3 });
                }
                if has(11) {
                    a[PASSING] = a[PASSING] + extra + r.fresh_int(2);
                }
                if has(12) {
                    a[DEFENDING] = a[DEFENDING] + 3 + r.fresh_int(3);
                }
                if has(13) {
                    a[SPEED] = a[SPEED] + base + r.fresh_int(3);
                }
            }
            2 => {
                a[DEFENDING] = round(strength as f64 * 0.9) + r.fresh_int(2);
                a[GOALKEEPING] = 1 + r.fresh_int(7);
                a[SPEED] = base + extra + r.fresh_int(4);
                a[TECHNIQUE] = base + extra + r.fresh_int(7);
                a[PASSING] = base + extra + r.fresh_int(3);
                a[FINISHING] = extra + r.fresh_int(6);
                a[PLAYMAKING] = base + r.fresh_int(5);
                if has(4) {
                    a[PLAYMAKING] = a[PLAYMAKING] + extra + r.fresh_int(5);
                    a[PASSING] = a[PASSING] + extra + r.fresh_int(5);
                }
                if has(5) {
                    a[FINISHING] = a[FINISHING] + extra + r.fresh_int(6);
                }
                if has(6) {
                    a[PASSING] = a[PASSING] + 2 + r.fresh_int(3);
                }
                if has(7) {
                    a[DEFENDING] = a[DEFENDING] + extra + r.fresh_int(3);
                }
                if has(8) {
                    a[TECHNIQUE] = a[TECHNIQUE] + extra + r.fresh_int(3);
                }
                if has(9) {
                    a[FINISHING] = a[FINISHING] + 3 + r.fresh_int(3);
                }
                if has(10) {
                    a[DEFENDING] = a[DEFENDING] + 3 + r.fresh_int(3);
                }
                if has(11) {
                    a[PASSING] = a[PASSING] + extra + r.fresh_int(2);
                }
                if has(12) {
                    a[DEFENDING] = a[DEFENDING] + 3 + r.fresh_int(3);
                    a[FINISHING] += 2;
                }
                if has(13) {
                    a[SPEED] = a[SPEED] + base + r.fresh_int(3);
                }
            }
            4 => {
                a[FINISHING] = round(strength as f64 * 0.8) + r.fresh_int(2);
                a[GOALKEEPING] = 1 + r.fresh_int(6);
                a[SPEED] = base + third + r.fresh_int(4);
                a[TECHNIQUE] = base + third + r.fresh_int(7);
                a[PASSING] = base + extra + r.fresh_int(3);
                a[DEFENDING] = extra + r.fresh_int(6);
                a[PLAYMAKING] = extra + base + r.fresh_int(5);
                if has(4) {
                    a[PLAYMAKING] = a[PLAYMAKING] + base + r.fresh_int(5);
                    a[PASSING] = a[PASSING] + extra + r.fresh_int(5);
                }
                if has(5) {
                    a[FINISHING] = a[FINISHING] + 2 + r.fresh_int(3);
                }
                if has(6) {
                    a[PASSING] = a[PASSING] + 2 + r.fresh_int(3);
                }
                if has(7) {
                    a[DEFENDING] = a[DEFENDING] + extra + r.fresh_int(3);
                }
                if has(8) {
                    a[TECHNIQUE] = a[TECHNIQUE] + extra + r.fresh_int(3);
                }
                if has(9) {
                    a[FINISHING] = a[FINISHING] + 3 + r.fresh_int(3);
                }
                if has(10) {
                    a[DEFENDING] = a[DEFENDING] + 3 + r.fresh_int(3);
                }
                if has(11) {
                    a[PASSING] = a[PASSING] + base + r.fresh_int(2);
                }
                if has(12) {
                    a[DEFENDING] = a[DEFENDING] + 3 + r.fresh_int(3);
                    a[FINISHING] += 2;
                }
                if has(13) {
                    a[SPEED] = a[SPEED] + base + r.fresh_int(3);
                }
            }
            _ => {}
        }
        let player = self.world.player_mut(p);
        for (i, v) in a.into_iter().enumerate() {
            player.attributes[i] = v.min(100);
        }
    }

    /// `F.fJ()`: the salary.
    pub fn compute_salary(&mut self, p: PlayerId) {
        let player = self.world.player(p);
        let mut base = 350;
        if let Some(club) = player.club {
            let c = self.world.club(club);
            let division = c.division;
            base = if TOP_LEAGUE_COUNTRIES.contains(&c.country) {
                match division {
                    1 => 750,
                    2 => 550,
                    3 => 500,
                    4 | 5 => 450,
                    _ => 350,
                }
            } else {
                match division {
                    1 => 600,
                    2 => 500,
                    3 => 450,
                    4 | 5 => 400,
                    _ => 350,
                }
            };
            if c.level > 20 {
                base += 50;
            }
        }
        base -= match player.position {
            0 => 70,
            1 => 30,
            2 => 40,
            4 => 50,
            _ => 0,
        };
        let base = java_round(0.5 * base as f64) as i32;
        let by_strength = player.strength * 2 * base;
        let by_age = (player.age - 32) * 300;
        let fame = if player.star || player.world_class { player.strength * 250 } else { 0 };
        let mut salary = if player.age < 32 { by_strength + fame } else { by_strength - by_age + fame };
        if salary < 500 {
            salary = 500;
        }
        if player.world_class {
            salary = java_round(salary as f64 * 1.4) as i32;
        }
        if player.youth {
            salary = java_round(salary as f64 * 0.1) as i32;
        }
        let monthly = self.world.game.monthly_salary;
        self.world.player_mut(p).salary = if monthly { salary * 4 } else { salary };
    }

    /// `F.fK()`: the market value.
    pub fn compute_market_value(&mut self, p: PlayerId) {
        let player = self.world.player(p);
        let confederation = countries::confederation(player.country);
        let level = player.club.map(|c| self.world.club(c).level).unwrap_or(10);
        let mut squared = player.strength * 2;
        squared *= squared;
        let mut factor = match level {
            21.. => 750,
            20 => 600,
            18..=19 => 500,
            12..=17 => 400,
            _ => 366,
        };
        if player.star {
            factor = if level >= 22 && confederation == 0 {
                factor * 3
            } else if level >= 21 && confederation == 0 {
                factor * 2
            } else {
                java_round(1.7 * factor as f64) as i32
            };
        }
        if player.world_class {
            factor = java_round(1.6 * factor as f64) as i32;
        }
        if player.position == 4 {
            factor = java_round(1.3 * factor as f64) as i32;
        }
        if player.status == 1 {
            factor = java_round(factor as f64 + 0.2 * factor as f64) as i32;
        }
        let season = self.world.season();
        let player = self.world.player_mut(p);
        if player.age < 16 {
            player.age = 16;
        }
        let age = player.age;
        let by_age = if age < 20 {
            (32 - age) * 27
        } else if age <= 25 {
            (32 - age) * 22
        } else if age < 32 {
            (32 - age) * 15
        } else if age < 34 {
            (34 - age) * 10
        } else {
            -(age - 34) * 50
        };
        factor += by_age;
        if factor <= 0 {
            factor = 60;
        }
        let mut value = squared.wrapping_mul(factor);
        let signed = player.signed_season;
        if player.youth {
            value = (java_round(value as f64 * 0.03) as i32).wrapping_mul(player.potential);
        } else if signed > 0 && season == signed {
            value = java_round(value as f64 * 0.18) as i32;
            if value > player.price && player.price > 0 {
                value = player.price;
            }
        } else if signed > 0 && season - 1 == signed {
            value = java_round(value as f64 * 0.35) as i32;
        } else if signed > 0 && season - 2 == signed {
            value = java_round(value as f64 * 0.65) as i32;
        }
        if player.youth && player.potential == 10 {
            value = java_round(value as f64 * 1.5) as i32;
        }
        player.market_value = value;
    }
}

/// The potential `F.a(ah, int, F, int, String, Boolean)` gives a youth player of a club of
/// `level` and `reputation` for a `draw` of 1 to 100.
pub fn youth_potential(level: i32, reputation: i32, draw: i32) -> i32 {
    if level < 19 && reputation <= 3 {
        if level >= 15 {
            match draw {
                ..=2 => 1,
                ..=5 => 2,
                ..=10 => 4,
                ..=30 => 5,
                ..=65 => 6,
                ..=90 => 7,
                ..=95 => 8,
                ..=98 => 9,
                _ => 10,
            }
        } else {
            match draw {
                ..=4 => 1,
                ..=8 => 2,
                ..=15 => 3,
                ..=25 => 4,
                ..=50 => 5,
                ..=75 => 6,
                ..=95 => 7,
                ..=98 => 8,
                ..=99 => 9,
                _ => 10,
            }
        }
    } else {
        match draw {
            ..=2 => 1,
            ..=5 => 2,
            ..=10 => 4,
            ..=25 => 5,
            ..=60 => 6,
            ..=80 => 7,
            ..=90 => 8,
            ..=98 => 9,
            _ => 10,
        }
    }
}
