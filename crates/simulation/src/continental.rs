// Port of the international club cups of the game: the continental cups (f.v, f.r, f.t, f.u,
// f.w, f.y and their second tiers) and the world club cup of their champions (f.A).

use brasfoot_model::ids::{ClubId, CompetitionId, CountryId};
use brasfoot_model::saved::CountrySlots;

use crate::Sim;
use crate::sorting::{by_level, by_level_and_reputation};
use crate::stages::StageOptions;

/// `components.aq`: a draw's pot of ties, the clubs of one tie kept from different countries
/// where it can.
#[derive(Clone, Debug, Default)]
pub(crate) struct Tie(pub(crate) Vec<ClubId>);

impl Tie {
    /// `em(int)`
    fn has_country(&self, sim: &Sim, country: i32) -> bool {
        self.0.iter().any(|&c| sim.world.club(c).country == country)
    }

    /// `ab(ArrayList)`: the first club of the list from a country not in the tie yet.
    fn first_new_country(&self, sim: &Sim, list: &[ClubId]) -> usize {
        list.iter().position(|&c| !self.has_country(sim, sim.world.club(c).country)).unwrap_or(0)
    }

    /// `a(ArrayList, int)`: the club at that place joins the tie, leaving the list.
    fn take(&mut self, list: &mut Vec<ClubId>, i: usize) {
        self.0.push(list.remove(i));
    }

    /// `e(ArrayList, ArrayList)`: the first of one pot and the first of the other from a new country.
    pub(crate) fn two(&mut self, sim: &Sim, a: &mut Vec<ClubId>, b: &mut Vec<ClubId>) {
        self.take(a, 0);
        let i = self.first_new_country(sim, b);
        self.take(b, i);
    }

    /// `f(ArrayList, ArrayList)`: the first of each.
    pub(crate) fn two_plain(&mut self, a: &mut Vec<ClubId>, b: &mut Vec<ClubId>) {
        self.take(a, 0);
        self.take(b, 0);
    }

    /// `a(ArrayList, ArrayList, ArrayList, ArrayList)`: a group of four, one of each pot.
    pub(crate) fn four(&mut self, sim: &Sim, pots: [&mut Vec<ClubId>; 4]) {
        let [a, b, c, d] = pots;
        self.take(a, 0);
        for pot in [b, c, d] {
            let i = self.first_new_country(sim, pot);
            self.take(pot, i);
        }
    }
}

/// `aq.a(aq[])`: in each tie with two clubs of one country, the first such club swaps places
/// with the club at the same place of the first other tie where neither country repeats.
pub(crate) fn separate_countries(sim: &Sim, ties: &mut [Tie]) {
    let mut found = Vec::new();
    for (t, tie) in ties.iter().enumerate() {
        for (i, &a) in tie.0.iter().enumerate() {
            let country = sim.world.club(a).country;
            if tie.0.iter().any(|&b| b != a && sim.world.club(b).country == country) {
                found.push((t, i));
                break;
            }
        }
    }
    for (t, i) in found {
        let country = sim.world.club(ties[t].0[i]).country;
        for u in 0..ties.len() {
            if u != t && !ties[u].has_country(sim, country) && !ties[t].has_country(sim, sim.world.club(ties[u].0[i]).country) {
                let (x, y) = (ties[t].0[i], ties[u].0[i]);
                ties[t].0[i] = y;
                ties[u].0[i] = x;
                break;
            }
        }
    }
}

/// `ac.valueOf("P" + id)` as the list of the game's countries (`c.a.SR.aG()`) holds them.
fn country_of(sim: &Sim, id: i32) -> CountryId {
    *sim.world.game.countries.iter().find(|&&c| sim.world.country(c).country == id).expect("every country of the game is listed")
}

