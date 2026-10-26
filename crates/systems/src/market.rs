// Port of bfmod.Mercado: how the AI judges players (valor, escala, the starters' averages), what
// it decides about the game's own yearly market (compradorDoJogo, aprovarVendaDoJogo) and how a
// bloated AI squad sheds players during a window (enxugar, abrirVaga). The AI's own signings of
// the windows (planejar, contratarUm) come with the market switched on.

use std::collections::{HashMap, HashSet};

use brasfoot_engine::hooks::NoSystems;
use brasfoot_engine::tactics::best_value;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;

use brasfoot_model::mods::SatellitePolicy;
use crate::config::MarketIntensity;
use crate::economy::{MI, ai_clubs, is_human, is_national_team};
use crate::{Mod, ModSystems, sorte};

/// `Mercado.TITULARES`: starters per position.
pub const STARTERS: [usize; 5] = [1, 2, 2, 3, 3];
const AUCTION_MINIMUM: i64 = 15 * MI;
/// `FORTUNA_COMPRADOR`: the fortune from which an AI club shops.
const BUYER_FORTUNE: i64 = 60 * MI;
const BIG_SPENDER: i64 = 300 * MI;
const MAGNATE: i64 = 650 * MI;
/// `ELENCO_ALVO`: the squad size AI clubs aim for; above it they shed players in a window.
pub const SQUAD_TARGET: usize = 28;
/// `ELENCO_MINIMO`
const SQUAD_MINIMUM: usize = 24;
/// `Motor.RAZAO_NORMAL`: a player is worth about this share of his strength on the pitch.
pub const NORMAL_RATIO: f64 = 0.66;

/// The day-long memory of the market (`movidosHoje`, `pendentesHoje`, `leiloesHoje`).
#[derive(Debug, Default)]
pub struct MarketDay {
    day: Option<(i32, i32, i32)>,
    moved: HashSet<PlayerId>,
    pending: HashMap<ClubId, i64>,
    auctions: HashMap<ClubId, i32>,
}

/// `Mercado.valor(F)`: the player at his best slot, as the engine sees him (`Motor.melhorValor`).
pub fn value(world: &World, p: PlayerId) -> i32 {
    best_value(world, p)
}

/// `Mercado.escala(int)`: a threshold in strength points on the scale of [`value`].
pub fn scale(world: &World, points: i32) -> i32 {
    if world.game.individual_skills { (points as f64 * NORMAL_RATIO).round() as i32 } else { points }
}

/// `Mercado.mediaTitulares(ah, int)`: the average value of the best `TITULARES[pos]` (loans
/// count; a missing starter counts 0).
pub fn starters_average(world: &World, club: ClubId, position: usize) -> f64 {
    let mut values: Vec<i32> = world.club(club).squad.iter().filter(|&&p| world.player(p).position as usize == position).map(|&p| value(world, p)).collect();
    values.sort_by(|a, b| b.cmp(a));
    let n = STARTERS[position];
    (0..n).fold(0.0, |sum, i| sum + values.get(i).copied().unwrap_or(0) as f64) / n as f64
}

/// `Mercado.mediaTime(ah)`: the starters' average over the eleven.
pub fn team_average(world: &World, club: ClubId) -> f64 {
    let mut sum = 0.0;
    let mut n = 0;
    for position in 0..STARTERS.len() {
        sum += starters_average(world, club, position) * STARTERS[position] as f64;
        n += STARTERS[position];
    }
    sum / n as f64
}

/// `Mercado.piorTitular(ah, int)`: the value of the weakest starter of the position, loans out.
pub fn worst_starter(world: &World, club: ClubId, position: usize) -> i32 {
    let mut values: Vec<i32> = world
        .club(club)
        .squad
        .iter()
        .filter(|&&p| world.player(p).position as usize == position && !world.player(p).on_loan)
        .map(|&p| value(world, p))
        .collect();
    values.sort_by(|a, b| b.cmp(a));
    values.get(STARTERS[position] - 1).copied().unwrap_or(0)
}

/// `Mercado.valorElenco(F)`: what keeps a player in the squad: screen strength protects stars,
/// age sends the old reserves first.
fn squad_value(world: &World, p: PlayerId) -> i32 {
    let x = world.player(p);
    let mut base = value(world, p).max(scale(world, x.strength) - scale(world, 3));
    if x.age >= 32 {
        base -= scale(world, 3);
    } else if x.age <= 22 {
        base += scale(world, 2);
    }
    base
}

