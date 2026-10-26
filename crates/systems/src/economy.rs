// Port of bfmod.Economia: the AI clubs' money. The game gives every AI club a fixed cash that
// never moves (and resets it on a coach change), so the mod keeps each fortune and writes it
// into the game's cash on every tick; money moves through the game's own transfer history.

use std::collections::{HashMap, HashSet};

use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{EconomyClub, EconomyState, OwnerEvent, SponsorshipKind};
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;

use crate::awards::season::Season;
use crate::{Mod, sorte};

pub const MI: i64 = 1_000_000;

/// `CLUBES_ESTADO`: clubs that start with a billionaire owner, by name.
const STATE_OWNED: [&str; 9] = ["Paris Saint-Germain", "PSG", "Manchester City", "Newcastle United", "Chelsea", "Al Hilal", "Al Nassr", "Al Ittihad", "Al Ahli SFC"];
const FORTUNE_FACTOR: [f64; 7] = [0.3, 0.3, 0.5, 0.8, 1.4, 2.5, 2.5];
/// `MARGEM`: the season's income over the wages, by reputation.
const MARGIN: [f64; 7] = [1.02, 1.02, 1.05, 1.10, 1.16, 1.22, 1.22];
/// The sizes of `MARCA_A`, `MARCA_B`, `DONO_A` and `DONO_B` (the words are in the catalog).
const BRAND_A: i32 = 30;
const BRAND_B: i32 = 16;
const OWNER_A: i32 = 16;
const OWNER_B: i32 = 6;

/// `Economia.ano()`
pub fn year(world: &World) -> i32 {
    world.game.start_year + world.season() - 1
}

/// `ah.lp()`: the club is a national team.
pub fn is_national_team(world: &World, club: ClubId) -> bool {
    world.game.countries.iter().any(|&c| world.country(c).national_team == Some(club))
}

/// The national teams, for the loops that ask [`is_national_team`] of every club.
pub fn national_teams(world: &World) -> HashSet<ClubId> {
    world.game.countries.iter().filter_map(|&c| world.country(c).national_team).collect()
}

/// `Economia.humanos()`: the clubs of the human coaches, by game id.
pub fn humans(world: &World) -> HashSet<i32> {
    world.game.human_coaches.iter().filter_map(|&c| world.coach(c).club_id).map(|c| world.club(c).id).collect()
}

/// `Economia.humano(ah)`
pub fn is_human(world: &World, club: ClubId) -> bool {
    humans(world).contains(&world.club(club).id)
}

/// `Economia.clubesIA()`: the AI clubs (no national teams, no human ones), in the game's order.
pub fn ai_clubs(world: &World) -> Vec<ClubId> {
    let humans = humans(world);
    let national = national_teams(world);
    world.game.clubs.iter().copied().filter(|&c| !national.contains(&c) && !world.club(c).human && !humans.contains(&world.club(c).id)).collect()
}

/// `Economia.timesPorId()`: the clubs by game id (the last one when two share it).
pub fn clubs_by_id(world: &World) -> HashMap<i32, ClubId> {
    world.game.clubs.iter().map(|&c| (world.club(c).id, c)).collect()
}

/// `Temporada.pesosDasLigas()`: how strong each club's league is (1.0, 1.5 or 2.0).
pub fn league_weights(world: &World) -> HashMap<ClubId, f64> {
    let mut out = HashMap::new();
    for &country in &world.game.league_countries {
        for &league in &world.country(country).divisions {
            let Some(stage) = world.competition(league).league else { continue };
            let clubs = &world.stage(stage).clubs;
            if clubs.is_empty() {
                continue;
            }
            let total: f64 = clubs.iter().map(|&c| world.club(c).reputation as f64).sum();
            let average = total / clubs.len() as f64;
            let weight = if average >= 3.35 {
                2.0
            } else if average >= 2.7 {
                1.5
            } else {
                1.0
            };
            for &c in clubs {
                out.insert(c, weight);
            }
        }
    }
    out
}

