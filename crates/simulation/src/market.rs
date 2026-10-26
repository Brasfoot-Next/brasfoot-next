// Port of the game's own market at the start of a season (best.ag as patched by the mod: jO,
// jS, jU, jR, jP, jT, jQ) and of the buyer search of best.l (as patched: the constructor, a,
// aDoJogo, k, cH, cI, cJ, a(ArrayList, boolean), and the squad filler a(ah, int[])), with the
// clubs' interest (ah.a(F, boolean), ah.b(F, boolean)), the squad filler (ah.ku) and the
// transfer itself (F.a(ah, int, boolean, boolean, boolean)).

use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, CountryId, PlayerId};
use brasfoot_model::saved::Transfer;

use crate::Sim;

/// `aq.qg`: the weakest player a league club wants, by division.
const LEAGUE_MINIMUM: [i32; 5] = [1, 40, 30, 20, 5];
/// `aq.qh`: the weakest player another club wants, by reputation.
const CLUB_MINIMUM: [i32; 6] = [1, 10, 20, 40, 50, 55];
/// `aq.qi`: the strongest player a club of that reputation can get.
const CLUB_MAXIMUM: [i32; 6] = [20, 30, 45, 85, 100, 100];

/// `best.l`: who is selling, for how much, and where the buyers are looked for.
struct Negotiation {
    player: PlayerId,
    /// `ci`
    value: i32,
    /// `cj`
    from_offer: bool,
    /// `ck`
    seller: ClubId,
    /// `cl`
    buyer: Option<ClubId>,
    /// `cn`, `co`, `cp`: countries to search, by preference.
    first: Vec<Option<CountryId>>,
    others: Vec<Option<CountryId>>,
    europe: Vec<Option<CountryId>>,
    /// `cs`: 0 a sale, 1 a sale of a good player, 2 a star to a big club.
    mode: i32,
    /// `cx`
    squad_limit: usize,
}