impl ModSystems {
    /// `Mercado.novoDia()`: the memory is kept for one calendar day.
    fn market_day(&mut self, world: &World, clock: &brasfoot_model::clock::Clock) {
        let f = clock.fields(world.today_millis());
        let today = (f.year, f.month, f.day);
        if self.market.day != Some(today) {
            self.market.moved.clear();
            self.market.pending.clear();
            self.market.auctions.clear();
            self.market.day = Some(today);
        }
    }

    /// `Mercado.compradorDoJogo(F, ah)`: a valuable sale of the game's yearly market is first
    /// offered to the richest clubs that would start him; one of the first three takes him.
    pub fn game_buyer(&mut self, world: &mut World, runtime: &mut brasfoot_model::runtime::Runtime, player: PlayerId, drawn: Option<ClubId>) -> Option<ClubId> {
        let price = world.player(player).market_value as i64;
        if drawn.is_none() || price < AUCTION_MINIMUM || self.config.market() == MarketIntensity::Off {
            return drawn;
        }
        let x = world.player(player);
        let Some(origin) = x.club else { return drawn };
        if is_human(world, origin) || is_national_team(world, origin) || x.age < 18 || x.age > 31 || x.on_loan {
            return drawn;
        }
        self.market_day(world, &runtime.clock);
        if self.market.moved.contains(&player) {
            return drawn;
        }
        let mut m = Mod { world, runtime, sys: self };
        let worth = value(m.world, player);
        let position = m.world.player(player).position as usize;
        let mut interested = Vec::new();
        for t in m.buyers() {
            if t == origin || m.world.club(t).squad.len() >= SQUAD_TARGET + 2 || m.sys.market.auctions.get(&t).copied().unwrap_or(0) >= m.auction_limit(t) {
                continue;
            }
            // spare money only: the price may take at most half of what is left today
            if m.fortune(t) + m.sys.market.pending.get(&t).copied().unwrap_or(0) < price * 2 {
                continue;
            }
            if !m.seller_accepts(player, t, origin) || !m.accepts_offer(player, t, origin) {
                continue;
            }
            if worth <= worst_starter(m.world, t, position) + scale(m.world, 1) {
                continue;
            }
            interested.push(t);
            if interested.len() == 3 {
                break;
            }
        }
        if interested.is_empty() {
            return drawn;
        }
        let winner = interested[sorte!(m, "Mercado").next_int_bounded(interested.len() as i32) as usize];
        *m.sys.market.auctions.entry(winner).or_default() += 1;
        Some(winner)
    }

    /// `Mercado.aprovarVendaDoJogo(F, ah, int)`
    pub fn approve_game_sale(&mut self, world: &mut World, runtime: &mut brasfoot_model::runtime::Runtime, player: PlayerId, buyer: ClubId, fee: i32) -> bool {
        if crate::credit::transfer_ban_in(world, &self.config, buyer) {
            return false;
        }
        self.market_day(world, &runtime.clock);
        if self.market.moved.contains(&player) {
            return false;
        }
        let mut m = Mod { world, runtime, sys: self };
        let origin = m.world.player(player).club;
        let fee = fee as i64;
        let buyer_ai = !is_human(m.world, buyer) && !is_national_team(m.world, buyer);
        if buyer_ai && m.fortune(buyer) + m.sys.market.pending.get(&buyer).copied().unwrap_or(0) < fee {
            return false;
        }
        let origin_ai = origin.is_some_and(|o| !is_human(m.world, o) && !is_national_team(m.world, o));
        if let Some(o) = origin
            && origin_ai
        {
            let fortune = m.fortune(o);
            let rich = fortune >= 240 * MI || (m.has_owner(o) && fortune >= 140 * MI);
            if rich && starter_in_squad(m.world, player, o) {
                return false;
            }
        }
        if let Some(o) = origin
            && m.world.player(player).strength >= 65
            && m.world.club(buyer).reputation < m.world.club(o).reputation
            && !m.accepts_offer(player, buyer, o)
        {
            return false;
        }
        self.market.moved.insert(player);
        if buyer_ai {
            *self.market.pending.entry(buyer).or_default() -= fee;
        }
        if let Some(o) = origin
            && origin_ai
        {
            *self.market.pending.entry(o).or_default() += fee;
        }
        true
    }
}