impl Sim<'_> {
    /// `at.cS()`: the competition's champion of this season.
    pub(crate) fn champion_now(&self, c: CompetitionId) -> Option<ClubId> {
        let season = self.world.season();
        let t = self.world.competition(c).titles.iter().find(|t| t.season == season)?;
        if t.champion >= 0 { self.world.club_by_game_id(t.champion) } else { None }
    }

    /// `f.r.yx()`: the Libertadores' best club from outside Mexico, from the knockout's last
    /// round back.
    fn libertadores_best_not_mexican(&self, libertadores: CompetitionId) -> Option<ClubId> {
        let main = self.world.competition(libertadores).main_stage?;
        let knockout = self.world.stage(main).knockout?;
        for r in self.world.stage(knockout).rounds.iter().rev() {
            if let Some(&w) = r.winners.iter().find(|&&w| self.world.club(w).country != 131) {
                return Some(w);
            }
        }
        None
    }

    /// `f.A.zC()` (the day task "cw"): the world club cup of the season's continental champions:
    /// the four of Africa, Asia, Oceania and Concacaf in one of three orders play the quarter
    /// finals; the champions of Europe and South America wait in the semi-finals (`al`).
    pub(crate) fn intercontinental_cup_draw(&mut self) {
        let Some(cup) = self.intercontinental_cup() else { return };
        let g = &self.world.game;
        let (europe, south_america, africa, asia, oceania, concacaf) =
            (g.champions_league, g.libertadores, g.caf_champions_league, g.afc_champions_league, g.ofc_champions_league, g.concacaf_champions_cup);
        let c = self.world.competition_mut(cup);
        c.quarter_finals = None;
        c.semi_finals = None;
        c.europe_champion = None;
        c.south_america_champion = None;
        c.africa_champion = None;
        c.asia_champion = None;
        c.oceania_champion = None;
        c.concacaf_champion = None;
        // each continental cup keeps its champion (at.N(ah))
        let take = |sim: &mut Self, comp: Option<CompetitionId>| -> Option<ClubId> {
            let comp = comp?;
            let champion = sim.champion_now(comp);
            sim.keep_title_holder(comp, champion);
            champion
        };
        let zo = take(self, europe);
        let mut zp = take(self, south_america);
        if zp.is_some_and(|club| self.world.club(club).country == 131) {
            zp = south_america.and_then(|l| self.libertadores_best_not_mexican(l));
        }
        let zq = take(self, africa);
        let zr = take(self, asia);
        let zs = take(self, oceania);
        let zt = take(self, concacaf);
        let c = self.world.competition_mut(cup);
        c.europe_champion = zo;
        c.south_america_champion = zp;
        c.africa_champion = zq;
        c.asia_champion = zr;
        c.oceania_champion = zs;
        c.concacaf_champion = zt;
        let (Some(_), Some(_), Some(q), Some(r), Some(s), Some(t)) = (zo, zp, zq, zr, zs, zt) else { return };
        let four = [q, r, s, t];
        const ORDERS: [[usize; 4]; 3] = [[2, 0, 3, 1], [3, 2, 1, 0], [0, 2, 3, 1]];
        let order = ORDERS[self.runtime.rng.fresh_int(3) as usize];
        let clubs: Vec<ClubId> = order.iter().map(|&i| four[i]).collect();
        let kind = self.world.competition(cup).kind;
        let stage = self.new_knockout_stage(cup, None, clubs.len() as i32, kind, 1, vec![false; 7], -1);
        // z.fc(150)
        self.world.stage_mut(stage).phase_code = 150;
        self.world.competition_mut(cup).quarter_finals = Some(stage);
        self.new_knockout_round(stage, &clubs, 0, false, 0, kind, false);
    }

    /// `f.A.al(ArrayList)` (the end of the quarter finals, 150): the champion of South America
    /// against the first winner, the champion of Europe against the second.
    pub(crate) fn intercontinental_semi_finals(&mut self, cup: CompetitionId, winners: &[ClubId]) {
        if winners.len() != 2 {
            return;
        }
        let c = self.world.competition(cup);
        let (Some(south_america), Some(europe)) = (c.south_america_champion, c.europe_champion) else { return };
        let clubs = [south_america, winners[0], europe, winners[1]];
        let stage = self.cup_knockout(cup, &clubs, 0, vec![false; 7], -1);
        self.world.competition_mut(cup).semi_finals = Some(stage);
    }

    /// `f.aR()`: the world club cup, made on first use when the game plays the international
    /// cups (`new f.A()`: kind 5, subkind 7, a third-place match; its name, "mundialclubes",
    /// is the screens').
    pub fn intercontinental_cup(&mut self) -> Option<CompetitionId> {
        if self.world.game.intercontinental_cup.is_none() && self.world.game.plays_international_clubs {
            let cup = brasfoot_model::saved::Competition {
                class: brasfoot_model::saved::CompetitionKind::IntercontinentalCup,
                kind: 5,
                subkind: 7,
                third_place_match: true,
                ..Default::default()
            };
            let id = CompetitionId(self.world.competitions.len() as u32);
            self.world.competitions.push(cup);
            self.world.game.intercontinental_cup = Some(id);
        }
        self.world.game.intercontinental_cup
    }

    /// `at.N(ah)` of the continental cups: `YT` (f.r, f.v) or `ZA` (the others).
    fn keep_title_holder(&mut self, comp: CompetitionId, club: Option<ClubId>) {
        use brasfoot_model::saved::CompetitionKind::*;
        let c = self.world.competition_mut(comp);
        match c.class {
            ChampionsLeague | Libertadores => c.club_yt = club,
            _ => c.title_holder = club,
        }
    }

    /// `f.ai()`: the champions of the second tiers get their places for the new season: the
    /// Europa League's in the Champions League (`T`), the Sudamericana's in the Libertadores
    /// (`O`), the Conference League's in the Europa League (`ab`); each keeps its title holder.
    pub(crate) fn continental_champions_places(&mut self) {
        let g = &self.world.game;
        let (europa, champions, sudamericana, libertadores, conference) = (g.europa_league, g.champions_league, g.copa_sudamericana, g.libertadores, g.conference_league);
        if let (Some(el), Some(cl)) = (europa, champions) {
            let champion = self.champion_now(el);
            self.world.competition_mut(cl).club_zc = champion;
            self.world.competition_mut(el).title_holder = champion;
        }
        if let (Some(lib), Some(sud)) = (libertadores, sudamericana) {
            let champion = self.champion_now(sud);
            self.world.competition_mut(lib).club_yu = champion;
            self.world.competition_mut(sud).title_holder = champion;
        }
        if let (Some(el), Some(conf)) = (europa, conference) {
            let champion = self.champion_now(conf);
            self.world.competition_mut(el).club_ago = champion;
        }
    }

    /// `f.jw()`: the Finalíssima's two clubs, the champions of the Euro and of the Copa
    /// América, when there are.
    pub(crate) fn finalissima_places(&mut self) {
        let g = &self.world.game;
        let (Some(fin), euro, copa) = (g.finalissima, g.european_championship, g.copa_america) else { return };
        if let Some(champion) = euro.and_then(|c| self.champion_now(c)) {
            self.world.competition_mut(fin).club_agg = Some(champion);
        }
        if let Some(champion) = copa.and_then(|c| self.champion_now(c)) {
            self.world.competition_mut(fin).club_agh = Some(champion);
        }
    }

    /// `Z.jd()`: the country's clubs in the order its continental places go: its first two
    /// divisions' final order (`f.s.yV()`), or, without leagues, a random pick (`Z.je()`).
    fn country_ranking(&mut self, country: CountryId) -> Vec<ClubId> {
        if let Some(r) = self.runtime.country_rankings.get(&country)
            && !r.is_empty()
        {
            return r.clone();
        }
        let divisions = self.world.country(country).divisions.clone();
        let ranking = if divisions.is_empty() {
            self.random_country_ranking(country)
        } else {
            let mut r = self.league_order(divisions[0]);
            if divisions.len() > 1 {
                r.extend(self.league_order(divisions[1]));
            }
            r
        };
        self.runtime.country_rankings.insert(country, ranking.clone());
        ranking
    }

    /// `Z.je()`: the clubs of a country without leagues, each with a chance by its place by
    /// level (85% the first, down to 35%, then 20%), the rest after.
    fn random_country_ranking(&mut self, country: CountryId) -> Vec<ClubId> {
        const CHANCES: [i32; 11] = [85, 80, 75, 70, 65, 60, 55, 50, 45, 40, 35];
        let mut clubs = self.world.country(country).clubs_hv.clone();
        for &c in &clubs {
            let n = self.runtime.rng.fresh_int(1000) + 1;
            self.world.club_mut(c).tiebreak_number = n;
        }
        clubs.sort_by(|&a, &b| by_level(self.world, a, b));
        let mut out = Vec::new();
        for (i, &c) in clubs.iter().enumerate() {
            let chance = CHANCES.get(i).copied().unwrap_or(20);
            if self.runtime.rng.fresh_int(100) + 1 <= chance {
                out.push(c);
            }
        }
        for &c in &clubs {
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }

    /// `Z.bk(int)`: the country's cup final of a season: champion and runner-up.
    fn cup_final(&self, country: CountryId, season: i32) -> Option<(Option<ClubId>, Option<ClubId>)> {
        let cup = self.world.country(country).cup?;
        let t = self.world.competition(cup).titles.iter().find(|t| t.season == season)?;
        let club = |id: i32| if id >= 0 { self.world.club_by_game_id(id) } else { None };
        Some((club(t.champion), club(t.runner_up)))
    }

    /// `Z.a(int, ArrayList, ArrayList, boolean)`: `count` of the country's clubs not taken yet,
    /// the last cup's champion first when `cup_champion`.
    fn country_picks(&mut self, country: CountryId, mut count: i32, out: &mut Vec<ClubId>, taken: &mut Vec<ClubId>, cup_champion: bool) {
        let ranking = self.country_ranking(country);
        if cup_champion
            && let Some((Some(champion), _)) = self.cup_final(country, self.world.season() - 1)
            && !taken.contains(&champion)
        {
            out.push(champion);
            taken.push(champion);
            count -= 1;
        }
        if count > 0 {
            for c in ranking {
                if count > 0 && !taken.contains(&c) {
                    out.push(c);
                    taken.push(c);
                    count -= 1;
                }
                if count == 0 {
                    break;
                }
            }
        }
    }

    /// `f.b.a(ArrayList, ArrayList, ArrayList, boolean)`: each country of the slots gives its
    /// clubs (a slot's flag puts the cup champion first, and so for the rest of the list).
    fn slot_picks(&mut self, slots: &[CountrySlots], out: &mut Vec<ClubId>, taken: &mut Vec<ClubId>, cup_champion: bool) {
        let mut cup_champion = cup_champion;
        for s in slots {
            let country = country_of(self, s.country);
            if s.flag_ry {
                cup_champion = true;
            }
            self.country_picks(country, s.slots, out, taken, cup_champion);
        }
    }

    /// `f.b.a(ArrayList, ArrayList, int, ArrayList)`: short of `target`, one more club of each
    /// country of the slots in turn.
    fn fill_from_slots(&mut self, out: &mut Vec<ClubId>, slots: &[CountrySlots], target: usize, taken: &mut Vec<ClubId>) {
        let mut i = 0;
        while i < slots.len() && out.len() < target {
            let one = [CountrySlots { country: slots[i].country, slots: 1, flag_ry: false }];
            self.slot_picks(&one, out, taken, false);
            i += 1;
        }
    }

    /// `f.ae()`: the continental cups of the new season: every country's ranking starts over
    /// (`Z.jy()`), each cup draws its clubs (`yq()`), the cups' scorers go (`mw()`), the
    /// national teams' continental cups start over (`xZ()`), the world club cup's draw goes on
    /// the last day of kind 4 ("cw") and the two super cups pair their title holders (`zR()`).
    pub(crate) fn continental_cups_new_season(&mut self) {
        self.runtime.country_rankings.clear();
        let g = &self.world.game;
        let draws = [
            g.concacaf_champions_cup,
            g.libertadores,
            g.champions_league,
            g.caf_champions_league,
            g.afc_champions_league,
            g.ofc_champions_league,
            g.copa_sudamericana,
            g.europa_league,
            g.conference_league,
        ];
        for cup in draws.into_iter().flatten() {
            self.continental_draw(cup);
        }
        let g = &self.world.game;
        let mut scorers = vec![
            g.copa_sudamericana,
            g.libertadores,
            g.champions_league,
            g.caf_champions_league,
            g.afc_champions_league,
            g.ofc_champions_league,
            g.concacaf_champions_cup,
            g.europa_league,
            g.conference_league,
        ];
        scorers.push(self.intercontinental_cup());
        let g = &self.world.game;
        scorers.extend([g.super_cup_d, g.super_cup_e, g.world_cup, g.european_championship, g.copa_america]);
        for cup in scorers.into_iter().flatten() {
            self.world.competition_mut(cup).scorers.clear();
        }
        let g = &self.world.game;
        for cup in [g.africa_cup_of_nations, g.asian_cup, g.gold_cup, g.oceania_nations_cup].into_iter().flatten() {
            // xZ() of f.c, f.e, f.h, f.g, then mw()
            if let Some(main) = self.world.competition(cup).main_stage {
                self.restart_stage(main, true);
            }
            let c = self.world.competition_mut(cup);
            c.main_stage = None;
            if c.class == brasfoot_model::saved::CompetitionKind::GoldCup {
                c.final_round = None;
            }
            c.countries.clear();
            c.clubs.clear();
            c.scorers.clear();
        }
        // a.d(4), a.b(day, 1, 0)
        if let Some(day) = self.world.game.days.iter().rposition(|d| d.kind == 4)
            && day > 0
        {
            self.add_task(day, 1, false);
        }
        let e = self.super_cup(brasfoot_model::saved::CompetitionKind::SuperCupE);
        self.continental_super_cup_draw(e);
        let d = self.super_cup(brasfoot_model::saved::CompetitionKind::SuperCupD);
        self.continental_super_cup_draw(d);
        for cup in [e, d] {
            self.world.competition_mut(cup).scorers.clear();
        }
    }

    /// `c.a.SR.aV()`, `aW()`: the Recopa Sudamericana (f.E: kind 8, subkind 1) and the European
    /// Super Cup (f.D: kind 8, subkind 0), made on first use (their names, "recopaS" and
    /// "recopaE", are the screens').
    fn super_cup(&mut self, class: brasfoot_model::saved::CompetitionKind) -> CompetitionId {
        use brasfoot_model::saved::CompetitionKind::*;
        let existing = if class == SuperCupE { self.world.game.super_cup_e } else { self.world.game.super_cup_d };
        if let Some(c) = existing {
            return c;
        }
        let cup = brasfoot_model::saved::Competition { class, kind: 8, subkind: if class == SuperCupE { 1 } else { 0 }, ..Default::default() };
        let id = CompetitionId(self.world.competitions.len() as u32);
        self.world.competitions.push(cup);
        // c.a.SR.a(f.E), a(f.D)
        if class == SuperCupE {
            self.world.game.super_cup_e = Some(id);
        } else {
            self.world.game.super_cup_d = Some(id);
        }
        id
    }

    /// `zR()` of f.E and f.D: the title holders of the cup and its second tier, when both are
    /// there and differ, in a two-legged final, the second tier's at home first.
    fn continental_super_cup_draw(&mut self, cup: CompetitionId) {
        use brasfoot_model::saved::CompetitionKind::*;
        let c = self.world.competition_mut(cup);
        c.participants.clear();
        c.final_stage = None;
        let class = c.class;
        let g = &self.world.game;
        let (first, second) = if class == SuperCupE { (g.libertadores, g.copa_sudamericana) } else { (g.champions_league, g.europa_league) };
        let holder = |sim: &Self, comp: Option<CompetitionId>| -> Option<ClubId> {
            let c = sim.world.competition(comp?);
            match c.class {
                ChampionsLeague | Libertadores => c.club_yt,
                _ => c.title_holder,
            }
        };
        let (Some(a), Some(b)) = (holder(self, first), holder(self, second)) else { return };
        if a == b {
            return;
        }
        let clubs = vec![b, a];
        self.world.competition_mut(cup).participants = clubs.clone();
        let stage = self.new_knockout_stage(cup, None, 2, 8, 0, vec![true; 7], -1);
        self.world.competition_mut(cup).final_stage = Some(stage);
        self.new_knockout_round(stage, &clubs, 0, true, 0, 8, false);
    }

    /// `yq()` of each continental cup.
    fn continental_draw(&mut self, cup: CompetitionId) {
        use brasfoot_model::saved::CompetitionKind::*;
        let class = self.world.competition(cup).class;
        let label = match class {
            ConcacafChampionsCup => "w",
            Libertadores => "r",
            ChampionsLeague => "v",
            CafChampionsLeague => "t",
            AfcChampionsLeague => "u",
            OfcChampionsLeague => "y",
            CopaSudamericana => "H",
            EuropaLeague => "x",
            _ => "J",
        };
        self.runtime.checkpoint(&format!("f.{label}.yq"));
        match class {
            ConcacafChampionsCup => self.concacaf_champions_cup_draw(cup),
            Libertadores => self.libertadores_draw(cup),
            ChampionsLeague => self.champions_league_draw(cup),
            CafChampionsLeague | AfcChampionsLeague | OfcChampionsLeague => self.groups_cup_draw(cup),
            CopaSudamericana => self.sudamericana_draw(cup),
            EuropaLeague => self.europa_league_draw(cup),
            ConferenceLeague => self.conference_league_draw(cup),
            other => unreachable!("{other:?} is not a continental cup"),
        }
    }

    /// `yq()` of f.t, f.u and f.y: the title holder (but in Oceania) and the countries' places,
    /// to 32, 40 or 8 with one more of each country; the groups by level in the cup's order.
    fn groups_cup_draw(&mut self, cup: CompetitionId) {
        use brasfoot_model::saved::CompetitionKind::*;
        /// f.t: 8 groups of 4, two go on
        const CAF: [usize; 32] = [0, 15, 23, 31, 3, 14, 22, 29, 5, 13, 21, 27, 7, 12, 20, 25, 1, 11, 19, 30, 2, 10, 18, 28, 4, 9, 17, 26, 6, 8, 16, 24];
        /// f.u: 10 groups of 4, the winners and the best thirds go on (1403)
        const AFC: [usize; 40] = [0, 10, 20, 30, 1, 11, 21, 31, 2, 12, 22, 32, 3, 13, 23, 33, 4, 14, 24, 34, 5, 15, 25, 35, 6, 16, 26, 36, 7, 17, 27, 37, 8, 18, 28, 38, 9, 19, 29, 39];
        /// f.y: 2 groups of 4, two go on
        const OFC: [usize; 8] = [0, 2, 4, 6, 1, 3, 5, 7];
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.main_stage = None;
        let class = c.class;
        let holder = if class == OfcChampionsLeague { None } else { c.title_holder };
        let slots = c.country_slots.clone();
        let mut clubs: Vec<ClubId> = holder.into_iter().collect();
        let mut taken = clubs.clone();
        self.slot_picks(&slots, &mut clubs, &mut taken, false);
        let (order, options): (&[usize], StageOptions) = match class {
            CafChampionsLeague => (&CAF, StageOptions { teams: 32, groups: 8, knockout_teams: 2, two_rounds: true, skip_two_dates: true, ..Default::default() }),
            AfcChampionsLeague => (
                &AFC,
                StageOptions { teams: 40, groups: 10, knockout_teams: 1, best_third_places: true, two_rounds: true, skip_two_dates: true, format_code: 1403, ..Default::default() },
            ),
            _ => (&OFC, StageOptions { teams: 8, groups: 2, knockout_teams: 2, two_rounds: true, skip_two_dates: true, ..Default::default() }),
        };
        if clubs.len() < order.len() {
            self.fill_from_slots(&mut clubs, &slots, order.len(), &mut taken);
        }
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        self.world.competition_mut(cup).clubs = clubs.clone();
        if clubs.len() >= order.len() {
            let drawn: Vec<ClubId> = order.iter().map(|&i| clubs[i]).collect();
            let stage = self.new_league_stage(&options, &drawn, 0, 4, cup, true);
            self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
            self.world.competition_mut(cup).main_stage = Some(stage);
        }
        // else the game prints "erro Liga ..." and plays none
    }

    /// `f.H.yq()`: the Sudamericana: twelve clubs for the groups (`YF`) and 32 for the first
    /// round (`YS`), none of the Libertadores'; the first round pairs them four by four (6100).
    fn sudamericana_draw(&mut self, cup: CompetitionId) {
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.main_stage = None;
        c.final_round = None;
        c.clubs.clear();
        c.clubs_ys.clear();
        c.clubs_aao.clear();
        let (slots, qualifier_slots) = (c.country_slots.clone(), c.country_slots_yw.clone());
        let mut taken = Vec::new();
        if let Some(lib) = self.world.game.libertadores {
            let l = self.world.competition(lib);
            taken.extend_from_slice(&l.clubs);
            taken.extend_from_slice(&l.clubs_ys);
        }
        let mut groups = Vec::new();
        let mut first_round = Vec::new();
        self.slot_picks(&slots, &mut groups, &mut taken, false);
        self.slot_picks(&qualifier_slots, &mut first_round, &mut taken, false);
        for _ in 0..2 {
            if groups.len() < 12 {
                self.fill_from_slots(&mut groups, &slots, 12, &mut taken);
            }
        }
        if first_round.len() < 32 {
            self.fill_from_slots(&mut first_round, &qualifier_slots, 32, &mut taken);
        }
        if first_round.len() < 32 {
            self.fill_from_slots(&mut first_round, &slots, 32, &mut taken);
        }
        groups.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let c = self.world.competition_mut(cup);
        c.clubs = groups.clone();
        c.clubs_ys = first_round.clone();
        let mut drawn = Vec::new();
        if first_round.len() == 32 {
            // f.H.b(ArrayList, int, int): four by four, each shuffled
            for i in 0..8 {
                let mut four = first_round[i * 4..i * 4 + 4].to_vec();
                self.runtime.rng.shuffle(&mut four);
                drawn.extend(four);
            }
        }
        if drawn.len() == 32 && groups.len() == 12 {
            let stage = self.cup_knockout(cup, &drawn, 1, vec![true, false, false, false, false, false, false], 6100);
            self.world.competition_mut(cup).final_round = Some(stage);
        }
        // else the game prints "erro sul americana size" and plays none
    }

    /// `f.x.yq()`: the Europa League: the Conference League's champion (or a German place),
    /// twelve clubs for the groups (`YF`) and nine qualifiers (`YS`), none of the Champions
    /// League's; its rounds start with the Champions League's losers.
    fn europa_league_draw(&mut self, cup: CompetitionId) {
        if self.world.season() == 3 {
            // f.x.Bd()
            self.world.competition_mut(cup).country_slots_yw[6].country = 162;
        }
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.clubs_ys.clear();
        c.clubs_ze.clear();
        c.main_stage = None;
        c.qualifying_round_1 = None;
        c.qualifying_round_2 = None;
        let (ago, slots, qualifier_slots) = (c.club_ago, c.country_slots.clone(), c.country_slots_yw.clone());
        let mut taken = Vec::new();
        if let Some(cl) = self.world.game.champions_league {
            let c = self.world.competition(cl);
            taken.extend_from_slice(&c.clubs);
            taken.extend_from_slice(&c.clubs_ys);
        }
        let mut groups = Vec::new();
        let mut extra = Vec::new();
        match ago {
            Some(a) if self.world.season() > 1 && !taken.contains(&a) => {
                groups.push(a);
                taken.push(a);
            }
            _ => extra.push(CountrySlots { country: 3, slots: 1, flag_ry: false }),
        }
        if !extra.is_empty() {
            self.slot_picks(&extra, &mut groups, &mut taken, false);
        }
        let mut qualifiers = Vec::new();
        self.slot_picks(&slots, &mut groups, &mut taken, false);
        self.slot_picks(&qualifier_slots, &mut qualifiers, &mut taken, false);
        if groups.len() < 12 {
            self.fill_from_slots(&mut groups, &slots, 12, &mut taken);
        }
        if qualifiers.len() < 9 {
            self.fill_from_slots(&mut qualifiers, &qualifier_slots, 9, &mut taken);
        }
        groups.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let c = self.world.competition_mut(cup);
        c.clubs = groups;
        c.clubs_ys = qualifiers;
    }

    /// `f.J.yq()`: the Conference League: 134 qualifiers (`YS`), none of the Champions League's
    /// or the Europa League's; the last 60, shuffled, start the first round (12001).
    fn conference_league_draw(&mut self, cup: CompetitionId) {
        if self.world.season() == 3 {
            // f.J.Bd()
            let c = self.world.competition_mut(cup);
            c.country_slots_yw[40].country = 162;
            c.country_slots_yw[87].country = 162;
        }
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.clubs_ys.clear();
        c.clubs_ze.clear();
        c.main_stage = None;
        c.qualifying_round_1 = None;
        c.qualifying_round_2 = None;
        c.qualifying_round_3 = None;
        c.qualifying_round_4 = None;
        let qualifier_slots = c.country_slots_yw.clone();
        let mut taken = Vec::new();
        for above in [self.world.game.champions_league, self.world.game.europa_league].into_iter().flatten() {
            let c = self.world.competition(above);
            taken.extend_from_slice(&c.clubs);
            taken.extend_from_slice(&c.clubs_ys);
        }
        let mut qualifiers = Vec::new();
        self.slot_picks(&qualifier_slots, &mut qualifiers, &mut taken, false);
        if qualifiers.len() < 134 {
            self.fill_from_slots(&mut qualifiers, &qualifier_slots, 134, &mut taken);
        }
        self.world.competition_mut(cup).clubs_ys = qualifiers.clone();
        if qualifiers.len() == 134 {
            let mut first = qualifiers[74..=133].to_vec();
            self.runtime.rng.shuffle(&mut first);
            let stage = self.cup_knockout(cup, &first, 1, vec![true, false, false, false, false, false, false], 12001);
            self.world.competition_mut(cup).qualifying_round_1 = Some(stage);
        }
        // else the game prints "erro Conference League()" and plays none
    }

    /// `f.w.yq()`: the Concacaf Champions Cup: the title holder, the countries' places and, to
    /// reach 24, one more club of each country; 8 groups of 3 by level, the group winners on.
    fn concacaf_champions_cup_draw(&mut self, cup: CompetitionId) {
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.main_stage = None;
        let holder = c.title_holder;
        let slots = c.country_slots.clone();
        let mut clubs: Vec<ClubId> = holder.into_iter().collect();
        let mut taken = clubs.clone();
        self.slot_picks(&slots, &mut clubs, &mut taken, false);
        if clubs.len() < 24 {
            self.fill_from_slots(&mut clubs, &slots, 24, &mut taken);
        }
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        self.world.competition_mut(cup).clubs = clubs.clone();
        if clubs.len() >= 24 {
            const POTS: [usize; 24] = [0, 15, 23, 4, 14, 22, 6, 13, 21, 2, 11, 20, 1, 12, 19, 5, 10, 18, 7, 9, 17, 3, 8, 16];
            let drawn: Vec<ClubId> = POTS.iter().map(|&i| clubs[i]).collect();
            let options = StageOptions { teams: 24, groups: 8, knockout_teams: 1, two_rounds: true, skip_two_dates: true, ..Default::default() };
            let kind = self.world.competition(cup).kind;
            let stage = self.new_league_stage(&options, &drawn, 0, kind, cup, true);
            self.world.competition_mut(cup).main_stage = Some(stage);
        }
    }

    /// `f.r.yq()`: the Libertadores of the new season (the game's default groups of the first
    /// season, with the real clubs, are another way).
    fn libertadores_draw(&mut self, cup: CompetitionId) {
        if self.world.season() == 1 && self.world.game.default_international_groups {
            crate::unported!("f.r.yr(): the first season's default Libertadores groups");
        }
        self.world.competition_mut(cup).flag_yx = false;
        self.libertadores_new_season(cup);
    }

    /// `f.r.ys()`: the title holders (or a Brazilian and an Argentine place), the countries'
    /// places for the groups (`YV`) and the qualifiers (`YW`), 28 and 19 with the spare ones;
    /// the first six qualifiers start the first round.
    fn libertadores_new_season(&mut self, cup: CompetitionId) {
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.clubs_ys.clear();
        c.main_stage = None;
        c.qualifying_round_1 = None;
        c.qualifying_round_2 = None;
        c.qualifying_round_3 = None;
        let (yt, yu) = (c.club_yt, c.club_yu);
        let (slots, qualifier_slots) = (c.country_slots.clone(), c.country_slots_yw.clone());
        let mut extra = Vec::new();
        let mut groups: Vec<ClubId> = Vec::new();
        match yt {
            None => extra.push(CountrySlots { country: 29, slots: 1, flag_ry: false }),
            Some(t) => groups.push(t),
        }
        match yu {
            None => extra.push(CountrySlots { country: 11, slots: 1, flag_ry: false }),
            Some(u) if Some(u) != yt => groups.push(u),
            Some(_) => {}
        }
        if let Some(t) = yt
            && yt == yu
        {
            extra.push(CountrySlots { country: self.world.club(t).country, slots: 1, flag_ry: false });
        }
        let mut qualifiers: Vec<ClubId> = Vec::new();
        let mut taken: Vec<ClubId> = groups.clone();
        self.slot_picks(&extra, &mut groups, &mut taken, false);
        self.slot_picks(&slots, &mut groups, &mut taken, false);
        self.slot_picks(&qualifier_slots, &mut qualifiers, &mut taken, false);
        if groups.len() < 28 {
            self.fill_from_slots(&mut groups, &slots, 28, &mut taken);
        }
        if qualifiers.len() < 19 {
            self.fill_from_slots(&mut qualifiers, &qualifier_slots, 19, &mut taken);
        }
        groups.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let c = self.world.competition_mut(cup);
        c.clubs = groups.clone();
        c.clubs_ys = qualifiers.clone();
        if qualifiers.len() == 19 && groups.len() == 28 {
            let first: Vec<ClubId> = qualifiers[..6].to_vec();
            self.libertadores_first_round(cup, first);
        }
        // else the game prints "erro Libertadores()" and plays none
    }

    /// `f.r.ah(ArrayList)`: the first qualifying round, shuffled, two legs (4101).
    fn libertadores_first_round(&mut self, cup: CompetitionId, mut clubs: Vec<ClubId>) {
        self.runtime.rng.shuffle(&mut clubs);
        let mut legs = vec![true, true, true, false, false, false, false];
        legs.truncate(7);
        let kind = self.world.competition(cup).kind;
        let stage = self.new_knockout_stage(cup, None, clubs.len() as i32, kind, 1, legs, 4101);
        self.world.competition_mut(cup).qualifying_round_1 = Some(stage);
        self.new_knockout_round(stage, &clubs, 0, true, 0, kind, false);
    }

    /// `f.r.g(ArrayList, ArrayList)` (the end of 4101): the losers out; the winners and the
    /// thirteen qualifiers left, the best eight against the other eight from other countries.
    pub(crate) fn libertadores_second_round(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4101);
        }
        if self.world.season() == 1 && self.world.competition(cup).flag_yx {
            crate::unported!("f.r.ae(ArrayList): the default second round");
        }
        let mut clubs: Vec<ClubId> = self.world.competition(cup).clubs_ys[6..=18].to_vec();
        clubs.extend_from_slice(winners);
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let mut best: Vec<ClubId> = clubs[..8].to_vec();
        let mut rest: Vec<ClubId> = clubs[8..16].to_vec();
        self.runtime.rng.shuffle(&mut best);
        self.runtime.rng.shuffle(&mut rest);
        let mut ties = vec![Tie::default(); 8];
        for t in ties.iter_mut() {
            t.two(self, &mut best, &mut rest);
        }
        separate_countries(self, &mut ties);
        let drawn: Vec<ClubId> = ties.iter().flat_map(|t| t.0.clone()).collect();
        let kind = self.world.competition(cup).kind;
        let legs = vec![true, false, false, false, false, false, false];
        let stage = self.new_knockout_stage(cup, None, drawn.len() as i32, kind, 1, legs, 4102);
        self.world.competition_mut(cup).qualifying_round_2 = Some(stage);
        self.new_knockout_round(stage, &drawn, 0, true, 0, kind, false);
    }

    /// `f.r.h(ArrayList, ArrayList)` (the end of 4102): the third round of the winners (4103).
    pub(crate) fn libertadores_third_round(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4102);
        }
        if self.world.season() == 1 && self.world.competition(cup).flag_yx {
            crate::unported!("f.r.af(ArrayList): the default third round");
        }
        let kind = self.world.competition(cup).kind;
        let legs = vec![true, false, false, false, false, false, false];
        let stage = self.new_knockout_stage(cup, None, winners.len() as i32, kind, 1, legs, 4103);
        self.world.competition_mut(cup).qualifying_round_3 = Some(stage);
        self.new_knockout_round(stage, winners, 0, true, 0, kind, false);
    }

    /// `f.r.i(ArrayList, ArrayList)` (the end of 4103): the losers go to the Sudamericana; the 28
    /// and the four winners in eight groups of four (4104), by pots.
    pub(crate) fn libertadores_groups(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4103);
        }
        if let Some(sud) = self.world.game.copa_sudamericana {
            self.world.competition_mut(sud).clubs_aao.extend_from_slice(losers);
            self.sudamericana_messages(losers, 0);
        }
        if self.world.season() == 1 && self.world.competition(cup).flag_yx {
            crate::unported!("f.r.ag(ArrayList): the default groups");
        }
        let clubs = self.world.competition(cup).clubs.clone();
        let mut pot1 = clubs[0..8].to_vec();
        let mut pot2 = clubs[8..16].to_vec();
        let mut pot3 = clubs[16..24].to_vec();
        let mut pot4 = clubs[24..28].to_vec();
        pot4.extend_from_slice(winners);
        for pot in [&mut pot1, &mut pot2, &mut pot3, &mut pot4] {
            self.runtime.rng.shuffle(pot);
        }
        let mut groups = vec![Tie::default(); 8];
        for g in groups.iter_mut() {
            g.four(self, [&mut pot1, &mut pot2, &mut pot3, &mut pot4]);
        }
        separate_countries(self, &mut groups);
        self.runtime.rng.shuffle(&mut groups);
        let drawn: Vec<ClubId> = groups.iter().flat_map(|g| g.0.clone()).collect();
        if drawn.len() >= 32 {
            let options = StageOptions {
                teams: 32,
                groups: 8,
                knockout_teams: 2,
                two_rounds: true,
                skip_two_dates: false,
                two_legged_knockout: vec![true, true, true, false, false, false, false],
                ..Default::default()
            };
            let kind = self.world.competition(cup).kind;
            let stage = self.new_league_stage(&options, &drawn, 0, kind, cup, true);
            // s.fb(4104), the cup's name
            self.world.stage_mut(stage).format_code = 4104;
            self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
            self.world.competition_mut(cup).main_stage = Some(stage);
        }
    }

    /// `f.H.c(ArrayList, int)`: the human clubs that drop into the Sudamericana hear of it.
    fn sudamericana_messages(&mut self, clubs: &[ClubId], from_groups: i32) {
        for &c in clubs {
            if self.world.club(c).human
                && let Some(coach) = self.world.club(c).coach
            {
                let (sender, id) = if from_groups == 0 { (32, 87) } else { (33, 88) };
                brasfoot_engine::messages::send(self.world, coach, sender, id, String::new(), String::new());
            }
        }
    }

    /// A knockout of the continental cups: `new z(null, clubs, kind, rounds, legs, cup, phase)`
    /// and its first round.
    pub(crate) fn cup_knockout(&mut self, cup: CompetitionId, clubs: &[ClubId], rounds: i32, legs: Vec<bool>, phase: i32) -> brasfoot_model::ids::StageId {
        let kind = self.world.competition(cup).kind;
        let first_leg_twice = legs[0];
        let stage = self.new_knockout_stage(cup, None, clubs.len() as i32, kind, rounds, legs, phase);
        self.new_knockout_round(stage, clubs, 0, first_leg_twice, 0, kind, false);
        stage
    }

    /// `f.v.yq()`: the Champions League of the new season: the title holders (or a French and
    /// a German place), 26 clubs for the groups (`YV`), 52 for the qualifiers (`YW`), the Europa
    /// League's place for the Conference League's champion kept out; the last four qualifiers
    /// play the first round (4000).
    fn champions_league_draw(&mut self, cup: CompetitionId) {
        if self.world.season() == 3 {
            // f.v.Bd(): the countries' places move in the third season
            let c = self.world.competition_mut(cup);
            c.country_slots[8].country = 192;
            c.country_slots_yw[0].country = 15;
            c.country_slots_yw[43].country = 102;
            c.country_slots_yw[48].country = 92;
            c.country_slots_yw[2].country = 162;
        }
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.clubs.clear();
        c.clubs_ys.clear();
        c.main_stage = None;
        c.final_round = None;
        c.qualifying_round_1 = None;
        c.qualifying_round_2 = None;
        c.qualifying_round_3 = None;
        c.qualifying_round_4 = None;
        let (yt, zc) = (c.club_yt, c.club_zc);
        let (slots, qualifier_slots) = (c.country_slots.clone(), c.country_slots_yw.clone());
        let europa_place = self.world.game.europa_league.and_then(|e| self.world.competition(e).club_ago);
        let mut extra = Vec::new();
        let mut groups: Vec<ClubId> = Vec::new();
        match yt {
            None => extra.push(CountrySlots { country: 72, slots: 1, flag_ry: false }),
            Some(t) => groups.push(t),
        }
        match zc {
            None => extra.push(CountrySlots { country: 3, slots: 1, flag_ry: false }),
            Some(z) if Some(z) != yt => groups.push(z),
            Some(_) => {}
        }
        if let Some(t) = yt
            && yt == zc
        {
            extra.push(CountrySlots { country: self.world.club(t).country, slots: 1, flag_ry: false });
        }
        let mut qualifiers = Vec::new();
        let mut taken = groups.clone();
        if !extra.is_empty() {
            self.slot_picks(&extra, &mut groups, &mut taken, false);
        }
        self.slot_picks(&slots, &mut groups, &mut taken, false);
        if self.world.season() > 1
            && let Some(e) = europa_place
        {
            taken.push(e);
        }
        self.slot_picks(&qualifier_slots, &mut qualifiers, &mut taken, false);
        if groups.len() < 26 {
            self.fill_from_slots(&mut groups, &slots, 26, &mut taken);
        }
        if qualifiers.len() < 52 {
            self.fill_from_slots(&mut qualifiers, &qualifier_slots, 52, &mut taken);
        }
        groups.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let c = self.world.competition_mut(cup);
        c.clubs = groups.clone();
        c.clubs_ys = qualifiers.clone();
        if groups.len() == 26 && qualifiers.len() == 52 {
            let mut first = qualifiers[48..52].to_vec();
            self.runtime.rng.shuffle(&mut first);
            let stage = self.cup_knockout(cup, &first, -1, vec![false; 7], 4000);
            self.world.competition_mut(cup).final_round = Some(stage);
        }
        // else the game prints "erro Liga C()" and plays none
    }

    /// The Conference League (`mj()`) takes the clubs out of the Champions League's or the
    /// Europa League's qualifiers (`zo().addAll`, `b(clubs, 0)`).
    pub(crate) fn send_to_conference_league(&mut self, clubs: &[ClubId]) {
        if let Some(conf) = self.world.game.conference_league {
            self.world.competition_mut(conf).clubs_ze.extend_from_slice(clubs);
            self.qualifier_messages(clubs, 35, 90, 35, 90, 0);
        }
    }

    /// The Europa League (`aK()`) takes them (`zo().addAll`, `b(clubs, 0)`).
    fn send_to_europa_league(&mut self, clubs: &[ClubId], from_groups: i32) {
        if let Some(el) = self.world.game.europa_league {
            if from_groups == 0 {
                self.world.competition_mut(el).clubs_ze.extend_from_slice(clubs);
            }
            self.qualifier_messages(clubs, 30, 85, 31, 86, from_groups);
        }
    }

    /// `b(ArrayList, int)` of f.x, f.J, f.H: the human clubs hear they move to the cup.
    fn qualifier_messages(&mut self, clubs: &[ClubId], sender: i32, id: i32, sender_groups: i32, id_groups: i32, from_groups: i32) {
        for &c in clubs {
            if self.world.club(c).human
                && let Some(coach) = self.world.club(c).coach
            {
                let (s, i) = if from_groups == 0 { (sender, id) } else { (sender_groups, id_groups) };
                brasfoot_engine::messages::send(self.world, coach, s, i, String::new(), String::new());
            }
        }
    }

    /// `f.v.aj(ArrayList)` (the end of 4000): the three losers to the Conference League; the
    /// next 29 qualifiers and the winner paired at random home or away (4001).
    pub(crate) fn champions_league_first_qualifying(&mut self, cup: CompetitionId, winners: &[ClubId]) {
        let ys = self.world.competition(cup).clubs_ys.clone();
        let mut out = Vec::new();
        for &c in &ys[48..=51] {
            if !winners.contains(&c) {
                out.push(c);
                self.place(c, cup, -1, 4000);
            }
        }
        self.send_to_conference_league(&out);
        let mut a: Vec<ClubId> = ys[19..=33].to_vec();
        let mut b: Vec<ClubId> = ys[34..=47].to_vec();
        b.push(winners[0]);
        self.runtime.rng.shuffle(&mut a);
        self.runtime.rng.shuffle(&mut b);
        let mut r = self.runtime.rng.new_random();
        let mut drawn = Vec::new();
        for i in 0..a.len() {
            if r.next_boolean() {
                drawn.push(a[i]);
                drawn.push(b[i]);
            } else {
                drawn.push(b[i]);
                drawn.push(a[i]);
            }
        }
        let stage = self.cup_knockout(cup, &drawn, 1, vec![true; 7], 4001);
        self.world.competition_mut(cup).qualifying_round_1 = Some(stage);
    }

    /// `f.v.g(ArrayList, ArrayList)` (the end of 4001): the losers to the Conference League; five
    /// more qualifiers with the winners against four more (4002).
    pub(crate) fn champions_league_second_qualifying(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4001);
        }
        self.send_to_conference_league(losers);
        let ys = self.world.competition(cup).clubs_ys.clone();
        let mut a: Vec<ClubId> = ys[10..=14].to_vec();
        a.extend_from_slice(winners);
        let mut b: Vec<ClubId> = ys[15..=18].to_vec();
        self.runtime.rng.shuffle(&mut a);
        self.runtime.rng.shuffle(&mut b);
        a.extend(b);
        let stage = self.cup_knockout(cup, &a, 1, vec![true; 7], 4002);
        self.world.competition_mut(cup).qualifying_round_2 = Some(stage);
    }

    /// `f.v.h(ArrayList, ArrayList)` (the end of 4002): the losers to the Europa League's first
    /// round (`f.x.aj`); the winners with more qualifiers in two pots (4003).
    pub(crate) fn champions_league_third_qualifying(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4002);
        }
        let mut first: Vec<ClubId> = losers[0..=9].to_vec();
        self.runtime.rng.shuffle(&mut first);
        let mut to_europa = first;
        to_europa.extend_from_slice(&losers[10..=11]);
        // aK().b(clubs, 0) tells the human clubs; aK().aj(clubs) draws them
        self.qualifier_messages(&to_europa, 30, 85, 31, 86, 0);
        if let Some(el) = self.world.game.europa_league {
            self.europa_league_first_round(el, &to_europa);
        }
        let ys = self.world.competition(cup).clubs_ys.clone();
        let mut a: Vec<ClubId> = winners[0..=9].to_vec();
        a.extend_from_slice(&ys[2..=3]);
        self.runtime.rng.shuffle(&mut a);
        let mut b: Vec<ClubId> = ys[4..=9].to_vec();
        b.extend_from_slice(&winners[10..=11]);
        self.runtime.rng.shuffle(&mut b);
        a.extend(b);
        let stage = self.cup_knockout(cup, &a, 1, vec![true; 7], 4003);
        self.world.competition_mut(cup).qualifying_round_3 = Some(stage);
    }

    /// `f.v.j(ArrayList, ArrayList)` (the end of 4003): the losers to the Europa League; the
    /// play-offs (4004).
    pub(crate) fn champions_league_playoffs(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4003);
        }
        self.send_to_europa_league(losers, 0);
        let ys = self.world.competition(cup).clubs_ys.clone();
        let mut a: Vec<ClubId> = winners[0..=5].to_vec();
        a.extend_from_slice(&ys[0..=1]);
        self.runtime.rng.shuffle(&mut a);
        let mut b: Vec<ClubId> = winners[6..=9].to_vec();
        self.runtime.rng.shuffle(&mut b);
        a.extend(b);
        let stage = self.cup_knockout(cup, &a, 1, vec![true; 7], 4004);
        self.world.competition_mut(cup).qualifying_round_4 = Some(stage);
    }

    /// `f.v.k(ArrayList, ArrayList)` (the end of 4004): the losers to the Europa League; the 26
    /// and the six winners in eight groups of four by pots (4005).
    pub(crate) fn champions_league_groups(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 4004);
        }
        self.send_to_europa_league(losers, 0);
        self.world.competition_mut(cup).main_stage = None;
        let clubs = self.world.competition(cup).clubs.clone();
        let mut drawn = Vec::new();
        if clubs.len() == 26 {
            let mut pot1 = clubs[0..8].to_vec();
            let mut pot2 = clubs[8..16].to_vec();
            let mut pot3 = clubs[16..24].to_vec();
            let mut pot4 = clubs[24..26].to_vec();
            pot4.extend_from_slice(winners);
            for pot in [&mut pot1, &mut pot2, &mut pot3, &mut pot4] {
                self.runtime.rng.shuffle(pot);
            }
            let mut groups = vec![Tie::default(); 8];
            for g in groups.iter_mut() {
                g.four(self, [&mut pot1, &mut pot2, &mut pot3, &mut pot4]);
            }
            separate_countries(self, &mut groups);
            self.runtime.rng.shuffle(&mut groups);
            drawn = groups.iter().flat_map(|g| g.0.clone()).collect();
        }
        if drawn.len() >= 32 {
            let options = StageOptions {
                teams: 32,
                groups: 8,
                knockout_teams: 2,
                two_rounds: true,
                skip_two_dates: false,
                two_legged_knockout: vec![true, true, true, false, false, false, false],
                ..Default::default()
            };
            let kind = self.world.competition(cup).kind;
            let stage = self.new_league_stage(&options, &drawn, 0, kind, cup, true);
            self.world.stage_mut(stage).format_code = 4005;
            self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
            self.world.competition_mut(cup).main_stage = Some(stage);
        }
    }

    /// `f.v.yt()` (the end of the groups, 4005): the thirds to the Europa League; group winners
    /// against runners-up of other countries, the knockout of the group stage.
    pub(crate) fn champions_league_knockout(&mut self, cup: CompetitionId) {
        self.groups_to_knockout(cup, true);
    }

    /// `f.r.yt()` (the end of the groups, 4104): the thirds hear of the Sudamericana; group
    /// winners against runners-up as they come.
    pub(crate) fn libertadores_knockout(&mut self, cup: CompetitionId) {
        self.groups_to_knockout(cup, false);
    }

    /// `yt()` of f.v and f.r: the seconds and firsts, shuffled, in ties (from different
    /// countries in Europe); the thirds go to the second tier.
    fn groups_to_knockout(&mut self, cup: CompetitionId, europe: bool) {
        let main = self.world.competition(cup).main_stage.expect("the cup's groups");
        let mut seconds = Vec::new();
        let mut firsts = Vec::new();
        let mut thirds = Vec::new();
        for g in &self.world.stage(main).groups {
            seconds.push(g.clubs[1]);
            firsts.push(g.clubs[0]);
            thirds.push(g.clubs[2]);
        }
        if europe {
            self.send_to_europa_league(&thirds, 1);
        } else {
            self.sudamericana_messages(&thirds, 1);
        }
        self.runtime.rng.shuffle(&mut seconds);
        self.runtime.rng.shuffle(&mut firsts);
        let mut ties = vec![Tie::default(); 8];
        for t in ties.iter_mut() {
            if europe {
                t.two(self, &mut seconds, &mut firsts);
            } else {
                t.two_plain(&mut seconds, &mut firsts);
            }
        }
        if europe {
            separate_countries(self, &mut ties);
        }
        let drawn: Vec<ClubId> = ties.iter().flat_map(|t| t.0.clone()).collect();
        let kind = self.world.competition(cup).kind;
        let legs = vec![true, true, true, false, false, false, false];
        let stage = self.new_knockout_stage(cup, Some(main), drawn.len() as i32, kind, 0, legs, -1);
        self.world.stage_mut(main).knockout = Some(stage);
        self.new_knockout_round(stage, &drawn, 0, true, 0, kind, false);
    }

    /// `f.x.aj(ArrayList)`: the Europa League's first round: ten of the Champions League's
    /// losers against the other two and two of its own qualifiers (6001).
    fn europa_league_first_round(&mut self, el: CompetitionId, losers: &[ClubId]) {
        let ys = self.world.competition(el).clubs_ys.clone();
        let mut a: Vec<ClubId> = losers[0..=9].to_vec();
        let mut b: Vec<ClubId> = losers[10..=11].to_vec();
        b.extend_from_slice(&ys[7..=8]);
        self.runtime.rng.shuffle(&mut a);
        self.runtime.rng.shuffle(&mut b);
        a.extend(b);
        let stage = self.cup_knockout(el, &a, 1, vec![true; 7], 6001);
        self.world.competition_mut(el).qualifying_round_1 = Some(stage);
    }
}