impl Sim<'_> {
    fn country_confederation(&self, c: CountryId) -> i32 {
        countries::confederation(self.world.country(c).country)
    }

    fn club_confederation(&self, c: ClubId) -> i32 {
        countries::confederation(self.world.club(c).country)
    }

    /// `ah.lp()`: the club is a national team.
    fn is_national_team(&self, club: ClubId) -> bool {
        self.world.game.countries.iter().any(|&c| self.world.country(c).national_team == Some(club))
    }

    /// `ah.J(true)` / `ah.J(false)`: the squad by position (the sixth count is the attacking
    /// midfielders with `true`, the holding ones with `false`); `false` leaves out loans.
    fn squad_counts(&self, club: ClubId, with_loans: bool) -> [i32; 6] {
        let mut counts = [0; 6];
        for &p in &self.world.club(club).squad {
            let x = self.world.player(p);
            if with_loans {
                counts[x.position as usize] += 1;
                if x.position == 3 && x.style == 1 {
                    counts[5] += 1;
                }
            } else if !x.on_loan {
                counts[x.position as usize] += 1;
                if x.position == 3 && x.style == 0 {
                    counts[5] += 1;
                }
            }
        }
        counts
    }

    /// `ah.a(F, boolean)`: the club would sign the player.
    fn club_wants(&self, club: ClubId, p: PlayerId, check_squad: bool) -> bool {
        let c = self.world.club(club);
        if c.squad.len() >= 35 {
            return false;
        }
        let strength = self.world.player(p).strength;
        let minimum = if c.league_club { LEAGUE_MINIMUM[c.division as usize] } else { CLUB_MINIMUM[c.reputation as usize] };
        if strength < minimum || strength > CLUB_MAXIMUM[c.reputation as usize] {
            return false;
        }
        if check_squad {
            let counts = self.squad_counts(club, false);
            const MAXIMUM: [i32; 5] = [3, 5, 5, 10, 5];
            let position = self.world.player(p).position as usize;
            if counts[position] > MAXIMUM[position] {
                return false;
            }
        }
        true
    }

    /// `ah.b(F, boolean)`: the player is good enough for the club.
    fn club_accepts(&self, club: ClubId, p: PlayerId) -> bool {
        let c = self.world.club(club);
        if c.squad.len() >= 35 {
            return false;
        }
        let minimum = if c.league_club { LEAGUE_MINIMUM[c.division as usize] } else { CLUB_MINIMUM[c.reputation as usize] };
        self.world.player(p).strength >= minimum
    }

    /// `ag.jO()` and `ag.jQ()`
    pub fn season_market(&mut self) {
        self.runtime.checkpoint("best.ag.jO");
        self.runtime.checkpoint("best.ag.jS");
        self.league_clubs_market();
        self.other_clubs_market();
        for club in self.world.game.clubs.clone() {
            if !self.world.club(club).human {
                self.fill_squad(club);
            }
        }
        self.runtime.checkpoint("best.ag.jP");
        self.national_squads();
        self.runtime.checkpoint("best.ag.jT");
        self.stars_to_big_clubs();
        self.runtime.checkpoint("best.ag.jQ");
        self.coaches_assessed();
    }

    /// `ag.jS()`: each AI league club buys and sells, more the lower it finished.
    fn league_clubs_market(&mut self) {
        for country in self.world.game.league_countries.clone() {
            for league in self.world.country(country).divisions.clone() {
                let stage = self.world.competition(league).league.expect("a league has its table");
                let mut i = 0;
                while i < self.world.stage(stage).clubs.len() {
                    let club = self.world.stage(stage).clubs[i];
                    if !self.world.club(club).human {
                        self.club_market(club, i as i32, true);
                    }
                    i += 1;
                }
            }
        }
    }

    /// `ag.jU()`: the clubs outside the leagues.
    fn other_clubs_market(&mut self) {
        for (i, club) in self.world.game.clubs.clone().into_iter().enumerate() {
            let c = self.world.club(club);
            if !c.league_club && !c.human {
                self.club_market(club, i as i32, false);
            }
        }
    }

    /// `ag.a(ah, int, boolean)`
    pub(crate) fn club_market(&mut self, club: ClubId, place: i32, league: bool) {
        let deals = match place {
            ..=1 => 1,
            ..=5 => 2,
            ..=10 => 3,
            _ => 4,
        };
        for _ in 0..deals {
            self.sell_one(club, league, true);
        }
        for _ in 0..deals {
            self.sell_one(club, league, false);
        }
    }

    /// `ag.a(ah, boolean, boolean)`: the club lets a player of a crowded position go.
    fn sell_one(&mut self, club: ClubId, league: bool, crowded: bool) {
        let counts = self.squad_counts(club, true);
        const FULL: [i32; 6] = [4, 5, 5, 10, 8, 0];
        const ENOUGH: [i32; 6] = [3, 4, 4, 8, 6, 0];
        const SPARE: [i32; 6] = [3, 4, 4, 6, 4, 0];
        let mut positions: Vec<usize> = (0..=4).collect();
        self.runtime.rng.shuffle(&mut positions);
        let chosen = if crowded {
            positions
                .iter()
                .copied()
                .find(|&p| counts[p] >= FULL[p])
                .or_else(|| positions.iter().copied().find(|&p| counts[p] >= ENOUGH[p]))
        } else {
            positions.iter().copied().find(|&p| counts[p] > SPARE[p])
        };
        let allow_stars = self.runtime.rng.fresh_int(100) > 10;
        let mut player = None;
        if let Some(position) = chosen {
            let mut candidates: Vec<PlayerId> = self
                .world
                .club(club)
                .squad
                .iter()
                .copied()
                .filter(|&p| {
                    let x = self.world.player(p);
                    x.position == position as i32 && !x.on_loan && (!x.star || x.star == allow_stars)
                })
                .collect();
            if !candidates.is_empty() {
                self.runtime.rng.shuffle(&mut candidates);
                player = Some(candidates[0]);
            }
        }
        let Some(p) = player else { return };
        let mut search_leagues = true;
        let mut mode = 0;
        let strength = self.world.player(p).strength;
        let star = self.world.player(p).star;
        let rng = &mut self.runtime.rng;
        if league {
            if strength < 40 && rng.fresh_int(100) > 90 {
                search_leagues = false;
            }
        } else {
            search_leagues = false;
            let picked = if star && rng.fresh_int(100) > 30 {
                true
            } else if strength >= 90 && rng.fresh_int(100) > 30 {
                true
            } else if (80..90).contains(&strength) && rng.fresh_int(100) > 35 {
                true
            } else if (70..80).contains(&strength) && rng.fresh_int(100) > 45 {
                true
            } else if (60..70).contains(&strength) && rng.fresh_int(100) > 75 {
                true
            } else {
                strength < 60 && rng.fresh_int(100) > 95
            };
            if picked {
                search_leagues = true;
                mode = 1;
            }
        }
        let value = self.world.player(p).market_value;
        let mut n = self.negotiation(p, value, false, mode);
        if search_leagues {
            self.find_buyer(&mut n);
        } else {
            self.find_buyer_outside_leagues(&mut n);
        }
        self.game_sale(p, n.buyer);
    }

    /// The mod's word on a sale of the game's market, then the transfer.
    fn game_sale(&mut self, p: PlayerId, drawn: Option<ClubId>) {
        let buyer = self.hooks.game_buyer(self.world, self.runtime, p, drawn);
        let value = self.world.player(p).market_value;
        if let Some(b) = buyer
            && self.hooks.approve_game_sale(self.world, self.runtime, p, b, value)
        {
            self.transfer(p, b, value);
        }
    }

    /// `new l(F, int, boolean, boolean, int)`
    fn negotiation(&mut self, p: PlayerId, value: i32, from_offer: bool, mode: i32) -> Negotiation {
        let seller = self.world.player(p).club.expect("a player for sale has a club");
        let confederation = self.club_confederation(seller);
        let mut n = Negotiation {
            player: p,
            value,
            from_offer,
            seller,
            buyer: None,
            first: Vec::new(),
            others: Vec::new(),
            europe: Vec::new(),
            mode,
            squad_limit: if from_offer { 35 } else { 32 },
        };
        let club_country = self.world.club(seller).country;
        let leagues = self.world.game.league_countries.clone();
        let add = |list: &mut Vec<Option<CountryId>>, c: CountryId| {
            if !list.contains(&Some(c)) {
                list.push(Some(c));
            }
        };
        if mode == 0 || mode == 1 {
            if self.world.club(seller).league_club {
                n.first.push(self.league_country(club_country));
            }
            let x = self.world.player(p);
            let mut abroad = club_country == 29 && x.strength > 50 && self.runtime.rng.fresh_int(100) > 10;
            if x.star || x.world_class {
                abroad = true;
            }
            if confederation == 0 || abroad {
                for &c in &leagues {
                    if self.country_confederation(c) == 0 {
                        add(&mut n.first, c);
                    }
                }
            }
            if mode == 1 {
                let pick = |sim: &Self, c: CountryId, set: &[i32]| set.contains(&sim.country_confederation(c));
                match confederation {
                    1 => leagues.iter().filter(|&&c| self.country_confederation(c) != 5).for_each(|&c| add(&mut n.first, c)),
                    2 => leagues.iter().filter(|&&c| pick(self, c, &[0, 3])).for_each(|&c| add(&mut n.first, c)),
                    3 => leagues.iter().filter(|&&c| pick(self, c, &[3, 0])).for_each(|&c| add(&mut n.first, c)),
                    _ => {}
                }
                if club_country != 131 && club_country != 68 {
                    if confederation == 4 {
                        leagues.iter().filter(|&&c| self.country_confederation(c) == 4).for_each(|&c| add(&mut n.first, c));
                    }
                } else {
                    leagues.iter().filter(|&&c| self.country_confederation(c) == 0).for_each(|&c| add(&mut n.first, c));
                }
                if confederation == 5 {
                    leagues.iter().filter(|&&c| pick(self, c, &[5, 3])).for_each(|&c| add(&mut n.first, c));
                }
            }
            for &c in &leagues {
                if self.country_confederation(c) == 0 && !n.first.contains(&Some(c)) {
                    n.others.push(Some(c));
                }
            }
            for &c in &leagues {
                if self.country_confederation(c) == 0 {
                    n.europe.push(Some(c));
                }
            }
        } else if mode == 2 {
            for &c in &leagues {
                if self.country_confederation(c) == 0 {
                    add(&mut n.first, c);
                }
            }
        }
        n
    }

    /// `l.a(boolean, boolean)` (`aDoJogo`)
    fn find_buyer(&mut self, n: &mut Negotiation) {
        let x = self.world.player(n.player).clone();
        let seller_human = self.world.club(n.seller).human;
        let mut usual = true;
        if x.strength > 30 && seller_human && self.runtime.rng.fresh_int(100) > 60 {
            usual = false;
        }
        if x.star && seller_human {
            usual = false;
        }
        if x.world_class {
            usual = false;
        }
        let reputation = self.world.club(n.seller).reputation;
        if usual {
            let first = n.first.clone();
            self.buyer_in_countries(n, &first);
            if n.buyer.is_none() && reputation > 2 {
                let others = n.others.clone();
                self.buyer_in_countries(n, &others);
            }
        } else {
            if n.buyer.is_none() && x.world_class {
                self.big_club_buyer(n);
            }
            if n.buyer.is_none() && x.world_class {
                let europe = n.europe.clone();
                self.buyer_in_countries(n, &europe);
            }
            if n.buyer.is_none() && x.star {
                let europe = n.europe.clone();
                self.buyer_in_countries(n, &europe);
            }
            if n.buyer.is_none() && reputation > 2 {
                let others = n.others.clone();
                self.buyer_in_countries(n, &others);
            }
            if n.buyer.is_none() {
                let first = n.first.clone();
                self.buyer_in_countries(n, &first);
            }
        }
        if n.buyer.is_none() {
            self.find_buyer_outside_leagues(n);
        }
        if n.buyer.is_none() && self.world.season() > 2 && n.from_offer && n.value < x.market_value {
            crate::unported!("l.cJ(): any club for an offer below the value");
        }
    }

    /// `l.cH()` (`l.k(boolean)`): a club outside the leagues.
    fn find_buyer_outside_leagues(&mut self, n: &mut Negotiation) {
        let x = self.world.player(n.player).clone();
        let mut candidates: Vec<ClubId> = self
            .world
            .game
            .clubs
            .iter()
            .copied()
            .filter(|&c| {
                let k = self.world.club(c);
                !k.league_club && Some(c) != x.club && !k.human && k.squad.len() < n.squad_limit && k.id != x.previous_club
            })
            .collect();
        self.runtime.rng.shuffle(&mut candidates);
        let check_squad = !self.world.club(n.seller).human;
        let player_confederation = countries::confederation(x.country);
        let same_country_first = (player_confederation == 1 || player_confederation == 4) && self.runtime.rng.fresh_int(100) > 15;
        let mut found = None;
        if n.mode == 2 {
            found = candidates.iter().copied().find(|&c| {
                let k = self.world.club(c);
                k.squad.len() < 35 && self.club_confederation(c) == 0 && k.reputation >= 4
            });
        }
        if found.is_none() && n.mode != 2 && same_country_first {
            found = candidates.iter().copied().find(|&c| self.world.club(c).country == x.country && self.club_wants(c, n.player, check_squad));
        }
        if found.is_none() && n.mode != 2 {
            let seller_confederation = self.club_confederation(n.seller);
            found = candidates
                .iter()
                .copied()
                .find(|&c| self.club_confederation(c) == seller_confederation && self.club_wants(c, n.player, check_squad));
        }
        if found.is_some() {
            n.buyer = found;
        }
    }

    /// `l.cI()`: a big European club for a world-class player.
    fn big_club_buyer(&mut self, n: &mut Negotiation) {
        let x = self.world.player(n.player).clone();
        let mut candidates: Vec<ClubId> = self
            .world
            .game
            .clubs
            .iter()
            .copied()
            .filter(|&c| {
                let k = self.world.club(c);
                k.reputation >= 4 && Some(c) != x.club && !k.human && k.squad.len() < n.squad_limit && k.id != x.previous_club
            })
            .collect();
        self.runtime.rng.shuffle(&mut candidates);
        let check_squad = !self.world.club(n.seller).human;
        if let Some(c) = candidates.iter().copied().find(|&c| self.club_confederation(c) == 0 && self.club_wants(c, n.player, check_squad)) {
            n.buyer = Some(c);
        }
    }

    /// `l.a(ArrayList, boolean)`: a club of those countries near the seller's division.
    fn buyer_in_countries(&mut self, n: &mut Negotiation, countries: &[Option<CountryId>]) {
        let x = self.world.player(n.player).clone();
        let seller = self.world.club(n.seller);
        let mut low = seller.division - 1;
        let mut high = seller.division + 1;
        if seller.division == 1 {
            low = 1;
        }
        if n.mode == 1 {
            low = 1;
            high = 2;
            if self.club_confederation(n.seller) == 0 && seller.reputation >= 4 && x.strength >= 40 {
                high = 1;
            }
        }
        let mut candidates = Vec::new();
        for &country in countries {
            let country = country.expect("the original fails on a missing league country");
            if x.strength <= 5 {
                low = 0;
                high = self.world.country(country).divisions.len() as i32;
            } else if x.strength <= 20 {
                high = self.world.country(country).divisions.len() as i32;
            }
            if x.strength <= 20 {
                low = 0;
            }
            for &c in &self.world.country(country).clubs_hv {
                let k = self.world.club(c);
                if Some(c) != x.club
                    && k.division <= high
                    && k.id != x.previous_club
                    && k.division >= low
                    && c != n.seller
                    && !k.human
                    && k.squad.len() < n.squad_limit
                {
                    candidates.push(c);
                }
            }
        }
        self.runtime.rng.shuffle(&mut candidates);
        let check_squad = !self.world.club(n.seller).human;
        let found = match n.mode {
            1 => candidates.iter().copied().find(|&c| self.club_accepts(c, n.player)),
            2 => candidates.iter().copied().find(|&c| {
                let k = self.world.club(c);
                k.squad.len() < 35 && k.reputation >= 4
            }),
            _ => candidates.iter().copied().find(|&c| self.club_wants(c, n.player, check_squad)),
        };
        n.buyer = found;
    }

    /// `ah.ku()`: an AI club short of players in a position buys or promotes one.
    fn fill_squad(&mut self, club: ClubId) {
        let counts = self.squad_counts(club, false);
        const MINIMUM: [i32; 6] = [2, 3, 3, 5, 3, 2];
        for position in 0..=4 {
            if counts[position] < MINIMUM[position] {
                let c = self.world.club(club);
                let strength = if c.league_club { LEAGUE_MINIMUM[c.division as usize] } else { CLUB_MINIMUM[c.reputation as usize] };
                self.sign_for_position(club, position as i32, strength);
            }
        }
    }

    /// `l.a(ah, int[])`: a player of the position and about that strength from a similar club,
    /// or a youth player promoted.
    pub(crate) fn sign_for_position(&mut self, club: ClubId, position: i32, strength: i32) {
        let c = self.world.club(club);
        let reputation = c.reputation;
        let mut candidates = Vec::new();
        if c.league_club {
            let low = (reputation - 2).max(0);
            let high = reputation + 1;
            let Some(country) = self.league_country(c.country) else { return self.promote_for_position(club, position) };
            for &k in &self.world.country(country).clubs_hv {
                let x = self.world.club(k);
                if !x.human && k != club && low <= x.reputation && high >= x.reputation {
                    candidates.push(k);
                }
            }
        } else {
            let low = (reputation - 1).max(0);
            let high = reputation + 1;
            let confederation = self.club_confederation(club);
            for &k in &self.world.game.clubs {
                let x = self.world.club(k);
                if !x.human && k != club && self.club_confederation(k) == confederation && low <= x.reputation && high >= x.reputation {
                    candidates.push(k);
                }
            }
        }
        self.runtime.rng.shuffle(&mut candidates);
        match self.pick_player(&candidates, position, strength) {
            Some(p) => {
                let value = self.world.player(p).market_value;
                self.transfer(p, club, value);
            }
            None => self.promote_for_position(club, position),
        }
    }

    fn promote_for_position(&mut self, club: ClubId, position: i32) {
        let p = self.new_youth_player(club, position, None, false);
        self.promote_from_youth_list(p, club);
    }

    /// `l.a(ArrayList, int[])`: the first club with enough players of the position gives one
    /// within five points of the strength.
    fn pick_player(&mut self, clubs: &[ClubId], position: i32, strength: i32) -> Option<PlayerId> {
        let low = (strength - 5).max(5);
        let high = (strength + 5).min(100);
        const ENOUGH: [i32; 6] = [3, 4, 4, 6, 4, 3];
        for &c in clubs {
            let mut squad = self.world.club(c).squad.clone();
            self.runtime.rng.shuffle(&mut squad);
            let mut count = 0;
            let mut found = None;
            for &p in &squad {
                let x = self.world.player(p);
                if x.position == position {
                    count += 1;
                }
                if found.is_none() && x.position == position && x.strength >= low && x.strength <= high && !x.star && !x.world_class {
                    found = Some(p);
                }
            }
            if found.is_some() && count >= ENOUGH[position as usize] {
                return found;
            }
        }
        None
    }

    /// `ag.jT()`: weak clubs' young stars go to the big clubs.
    fn stars_to_big_clubs(&mut self) {
        let mut stars = Vec::new();
        for club in self.world.game.clubs.clone() {
            let c = self.world.club(club);
            if c.human {
                continue;
            }
            let confederation = self.club_confederation(club);
            let weak = (confederation != 0 && c.reputation < 5) || (confederation == 0 && c.reputation < 4);
            if !weak {
                continue;
            }
            for p in self.world.club(club).squad.clone() {
                let x = self.world.player(p);
                if x.strength > 50 && x.age < 31 && x.star && self.runtime.rng.fresh_int(100) > 25 {
                    stars.push(p);
                }
            }
        }
        for p in stars {
            let value = self.world.player(p).market_value;
            let mut n = self.negotiation(p, value, false, 2);
            self.find_buyer(&mut n);
            self.game_sale(p, n.buyer);
            if n.buyer.is_none() {
                let value = self.world.player(p).market_value;
                let mut n = self.negotiation(p, value, false, 2);
                self.find_buyer_outside_leagues(&mut n);
                self.game_sale(p, n.buyer);
            }
        }
    }

    /// `ag.jQ()`: the season's coach changes start over; boards weigh last season's wins.
    fn coaches_assessed(&mut self) {
        self.world.game.coach_changes.clear();
        for coach in self.world.game.coaches.clone() {
            if self.coach_club(coach).is_some() {
                self.world.coach_mut(coach).seasons_at_club += 1;
            }
        }
        let last = self.world.season() - 1;
        for coach in self.world.game.coaches.clone() {
            let c = self.world.coach(coach);
            if self.coach_club(coach).is_none() || c.human || c.seasons_at_club < 1 {
                continue;
            }
            let mut share = 0;
            for j in c.seasons.iter().rev() {
                if j.season == last {
                    share = (j.wins as f32 * 100.0 / j.games as f32) as i32;
                    break;
                }
            }
            if self.runtime.rng.fresh_int(90) + 1 > share {
                let club = self.coach_club(coach).expect("checked above");
                self.replace_coach(club);
            }
        }
    }

    /// `F.a(ah, int, boolean, boolean, boolean)` for a sale: the player moves, the history
    /// keeps the deal.
    pub fn transfer(&mut self, p: PlayerId, to: ClubId, fee: i32) {
        self.move_player(p, to, fee, false, false);
    }

    /// `F.c(ah, int)`: a youth player moves to another club's youth squad for a fee, both
    /// clubs' cash and the transfer history recording it.
    pub fn transfer_youth(&mut self, p: PlayerId, to: ClubId, fee: i32) {
        let from = self.world.player(p).club.expect("a youth player who moves has a club");
        let player = self.world.player_mut(p);
        player.club = Some(to);
        player.shirt_number = 0;
        player.listed_for_loan = false;
        player.listed_for_sale = false;
        self.world.club_income(from, fee, 1);
        self.world.club_payment(to, fee, 1);
        let date = self.runtime.clock.fields(self.world.today_millis());
        let (from_id, to_id) = (self.world.club(from).id, self.world.club(to).id);
        self.world.game.transfers.push(Transfer { player: Some(p), day: date.day, month: date.month + 1, year: date.year, from_club: from_id, to_club: to_id, fee, ..Default::default() });
        self.world.player_mut(p).price = fee;
        let youth = &mut self.world.club_mut(from).youth_squad;
        if let Some(i) = youth.iter().position(|&x| x == p) {
            youth.remove(i);
        }
        self.world.club_mut(to).youth_squad.push(p);
    }

    /// `F.r(ah)` (`F.a(club, 0, false, false, true)`): the loan is over and the player goes back,
    /// for nothing, with the history keeping the move.
    pub fn return_from_loan(&mut self, p: PlayerId, to: ClubId) {
        self.move_player(p, to, 0, false, true);
    }

    /// `F.q(ah)`: the player goes on loan for a year (`new components.t(F, ah)`, 366 days from
    /// today), for nothing, on a contract of 365 days (`F.a(club, 0, false, true, false)`).
    pub fn lend(&mut self, p: PlayerId, to: ClubId) {
        let owner = self.world.player(p).club;
        let until = self.world.today_millis() + 366 * 86_400_000;
        self.world.game.loans.push(brasfoot_model::saved::Loan { until, owner, flag_po: false, player: Some(p) });
        self.move_player(p, to, 0, true, false);
    }

    fn move_player(&mut self, p: PlayerId, to: ClubId, fee: i32, loan: bool, loan_return: bool) {
        let from = self.world.player(p).club.expect("a player who moves has a club");
        let from_id = self.world.club(from).id;
        let player = self.world.player_mut(p);
        player.previous_club = from_id;
        player.club = Some(to);
        if from == to {
            return;
        }
        player.shirt_number = 0;
        player.listed_for_loan = false;
        player.listed_for_sale = false;
        if loan && !loan_return {
            player.on_loan = true;
        }
        if loan_return {
            player.on_loan = false;
        }
        if self.world.club(from).human && self.world.club(to).human {
            crate::unported!("F.a(ah, int, boolean, boolean, boolean) between two human clubs (F.e(true), ah.I(false))");
        }
        if loan || loan_return {
            // no money moves
        } else if self.world.club(from).human {
            self.world.club_mut(from).cash += fee as i64;
            self.world.club_mut(from).finances.income_transfers += fee as i64;
        }
        if !loan && !loan_return && self.world.club(to).human {
            self.world.club_mut(to).cash -= fee as i64;
            self.world.club_mut(to).finances.expense_transfers += fee as i64;
        }
        self.extend_contract(p, if loan { 365 } else { 180 }, true);
        let player = self.world.player_mut(p);
        player.price = player.market_value;
        // ah.ke(), ah.kd(): the old club loses a taker or captain who left
        self.drop_departed_takers(from);
        let date = self.runtime.clock.fields(self.world.today_millis());
        let to_id = self.world.club(to).id;
        self.world.game.transfers.push(Transfer {
            player: Some(p),
            day: date.day,
            month: date.month + 1,
            year: date.year,
            from_club: from_id,
            to_club: to_id,
            fee,
            ..Default::default()
        });
        let squad = &mut self.world.club_mut(from).squad;
        if let Some(i) = squad.iter().position(|&x| x == p) {
            squad.remove(i);
        }
        if let Some(i) = squad.iter().position(|&x| x == p) {
            squad.remove(i);
        }
        self.world.club_mut(to).squad.push(p);
    }

    /// The getters `ah.kd()` and `ah.ke()`: a captain or penalty taker who no longer plays for
    /// the club (outside a national team) is dropped.
    fn drop_departed_takers(&mut self, club: ClubId) {
        let national = self.is_national_team(club);
        let c = self.world.club(club);
        for (taker, is_captain) in [(c.penalty_taker, false), (c.captain, true)] {
            let Some(t) = taker else { continue };
            if self.world.player(t).club != Some(club) {
                let keep = national && self.world.club(club).squad.contains(&t);
                if !keep {
                    let c = self.world.club_mut(club);
                    if is_captain {
                        c.captain = None;
                    } else {
                        c.penalty_taker = None;
                    }
                }
            }
        }
    }
}