/// `Mercado.titularNoElenco(F, ah)`: the player is among the club's starters at his position.
fn starter_in_squad(world: &World, player: PlayerId, club: ClubId) -> bool {
    let p = world.player(player);
    let position = p.position as usize;
    if position >= STARTERS.len() {
        return false;
    }
    let value = value(world, player);
    let better = world
        .club(club)
        .squad
        .iter()
        .filter(|&&o| {
            let x = world.player(o);
            if o == player || x.position != p.position || x.on_loan {
                return false;
            }
            let v = best_value(world, o);
            v > value || (v == value && x.strength > p.strength)
        })
        .count();
    better < STARTERS[position]
}

impl Mod<'_> {
    /// `Mercado.aceitaProposta(F, ah, ah)`: a good player steps down only for money.
    fn accepts_offer(&mut self, player: PlayerId, buyer: ClubId, owner: ClubId) -> bool {
        let (buyer_reputation, owner_reputation) = (self.world.club(buyer).reputation, self.world.club(owner).reputation);
        if buyer_reputation >= owner_reputation || self.fortune(owner) < 0 {
            return true;
        }
        let gap = owner_reputation - buyer_reputation;
        let fortune = self.fortune(buyer);
        let rich_or_owned = self.has_owner(buyer) || fortune >= BIG_SPENDER;
        let age = self.world.player(player).age;
        if self.in_project(buyer) && age >= 20 && gap <= 2 {
            return true;
        }
        if rich_or_owned && gap == 1 && age >= 21 {
            return true;
        }
        if (fortune >= MAGNATE || (self.has_owner(buyer) && fortune >= 350 * MI)) && gap <= 2 && age >= 23 {
            return true;
        }
        if rich_or_owned && age >= 27 {
            return true;
        }
        age >= 29 && fortune >= 120 * MI
    }

    /// `Mercado.planoDeProjeto(ah)`: a new owner's spree, 4 to 6 signings with up to 80% of the
    /// fortune: {club id, signings, budget}.
    pub fn project_plan(&mut self, club: ClubId) -> (i32, i64, i64) {
        let signings = 4 + sorte!(self, "Mercado").next_int_bounded(3) as i64;
        let budget = brasfoot_model::rng::java_round(self.fortune(club) as f64 * 0.8);
        (self.world.club(club).id, signings, budget)
    }

    /// `Mercado.compradores()`: the AI clubs rich enough to shop, richest first.
    pub fn buyers(&mut self) -> Vec<ClubId> {
        let mut out: Vec<(ClubId, i64)> = Vec::new();
        for t in ai_clubs(self.world) {
            if self.transfer_ban(t) {
                continue;
            }
            let policy = self.satellite_policy(t);
            if policy == Some(SatellitePolicy::Profit) {
                continue;
            }
            let minimum = if policy == Some(SatellitePolicy::Promotion) { 25 * MI } else { BUYER_FORTUNE };
            let fortune = self.fortune(t);
            if fortune >= minimum {
                out.push((t, fortune));
            }
        }
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out.into_iter().map(|(t, _)| t).collect()
    }

    /// `Mercado.propostasParaHumanos(int)`: the rich clubs' offers for the human clubs'
    /// starters, at most `limit`.
    pub(crate) fn offers_for_humans(&mut self, limit: usize) -> Vec<crate::windows::SquadOffer> {
        let buyers = self.buyers();
        let mut offers = Vec::new();
        for coach in self.world.game.human_coaches.clone() {
            let Some(mine) = self.world.coach(coach).club_id else { continue };
            let world = &*self.world;
            let mut squad: Vec<PlayerId> = world.club(mine).squad.iter().copied().filter(|&p| !world.player(p).on_loan).collect();
            squad.sort_by(|&a, &b| value(world, b).cmp(&value(world, a)));
            squad.truncate(11);
            sorte!(self, "Mercado").shuffle(&mut squad);
            let mut made = 0;
            for p in squad {
                if made >= 2 {
                    break;
                }
                if sorte!(self, "Mercado").next_int_bounded(100) >= 35 {
                    continue;
                }
                if let Some(offer) = self.offer_for(p, &buyers) {
                    offers.push(offer);
                    made += 1;
                }
            }
        }
        offers.truncate(limit);
        offers
    }

    /// `Mercado.proposta(F, List)`: the richest buyer that needs the player and can pay.
    fn offer_for(&mut self, p: PlayerId, buyers: &[ClubId]) -> Option<crate::windows::SquadOffer> {
        for &t in buyers {
            if self.world.club(t).squad.len() >= 30 {
                continue;
            }
            let position = self.world.player(p).position as usize;
            if (value(self.world, p) as f64) < starters_average(self.world, t, position) + scale(self.world, 2) as f64 {
                continue;
            }
            let fortune = self.fortune(t);
            let wealth = (fortune as f64 / (800.0 * MI as f64)).min(1.0);
            let amount = brasfoot_model::rng::java_round(self.world.player(p).market_value as f64 * (1.2 + 0.6 * wealth));
            if amount as f64 > fortune as f64 * 0.6 {
                continue;
            }
            return Some(crate::windows::SquadOffer { player: p, buyer: t, value: amount, loan: false });
        }
        None
    }

    /// `Mercado.limiteDeLeiloes(ah)`: auction wins a day.
    fn auction_limit(&mut self, club: ClubId) -> i32 {
        let fortune = self.fortune(club);
        if fortune >= MAGNATE {
            5
        } else if fortune >= BIG_SPENDER {
            4
        } else if fortune >= 180 * MI {
            3
        } else {
            2
        }
    }

    /// `Mercado.vendedorAceita(F, ah, ah)`: rich clubs keep their starters; the others sell to
    /// clubs richer than them.
    fn seller_accepts(&mut self, player: PlayerId, buyer: ClubId, owner: ClubId) -> bool {
        let seller_fortune = self.fortune(owner);
        if seller_fortune < 0 {
            return true;
        }
        let rich_seller = seller_fortune >= 240 * MI || (self.has_owner(owner) && seller_fortune >= 140 * MI);
        if rich_seller && starter_in_squad(self.world, player, owner) {
            return false;
        }
        let buyer_fortune = self.fortune(buyer);
        // ah.kd(): the captain
        if seller_fortune >= 140 * MI && self.world.club(owner).captain == Some(player) {
            return false;
        }
        if self.in_project(buyer) || buyer_fortune >= MAGNATE || (self.has_owner(buyer) && buyer_fortune >= BIG_SPENDER) {
            return !rich_seller || seller_fortune as f64 <= buyer_fortune as f64 * 1.15;
        }
        if buyer_fortune >= BIG_SPENDER {
            return seller_fortune as f64 <= buyer_fortune as f64 * 0.90;
        }
        seller_fortune as f64 <= buyer_fortune as f64 * 0.80
    }

    /// `Mercado.preco(F, ah, ah)`: the seller's price over the market value, more for bigger
    /// sellers, a bargain for clubs in crisis; the draw is fixed per player and year.
    pub fn price(&mut self, player: PlayerId, seller: ClubId, buyer: Option<ClubId>) -> i64 {
        use brasfoot_model::java_hash::JavaHash;
        let x = self.world.player(player);
        let seed = (x.name.java_hash() as i64).wrapping_mul(31).wrapping_add(crate::economy::year(self.world) as i64);
        let mut factor = 1.15 + self.world.club(seller).reputation.max(0) as f64 * 0.05 + brasfoot_model::rng::JavaRandom::new(seed).next_double() * 0.35;
        let value = x.market_value;
        if self.fortune(seller) < 0 {
            factor *= 0.8;
        } else if let Some(b) = buyer {
            if self.world.club(b).reputation < self.world.club(seller).reputation {
                factor *= 1.20;
            }
            if self.fortune(seller) as f64 > self.fortune(b) as f64 * 0.75 {
                factor *= 1.15;
            }
        }
        brasfoot_model::rng::java_round(value as f64 * factor)
    }

    /// `Mercado.planejar(int)`: how many signings and how much money each buyer brings to a
    /// window: {club id, signings, budget}.
    pub fn plan_window(&mut self, number: i32) -> Vec<(i32, i64, i64)> {
        let intensity = self.sys.config.market();
        if intensity == MarketIntensity::Off {
            return Vec::new();
        }
        self.update_reputations();
        let mut out = Vec::new();
        for buyer in self.buyers() {
            let fortune = self.fortune(buyer);
            if self.in_project(buyer) {
                out.push(self.project_plan(buyer));
                continue;
            }
            let signings = self.how_many(buyer, fortune, number, intensity);
            if signings > 0 {
                let aggressive = fortune >= MAGNATE || self.has_owner(buyer) || self.satellite_policy(buyer) == Some(SatellitePolicy::Promotion);
                let share = if number == 1 {
                    if aggressive { 0.72 } else if fortune >= BIG_SPENDER { 0.65 } else { 0.60 }
                } else if aggressive {
                    0.45
                } else if fortune >= BIG_SPENDER {
                    0.38
                } else {
                    0.30
                };
                out.push((self.world.club(buyer).id, signings as i64, brasfoot_model::rng::java_round(fortune as f64 * share)));
            }
        }
        out
    }

    /// `Mercado.quantas(ah, long, int, Config.Mercado)`
    fn how_many(&mut self, buyer: ClubId, fortune: i64, window: i32, intensity: MarketIntensity) -> i32 {
        match self.satellite_policy(buyer) {
            Some(SatellitePolicy::Profit) => return 0,
            Some(SatellitePolicy::Promotion) => {
                let base = if window == 1 { 2 } else { 1 };
                return base + sorte!(self, "Mercado").next_int_bounded(2);
            }
            _ => {}
        }
        if intensity == MarketIntensity::Contained {
            return if sorte!(self, "Mercado").next_int_bounded(100) < (if window == 1 { 50 } else { 20 }) { 1 } else { 0 };
        }
        let owner = self.has_owner(buyer);
        let magnate = fortune >= MAGNATE || (owner && fortune >= 350 * MI);
        let spender = magnate || fortune >= BIG_SPENDER || (owner && fortune >= 160 * MI);
        let behind = spender && (team_average(self.world, buyer) < scale(self.world, 70) as f64 || worst_starter_overall(self.world, buyer) < scale(self.world, 64));
        let draw = |m: &mut Self, n: i32| sorte!(m, "Mercado").next_int_bounded(n);
        if intensity == MarketIntensity::Busy {
            if magnate {
                return if window == 2 { 2 + draw(self, 3) } else { 4 + draw(self, 3) + behind as i32 };
            }
            if spender {
                return if window == 2 { 1 + draw(self, 3) } else { 3 + draw(self, 3) };
            }
            return if window == 2 { draw(self, 3) } else { 1 + draw(self, if fortune >= 150 * MI { 4 } else { 3 }) };
        }
        if window == 2 {
            if magnate {
                return (if behind { 2 } else { 1 }) + draw(self, 2);
            }
            if spender {
                return 1 + (behind && sorte!(self, "Mercado").next_boolean()) as i32;
            }
            return if draw(self, 100) < (if fortune >= 150 * MI { 60 } else { 35 }) { 1 } else { 0 };
        }
        if magnate {
            return (if behind { 4 } else { 3 }) + draw(self, 3);
        }
        if spender {
            return (if behind { 3 } else { 2 }) + draw(self, 3);
        }
        if fortune >= 150 * MI {
            return 1 + draw(self, 3);
        }
        draw(self, 2)
    }

    /// `Mercado.contratarUm(ah, long, Set)`: one signing within the budget (the news of a big one
    /// is not ported): (player, from, fee).
    pub fn sign_one(&mut self, buyer: ClubId, budget: i64) -> Option<(PlayerId, ClubId, i64)> {
        // a full squad sells its worst benched players to make room
        for _ in 0..3 {
            if self.world.club(buyer).squad.len() < SQUAD_TARGET || !self.make_room(buyer) {
                break;
            }
        }
        if self.world.club(buyer).squad.len() >= SQUAD_TARGET + 2 {
            return None;
        }
        let buyer_fortune = self.fortune(buyer);
        let top = buyer_fortune >= BIG_SPENDER || self.has_owner(buyer) || self.world.club(buyer).reputation >= 5;
        let max_age = if self.satellite_policy(buyer) == Some(SatellitePolicy::YoungPlayers) { 23 } else { 31 };
        let team = team_average(self.world, buyer);
        let positions = self.urgency(buyer);
        let min_strength = if top { 74 } else { 60 };
        let mut min_upgrade = [0i32; 5];
        let mut min_ambitious = [0i32; 5];
        let mut aging = [false; 5];
        for pos in 0..5 {
            let worst = worst_starter(self.world, buyer, pos);
            let average = starters_average(self.world, buyer, pos);
            min_upgrade[pos] = scale(self.world, 54).max(worst + scale(self.world, 2));
            let s1 = scale(self.world, 1) as f64;
            let s2 = scale(self.world, 2) as f64;
            min_ambitious[pos] = ((min_upgrade[pos] + scale(self.world, 2)) as f64).max((scale(self.world, 71) as f64).min((average + s1).max(team - s2))) as i32;
            aging[pos] = starters_average_age(self.world, buyer, pos) >= 29.5;
        }
        let mut pool: [Vec<PlayerId>; 5] = Default::default();
        for i in 0..self.world.game.players.len() {
            let p = self.world.game.players[i];
            let x = self.world.player(p);
            let pos = x.position;
            if !(0..5).contains(&pos) || x.age < 18 || x.age > max_age || x.strength < min_strength || x.on_loan {
                continue;
            }
            if x.market_value as i64 > budget {
                continue;
            }
            let Some(owner) = x.club else { continue };
            if owner == buyer || is_national_team(self.world, owner) || is_human(self.world, owner) || self.world.club(owner).squad.len() <= 18 || self.sys.traded.contains(&p) {
                continue;
            }
            if value(self.world, p) < min_upgrade[pos as usize] {
                continue;
            }
            if !self.accepts_offer(p, buyer, owner) || !self.seller_accepts(p, buyer, owner) {
                continue;
            }
            if self.price(p, owner, Some(buyer)) > budget {
                continue;
            }
            pool[pos as usize].push(p);
        }
        for mode in 0..2 {
            for &position in &positions {
                let worst = worst_starter(self.world, buyer, position);
                if mode == 1 && worst >= scale(self.world, 72) {
                    continue;
                }
                let minimum = if mode == 0 { min_ambitious[position] } else { min_upgrade[position] };
                let mut candidates = filter_candidates(self.world, &pool[position], minimum, aging[position]);
                if candidates.is_empty() && aging[position] {
                    candidates = filter_candidates(self.world, &pool[position], minimum, false);
                }
                if candidates.is_empty() {
                    continue;
                }
                candidates.sort_by(|&a, &b| signing_score(self.world, b, top).cmp(&signing_score(self.world, a, top)));
                let limit = if top { 2 } else if buyer_fortune >= 150 * MI { 4 } else { 7 };
                let n = limit.min(candidates.len());
                let chosen = candidates[sorte!(self, "Mercado").next_int_bounded(n as i32) as usize];
                let from = self.world.player(chosen).club.expect("a candidate has a club");
                let fee = self.price(chosen, from, Some(buyer));
                self.move_player(chosen, buyer, fee.min(i32::MAX as i64) as i32);
                // into the books right away, so the next deal of the day sees the real cash
                self.account_transfers();
                return Some((chosen, from, fee));
            }
        }
        None
    }

    /// `Mercado.posicoesPorUrgencia(ah)`: the five positions, the weakest first (with a little
    /// noise).
    pub(crate) fn urgency(&mut self, club: ClubId) -> Vec<usize> {
        let mut score = [0.0f64; 5];
        for (pos, s) in score.iter_mut().enumerate() {
            let base = 0.65 * worst_starter(self.world, club, pos) as f64 + 0.35 * starters_average(self.world, club, pos);
            *s = base + sorte!(self, "Mercado").next_double() * 1.5;
        }
        let mut order: Vec<usize> = (0..5).collect();
        order.sort_by(|&a, &b| score[a].total_cmp(&score[b]));
        order
    }

    /// `Mercado.enxugar(ah, int, Set)`: up to `limit` departures from a bloated AI squad.
    pub fn trim_squad(&mut self, club: ClubId, limit: usize) -> usize {
        let mut out = 0;
        while out < limit && self.world.club(club).squad.len() > SQUAD_TARGET {
            if !self.make_room(club) {
                break;
            }
            out += 1;
        }
        out
    }

    /// `Mercado.abrirVaga(ah, Set)`: the weakest player outside the 20 best leaves for a
    /// smaller AI club: sold if it can pay, lent if he is young, else free; from 26 on, with
    /// nobody wanting him, he retires. Positions keep one more than their starters.
    fn make_room(&mut self, club: ClubId) -> bool {
        let world = &*self.world;
        let squad = world.club(club).squad.clone();
        let mut per_position = [0usize; 5];
        for &p in &squad {
            let x = world.player(p);
            if (0..5).contains(&x.position) && !x.on_loan {
                per_position[x.position as usize] += 1;
            }
        }
        if squad.len() <= SQUAD_MINIMUM {
            return false;
        }
        let mut sorted = squad;
        sorted.sort_by(|&a, &b| squad_value(world, b).cmp(&squad_value(world, a)));
        let mut benched: Vec<PlayerId> = sorted[20.min(sorted.len())..].to_vec();
        benched.sort_by(|&a, &b| squad_value(world, a).cmp(&squad_value(world, b)));
        for p in benched {
            let x = self.world.player(p);
            let position = x.position;
            if x.on_loan || self.sys.traded.contains(&p) {
                continue;
            }
            if (0..5).contains(&position) && per_position[position as usize] <= STARTERS[position as usize] + 1 {
                continue;
            }
            if self.absorb_surplus(club, p) {
                return true;
            }
            let reputation = self.world.club(club).reputation;
            let country = self.world.club(club).country;
            let worth = value(self.world, p);
            let margin = scale(self.world, 8) as f64;
            let mut destinations: Vec<ClubId> = ai_clubs(self.world)
                .into_iter()
                .filter(|&t| {
                    let c = self.world.club(t);
                    t != club && c.squad.len() < SQUAD_TARGET && c.reputation <= reputation && worth as f64 >= starters_average(self.world, t, position as usize) - margin
                })
                .collect();
            sorte!(self, "Mercado").shuffle(&mut destinations);
            destinations.sort_by_key(|&t| self.world.club(t).country != country);
            let price = self.world.player(p).market_value;
            for &d in &destinations {
                if self.fortune(d) >= price as i64 {
                    self.move_player(p, d, price);
                    self.account_transfers();
                    return true;
                }
            }
            if self.world.player(p).age <= 23 && !destinations.is_empty() {
                self.lend(p, destinations[0]);
                self.sys.traded.insert(p);
                return true;
            }
            if let Some(&d) = destinations.iter().find(|&&d| self.world.club(d).squad.len() < SQUAD_TARGET) {
                self.move_player(p, d, 0);
                return true;
            }
            if self.world.player(p).age >= 26 {
                self.sys.traded.insert(p);
                Sim { world: self.world, runtime: self.runtime, hooks: &mut NoSystems::default(), human: None }.retire_by_request(p, None, -1);
                return true;
            }
        }
        false
    }

    /// `F.a(ah, int, false, false, false)` by the mod, and the player marked as moved.
    fn move_player(&mut self, p: PlayerId, to: ClubId, fee: i32) {
        Sim { world: self.world, runtime: self.runtime, hooks: &mut NoSystems::default(), human: None }.transfer(p, to, fee);
        self.sys.traded.insert(p);
    }
}