/// `Economia.assinatura(ap)`: how the economy recognises a transfer it already charged.
fn signature(world: &World, t: &brasfoot_model::saved::Transfer) -> String {
    let name = t.player.map(|p| world.player(p).name.clone()).unwrap_or_else(|| t.player_number.to_string());
    format!("{}|{}/{}/{}|{}|{}|{}|{}", t.year, t.day, t.month, t.year, name, t.from_club, t.to_club, t.fee)
}

impl Mod<'_> {
    fn state(&mut self) -> &mut EconomyState {
        self.world.mods.economy.as_mut().expect("the economy has started")
    }

    /// `Economia.carregar()` and `iniciar()`: the first time on a save, with no state beside it.
    pub fn ensure_economy(&mut self) {
        if self.world.mods.economy.is_some() {
            if self.sys.humans_before.is_none() {
                self.sys.humans_before = Some(humans(self.world));
            }
            return;
        }
        let state = EconomyState { last_season: self.world.season(), ..Default::default() };
        self.world.mods.economy = Some(state);
        self.sys.humans_before = Some(humans(self.world));
        self.start_economy();
    }

    /// `Economia.iniciar()`: every AI club's fortune and owner, then its sponsor; the transfers
    /// before are not charged.
    fn start_economy(&mut self) {
        let year = year(self.world);
        let clubs = ai_clubs(self.world);
        for &t in &clubs {
            let mut c = EconomyClub { id: self.world.club(t).id, fortune: self.initial_fortune(t), ..Default::default() };
            let name = self.world.club(t).name.to_lowercase();
            for owned in STATE_OWNED {
                if owned.to_lowercase() == name {
                    c.has_owner = true;
                    c.owner_funding = (120 + sorte!(self, "Economia").next_int_bounded(131)) as i64 * MI;
                    c.owner_since = year;
                    c.owner_name = Some(self.new_owner_name());
                    c.fortune += (300 + sorte!(self, "Economia").next_int_bounded(301)) as i64 * MI;
                }
            }
            self.state().clubs.push(c);
        }
        let weights = league_weights(self.world);
        for &t in &clubs {
            self.new_sponsorship(t, None, &weights, false);
        }
        for i in 0..self.world.game.transfers.len() {
            if self.world.game.transfers[i].player.is_none() {
                // the mod relinks records the load left without a player
                self.world.relink_transfer(i);
            }
        }
        let signatures = self.world.game.transfers.iter().map(|t| signature(self.world, t)).collect();
        self.state().processed = signatures;
        self.state().last_season = self.world.season();
        self.sys.humans_before = Some(humans(self.world));
    }

    /// `Economia.tick()`
    pub fn economy_tick(&mut self) {
        self.ensure_economy();
        self.ensure_sponsorships();
        self.clean_up();
        // comandos(): the test commands of sav/<save>.comandos are not ported
        self.account_transfers();
        if self.world.season() > self.state().last_season {
            let season = self.world.season();
            self.state().last_season = season;
            self.new_economy_season();
        }
        self.windows_tick();
        self.account_transfers();
        self.assets_tick();
        self.satellites_tick();
        self.sync_cash();
        // salvar(): what sav/<save>.economia holds until the next tick
        self.sys.economy_saved = self.world.mods.economy.clone();
    }

    /// `Economia.fortunaInicial(ah)`: a year of wages times a factor of the reputation.
    fn initial_fortune(&mut self, club: ClubId) -> i64 {
        let yearly = self.world.club_wages(club) as f64 * 12.0;
        let reputation = self.world.club(club).reputation.clamp(0, 6) as usize;
        let draw = 0.7 + sorte!(self, "Economia").next_double() * 0.6;
        (2 * MI).max(java_round(yearly * FORTUNE_FACTOR[reputation] * draw))
    }

    /// `Economia.novoNomeDeDono()`: indices into `DONO_A` and `DONO_B`.
    pub(crate) fn new_owner_name(&mut self) -> (u16, u16) {
        let a = sorte!(self, "Economia").next_int_bounded(OWNER_A) as u16;
        (a, sorte!(self, "Economia").next_int_bounded(OWNER_B) as u16)
    }

    /// `Economia.garantirPatrocinios()`: a club with no sponsor gets one.
    fn ensure_sponsorships(&mut self) {
        let mut weights = None;
        for t in ai_clubs(self.world) {
            if self.club(t).brand.is_none() {
                let w = weights.get_or_insert_with(|| league_weights(self.world)).clone();
                self.new_sponsorship(t, None, &w, false);
            }
        }
    }

    /// `Economia.sanear()`: once per save, the debts of the mod's first versions are forgiven.
    fn clean_up(&mut self) {
        if self.state().cleaned {
            return;
        }
        self.state().cleaned = true;
        for t in ai_clubs(self.world) {
            if self.club(t).fortune < 0 {
                let fortune = java_round(self.initial_fortune(t) as f64 * 0.3);
                self.club(t).fortune = fortune;
            }
        }
        self.update_reputations();
    }

    /// `Economia.patrocinioEsperado(ah, Map)`: what a club this size usually gets a year.
    pub fn expected_sponsorship(&self, club: ClubId, weights: &HashMap<ClubId, f64>) -> i64 {
        let yearly = self.world.club_wages(club) as f64 * 12.0;
        let reputation = self.world.club(club).reputation.max(1);
        java_round(yearly * (0.15 + 0.05 * reputation as f64) * (weights.get(&club).copied().unwrap_or(1.0) / 1.5))
    }

    /// `Economia.novoPatrocinio(ah, Clube, Temporada, Map, boolean)`: the club's master sponsor,
    /// worth more after titles; owners sometimes inflate it. The news is not ported.
    fn new_sponsorship(&mut self, club: ClubId, last: Option<&Season>, weights: &HashMap<ClubId, f64>, _news: bool) {
        let expected = self.expected_sponsorship(club, weights) as f64;
        let factor = self.sponsorship_factor(club);
        let year = year(self.world);
        let mut value = expected * factor * (0.85 + sorte!(self, "Economia").next_double() * 0.3);
        if let Some(t) = last {
            value *= 1.0 + (t.title_points(club) / 60.0).min(0.3);
        }
        let (old_brand, has_owner) = {
            let c = self.club(club);
            (c.brand, c.has_owner)
        };
        let kind = if has_owner && sorte!(self, "Economia").next_int_bounded(100) < 50 {
            value *= 2.5;
            SponsorshipKind::OwnerLinked
        } else {
            match sorte!(self, "Economia").next_int_bounded(100) {
                ..50 => SponsorshipKind::Fixed,
                ..85 => SponsorshipKind::Mixed,
                _ => SponsorshipKind::Risky,
            }
        };
        let keep = sorte!(self, "Economia").next_int_bounded(100) < 60 && old_brand.is_some();
        let brand = if keep { old_brand } else { Some(self.new_brand()) };
        let until = year + 1 + sorte!(self, "Economia").next_int_bounded(3);
        let c = self.club(club);
        c.brand = brand;
        c.sponsorship_kind = kind;
        c.sponsorship = java_round(value);
        c.sponsorship_until = until;
    }

    /// `Economia.novaTemporada()`: the season's income of every AI club, then the owners'
    /// events and the reputations.
    fn new_economy_season(&mut self) {
        let last = (self.world.season() > 1).then(|| Season::new(self.world, self.world.season() - 1));
        let weights = league_weights(self.world);
        let year = year(self.world);
        for t in ai_clubs(self.world) {
            if self.club(t).brand.is_none() {
                self.new_sponsorship(t, None, &weights, false);
            }
            // only the gap to what a club this size usually gets counts
            let gap = self.club(t).sponsorship - self.expected_sponsorship(t, &weights);
            self.club(t).fortune += gap;
            if year >= self.club(t).sponsorship_until {
                self.new_sponsorship(t, last.as_ref(), &weights, true);
            }
            let yearly = self.world.club_wages(t) as f64 * 12.0;
            let reputation = self.world.club(t).reputation.clamp(0, 6) as usize;
            let mut income = yearly * (MARGIN[reputation] - 1.0) * (0.9 + sorte!(self, "Economia").next_double() * 0.2);
            if let Some(s) = &last {
                income += s.title_points(t) * 1.5 * MI as f64;
            }
            let c = self.club(t);
            c.fortune += java_round(income) + if c.has_owner { c.owner_funding } else { 0 };
            // the upkeep of a big club keeps fortunes from piling up forever
            if c.fortune > 200 * MI {
                c.fortune -= java_round((c.fortune - 200 * MI) as f64 * 0.15);
            }
        }
        self.owner_events();
        self.update_reputations();
    }

    /// `Economia.atualizarReputacoes()`: money and a strong squad lift an AI club's reputation.
    pub fn update_reputations(&mut self) {
        if self.world.mods.economy.is_none() {
            return;
        }
        for t in ai_clubs(self.world) {
            let (has_owner, fortune) = {
                let c = self.club(t);
                (c.has_owner, c.fortune)
            };
            let mut reputation = self.world.club(t).reputation;
            if reputation >= 5 {
                continue;
            }
            let average = crate::market::team_average(self.world, t);
            if reputation < 3 && (has_owner || fortune >= 120 * MI) {
                self.world.club_mut(t).reputation = 3;
                reputation = 3;
            }
            if reputation < 4 && (fortune >= 260 * MI || (has_owner && fortune >= 160 * MI)) && average >= crate::market::scale(self.world, 56) as f64 {
                self.world.club_mut(t).reputation = 4;
                reputation = 4;
            }
            if reputation < 5 && (fortune >= 600 * MI || (has_owner && fortune >= 420 * MI)) && average >= crate::market::scale(self.world, 65) as f64 {
                self.world.club_mut(t).reputation = 5;
            }
        }
    }

    /// `Economia.eventosDeDonos()`: investors buy a club most seasons, sometimes two; an owner
    /// may give up.
    fn owner_events(&mut self) {
        for chance in [45, 15] {
            if sorte!(self, "Economia").next_int_bounded(100) >= chance {
                break;
            }
            let targets: Vec<ClubId> = ai_clubs(self.world).into_iter().filter(|&t| self.world.club(t).reputation >= 2 && !self.club(t).has_owner).collect();
            if !targets.is_empty() {
                let i = sorte!(self, "Economia").next_int_bounded(targets.len() as i32) as usize;
                self.buy_club(targets[i]);
            }
        }
        if sorte!(self, "Economia").next_int_bounded(100) < 10 {
            let owned: Vec<ClubId> = ai_clubs(self.world).into_iter().filter(|&t| self.club(t).has_owner).collect();
            if !owned.is_empty() {
                let i = sorte!(self, "Economia").next_int_bounded(owned.len() as i32) as usize;
                // the news is not ported
                self.owner_leaves(owned[i]);
            }
        }
    }

    /// `Economia.comprarClube(ah)`: a fund buys an AI club: money now, a yearly injection, an
    /// owner-linked sponsor and three seasons of spending (joining the open window).
    pub fn buy_club(&mut self, club: ClubId) {
        let year = year(self.world);
        let name = self.new_owner_name();
        let funding = (80 + sorte!(self, "Economia").next_int_bounded(171)) as i64 * MI;
        let injection = (150 + sorte!(self, "Economia").next_int_bounded(251)) as i64 * MI;
        let c = self.club(club);
        c.has_owner = true;
        c.owner_since = year;
        c.owner_name = Some(name);
        c.owner_funding = funding;
        c.fortune += injection;
        c.project_until = year + 2;
        if self.world.club(club).reputation < 3 {
            self.world.club_mut(club).reputation = 3;
        }
        let weights = league_weights(self.world);
        self.new_sponsorship(club, None, &weights, false);
        let club_name = self.world.club(club).name.clone();
        self.state().events.push(OwnerEvent { year, club: club_name, arrived: true });
        // the news is not ported
        self.new_project(club);
    }

    /// `Economia.novaMarca()`: indices into `MARCA_A` and `MARCA_B`.
    pub(crate) fn new_brand(&mut self) -> (u16, u16) {
        let a = sorte!(self, "Economia").next_int_bounded(BRAND_A) as u16;
        (a, sorte!(self, "Economia").next_int_bounded(BRAND_B) as u16)
    }

    /// `Economia.venderClubeDoJogador(ah, String, long, long)`: the player's club accepts an
    /// investor; the money goes into the game's cash now, under "Multas/outros", and the yearly
    /// injection is paid by `Investidor` from the next season on.
    pub fn sell_human_club(&mut self, club: ClubId, investor: (u16, u16), now: i64, yearly: i64) {
        let year = year(self.world);
        let c = self.club(club);
        c.has_owner = true;
        c.owner_since = year;
        c.owner_name = Some(investor);
        c.owner_funding = yearly;
        c.project_until = 0;
        self.world.club_income(club, now.min(i32::MAX as i64) as i32, 9);
        let name = self.world.club(club).name.clone();
        self.state().events.push(OwnerEvent { year, club: name, arrived: true });
    }

    /// `Economia.donoSai(ah)`: no more yearly injections; an AI club also loses half its money.
    pub fn owner_leaves(&mut self, club: ClubId) {
        let human = is_human(self.world, club);
        let c = self.club(club);
        c.has_owner = false;
        c.owner_funding = 0;
        c.owner_name = None;
        if !human {
            c.fortune /= 2;
        }
        let (year, name) = (year(self.world), self.world.club(club).name.clone());
        self.state().events.push(OwnerEvent { year, club: name, arrived: false });
    }

    /// `Economia.contabilizarTransferencias()`: charges every transfer of the game's history
    /// not charged yet, newest first back to last year.
    pub fn account_transfers(&mut self) {
        if self.world.mods.economy.is_none() {
            return;
        }
        let clubs = clubs_by_id(self.world);
        let humans = humans(self.world);
        let national = national_teams(self.world);
        let first_year = year(self.world) - 1;
        for i in (0..self.world.game.transfers.len()).rev() {
            let t = &self.world.game.transfers[i];
            if t.year < first_year {
                break;
            }
            let sig = signature(self.world, t);
            let processed = &mut self.world.mods.economy.as_mut().expect("started").processed;
            if !processed.insert(sig) {
                continue;
            }
            let t = &self.world.game.transfers[i];
            let fee = t.fee as i64;
            let (from, to) = (clubs.get(&t.from_club).copied(), clubs.get(&t.to_club).copied());
            if let Some(to) = to
                && !humans.contains(&self.world.club(to).id)
                && !national.contains(&to)
            {
                self.club(to).fortune -= fee;
            }
            if let Some(from) = from
                && !humans.contains(&self.world.club(from).id)
                && !national.contains(&from)
            {
                self.club(from).fortune += fee;
            }
            if let Some(from) = from
                && fee > 0
            {
                let player = self.world.game.transfers[i].player.map(|p| self.world.player(p).name.clone());
                self.satellite_sell_on(from, to, fee, player);
                self.withhold_income(from, fee, crate::credit::IncomeSource::PlayerSale);
            }
        }
        self.state().processed.retain(|s| s.split('|').next().and_then(|y| y.parse::<i32>().ok()).is_none_or(|y| y >= first_year));
    }

    /// `Economia.sincronizar()`: the fortunes into the game's cash; a club the player just left
    /// goes on with the money he left.
    fn sync_cash(&mut self) {
        let before = self.sys.humans_before.take().unwrap_or_default();
        for t in ai_clubs(self.world) {
            let id = self.world.club(t).id;
            if before.contains(&id) {
                let cash = self.world.club(t).cash;
                self.club(t).fortune = cash;
            }
            let fortune = self.club(t).fortune;
            self.world.club_mut(t).cash = fortune;
        }
        self.sys.humans_before = Some(humans(self.world));
    }

    /// `Economia.clube(ah)`: the club's figures, created on first use.
    pub fn club(&mut self, club: ClubId) -> &mut EconomyClub {
        self.ensure_economy();
        let id = self.world.club(club).id;
        if self.state().club(id).is_none() {
            let fortune = self.initial_fortune(club);
            self.state().clubs.push(EconomyClub { id, fortune, ..Default::default() });
        }
        self.state().club_mut(id).expect("just added")
    }

    /// `Economia.fortuna(ah)`: game cash for human clubs, the mod's figure for the AI.
    pub fn fortune(&mut self, club: ClubId) -> i64 {
        if is_human(self.world, club) {
            return self.world.club(club).cash;
        }
        self.club(club).fortune
    }

    /// `Economia.estimativa(ah)`: the press's figure of an AI club's money, within 20% of the
    /// truth, fixed per club, year and save.
    pub fn estimate(&mut self, club: ClubId) -> i64 {
        if is_human(self.world, club) {
            return self.world.club(club).cash;
        }
        use brasfoot_model::java_hash::JavaHash;
        let save = self.world.game.save_name.as_ref().map(|s| s.java_hash() as i64).unwrap_or(0);
        let seed = 31i64.wrapping_mul(self.world.club(club).id as i64).wrapping_add(7919 * year(self.world) as i64).wrapping_add(save);
        let error = 0.8 + brasfoot_model::rng::JavaRandom::new(seed).next_double() * 0.4;
        java_round(self.fortune(club) as f64 * error)
    }

    /// `Economia.patrocinador(ah)`'s yearly amount: the human's master sponsor, or an AI club's
    /// deal; none without a sponsor.
    pub fn club_sponsorship(&mut self, club: ClubId) -> Option<i64> {
        if is_human(self.world, club) {
            return self.world.mods.master_sponsor.active().map(|k| k.yearly);
        }
        let c = self.club(club);
        c.brand.map(|_| c.sponsorship)
    }

    /// `Economia.estimativaPatrocinio(ah, long)`: the press's guess of a deal, within 20%, fixed
    /// per club and season.
    pub fn sponsorship_estimate(&mut self, club: ClubId, value: i64) -> i64 {
        if is_human(self.world, club) {
            return value;
        }
        use brasfoot_model::java_hash::JavaHash;
        let save = self.world.game.save_name.as_ref().map(|s| s.java_hash() as i64).unwrap_or(0);
        let seed = 97i64.wrapping_mul(self.world.club(club).id as i64).wrapping_add(1009 * year(self.world) as i64).wrapping_add(save).wrapping_add(13);
        java_round(value as f64 * (0.8 + brasfoot_model::rng::JavaRandom::new(seed).next_double() * 0.4))
    }

    /// `Economia.gastar(ah, long)`: an AI club's spending.
    pub fn spend(&mut self, club: ClubId, value: i64) {
        if !is_human(self.world, club) {
            self.club(club).fortune -= value;
        }
    }

    /// `Economia.temDono(ah)`
    pub fn has_owner(&mut self, club: ClubId) -> bool {
        self.club(club).has_owner
    }

    /// `Economia.emProjeto(ah)`: a new owner's spending spree.
    pub fn in_project(&mut self, club: ClubId) -> bool {
        let year = year(self.world);
        !is_human(self.world, club) && self.club(club).project_until >= year
    }
}