/// `Mercado.piorTitularGeral(ah)`: the weakest starter of all positions.
fn worst_starter_overall(world: &World, club: ClubId) -> i32 {
    (0..5).map(|p| worst_starter(world, club, p)).min().unwrap_or(0)
}

/// `Mercado.idadeMediaTitulares(ah, int)`: the starters' average age (25 when there are none).
fn starters_average_age(world: &World, club: ClubId, position: usize) -> f64 {
    let mut players: Vec<PlayerId> = world.club(club).squad.iter().copied().filter(|&p| world.player(p).position as usize == position && !world.player(p).on_loan).collect();
    players.sort_by(|&a, &b| value(world, b).cmp(&value(world, a)));
    let n = STARTERS[position].min(players.len());
    if n == 0 {
        return 25.0;
    }
    players[..n].iter().fold(0.0, |s, &p| s + world.player(p).age as f64) / n as f64
}

/// `Mercado.filtrarCandidatos(List, int, boolean)`: under 29 when the starters are old.
fn filter_candidates(world: &World, list: &[PlayerId], minimum: i32, avoid_thirties: bool) -> Vec<PlayerId> {
    let max_age = if avoid_thirties { 28 } else { 31 };
    list.iter().copied().filter(|&p| world.player(p).age <= max_age && value(world, p) >= minimum).collect()
}

/// `Mercado.notaContratacao(F, boolean)`: the engine's value, the screen strength and, for the
/// big clubs, the prime age.
fn signing_score(world: &World, p: PlayerId, top: bool) -> i32 {
    let x = world.player(p);
    let mut v = value(world, p) * 14 + scale(world, x.strength);
    if top {
        v += match x.age {
            20..=26 => 16,
            27..=28 => 8,
            30 => -12,
            a if a >= 31 => -24,
            _ => 0,
        };
    }
    v
}
