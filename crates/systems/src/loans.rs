// Port of bfmod.Emprestimos: the AI clubs lend their benched youngsters (21 or under) to smaller
// clubs where they would start, for a fee (a share of the market value, free inside a satellite
// network), and a borrower whose loanee plays against his owner pays a fine. The loans are the
// game's own (F.q), which bring the players back after a year (f.au).

use std::collections::HashSet;

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::LoansState;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;

use crate::economy::{ai_clubs, is_human};
use crate::market::{starters_average, value};
use crate::{Mod, sorte};

/// `Emprestimos.IDADE_MAX`
const MAX_AGE: i32 = 21;
/// `Emprestimos.TAXA`: the fee, as a share of the market value.
const FEE: f64 = 0.10;
/// `Emprestimos.MULTA_TAXAS`: the fine for playing against the owner, in fees.
const FINE_FEES: i64 = 2;

/// `Emprestimos.emprestimoDe(F)`: the game's loan record of the player, if he is on loan.
pub fn loan_of(world: &World, p: PlayerId) -> Option<usize> {
    world.game.loans.iter().position(|l| l.player == Some(p))
}

/// `Emprestimos.temporada(F)`: this season's games, goals and average rating.
pub fn season_numbers(world: &World, p: PlayerId) -> (i32, i32, f64) {
    let season = world.season();
    let (mut games, mut goals, mut sum, mut rated) = (0, 0, 0.0, 0);
    for s in &world.player(p).season_stats {
        if s.season == season {
            games += s.games;
            goals += s.goals;
            sum += s.rating_sum;
            rated += s.rated_games;
        }
    }
    (games, goals, if rated > 0 { sum / rated as f64 } else { 0.0 })
}

impl Mod<'_> {
    /// `F.q(ah)`: the game's loan of a year.
    pub(crate) fn lend(&mut self, p: PlayerId, to: ClubId) {
        Sim { world: self.world, runtime: self.runtime, hooks: &mut NoSystems::default(), human: None }.lend(p, to);
    }

    /// `Emprestimos.janela(Set, int)`: in a window, each big AI club (reputation 3 or more, more
    /// than 22 players) may lend one benched youngster to a smaller club.
    pub(crate) fn window_loans(&mut self, number: i32) {
        let mut clubs = ai_clubs(self.world);
        sorte!(self, "Emprestimos").shuffle(&mut clubs);
        for origin in clubs {
            let c = self.world.club(origin);
            if c.reputation < 3 || c.squad.len() <= 22 {
                continue;
            }
            let chance = if number == 1 { 60 } else { 30 };
            if sorte!(self, "Emprestimos").next_int_bounded(100) >= chance {
                continue;
            }
            let mut done = 0;
            for p in self.benched(origin) {
                if done >= 1 || self.world.club(origin).squad.len() <= 22 {
                    break;
                }
                if self.sys.traded.contains(&p) {
                    continue;
                }
                if let Some(to) = self.loan_destination(p, origin, false) {
                    let fee = self.loan_fee(p, origin, to);
                    self.lend(p, to);
                    self.pay_loan(to, origin, fee, false);
                    self.sys.traded.insert(p);
                    done += 1;
                }
            }
        }
        // Jornal.noticia: the window's loans are news only
    }

    /// `Emprestimos.pedidosParaHumanos()`: the AI clubs that ask to borrow the human clubs'
    /// benched youngsters, two at most, each with its fee.
    pub(crate) fn loan_requests_for_humans(&mut self) -> Vec<crate::windows::SquadOffer> {
        let mut out = Vec::new();
        for coach in self.world.game.human_coaches.clone() {
            let Some(mine) = self.world.coach(coach).club_id else { continue };
            let mut candidates = self.benched(mine);
            sorte!(self, "Emprestimos").shuffle(&mut candidates);
            for p in candidates {
                if out.len() >= 2 {
                    break;
                }
                let to = self.loan_destination(p, mine, true);
                if let Some(to) = to
                    && sorte!(self, "Emprestimos").next_int_bounded(100) < 50
                {
                    let fee = self.loan_fee(p, mine, to);
                    out.push(crate::windows::SquadOffer { player: p, buyer: to, value: fee, loan: true });
                }
            }
        }
        out
    }

    /// `Emprestimos.aoEmprestar(F, ah, ah, long)`: the borrower pays the fee.
    pub(crate) fn pay_loan_fee(&mut self, borrower: ClubId, owner: ClubId, fee: i64) {
        self.pay_loan(borrower, owner, fee, false);
    }

    /// `Emprestimos.encostados(ah)`: the youngsters outside the 18 best of the squad, not on loan.
    fn benched(&self, club: ClubId) -> Vec<PlayerId> {
        let world = &*self.world;
        let mut squad = world.club(club).squad.clone();
        squad.sort_by(|&a, &b| value(world, b).cmp(&value(world, a)));
        squad.into_iter().skip(18).filter(|&p| world.player(p).age <= MAX_AGE && !world.player(p).on_loan && loan_of(world, p).is_none()).collect()
    }

    /// `Emprestimos.destino(F, ah, boolean)`: a smaller AI club with room where the kid would start
    /// and that can pay twice the fee; seven in ten times one of the owner's country.
    fn loan_destination(&mut self, p: PlayerId, origin: ClubId, for_human: bool) -> Option<ClubId> {
        let mut same = Vec::new();
        let mut others = Vec::new();
        for t in ai_clubs(self.world) {
            let c = self.world.club(t);
            if t == origin || c.squad.len() >= 30 {
                continue;
            }
            if c.reputation >= self.world.club(origin).reputation && !for_human {
                continue;
            }
            let position = self.world.player(p).position as usize;
            if (value(self.world, p) as f64) < starters_average(self.world, t, position) {
                continue;
            }
            let fee = self.loan_fee(p, origin, t);
            if self.fortune(t) < 2 * fee {
                continue;
            }
            if self.world.club(t).country == self.world.club(origin).country { same.push(t) } else { others.push(t) }
        }
        let list = if !same.is_empty() && sorte!(self, "Emprestimos").next_int_bounded(100) < 70 {
            same
        } else if !others.is_empty() {
            others
        } else {
            same
        };
        if list.is_empty() {
            return None;
        }
        let i = sorte!(self, "Emprestimos").next_int_bounded(list.len() as i32) as usize;
        Some(list[i])
    }

    /// `Emprestimos.taxa(F, ah, ah)`: nothing between the clubs of one network.
    pub(crate) fn loan_fee(&mut self, p: PlayerId, owner: ClubId, to: ClubId) -> i64 {
        if self.parent_of(to) == Some(owner) || self.parent_of(owner) == Some(to) {
            return 0;
        }
        brasfoot_model::rng::java_round(self.world.player(p).market_value as f64 * FEE)
    }

    /// `Emprestimos.pagar(ah, ah, long, boolean)`: a fee (category 1) or a fine (9 for the
    /// receiver) from one club to the other.
    fn pay_loan(&mut self, from: ClubId, to: ClubId, amount: i64, fine: bool) {
        if amount <= 0 {
            return;
        }
        let v = amount.min(i32::MAX as i64) as i32;
        if is_human(self.world, from) {
            self.world.club_payment(from, v, 1);
        } else {
            self.spend(from, amount);
        }
        if is_human(self.world, to) {
            self.world.club_income(to, v, if fine { 9 } else { 1 });
        } else {
            self.spend(to, -amount);
        }
    }

    /// `Emprestimos.tick()`: once per finished match day, the borrowers whose loanee played
    /// against his owner pay the fine; a save that never ran this starts from today.
    pub fn loans_tick(&mut self) {
        let first = self.world.mods.loans.is_none();
        let state = *self.world.mods.loans.get_or_insert(LoansState { season_seen: -1, day_seen: -1 });
        let season = self.world.season();
        let mut seen = state;
        if season != seen.season_seen {
            seen.season_seen = season;
            seen.day_seen = -1;
        }
        let mut last = seen.day_seen;
        let today = self.world.game.day;
        let mut d = seen.day_seen + 1;
        while d <= today && (d as usize) < self.world.game.days.len() {
            let matches = self.world.game.days[d as usize].matches.clone();
            if !matches.iter().all(|&m| self.world.game_match(m).played) {
                break;
            }
            if !first {
                for m in matches {
                    self.fine_loanees(m);
                }
            }
            last = d;
            d += 1;
        }
        seen.day_seen = last;
        self.world.mods.loans = Some(seen);
    }

    /// `Emprestimos.multar(I)`
    fn fine_loanees(&mut self, m: brasfoot_model::ids::MatchId) {
        let g = self.world.game_match(m);
        let (Some(home), Some(away)) = (g.home, g.away) else { return };
        for i in 0..self.world.game.loans.len() {
            let loan = self.world.game.loans[i].clone();
            let (Some(p), Some(owner)) = (loan.player, loan.owner) else { continue };
            let Some(team) = self.world.player(p).club else { continue };
            if team == owner || is_human(self.world, team) {
                continue;
            }
            let against = (team == home && owner == away) || (team == away && owner == home);
            if !against || !played_in(self.world, m, p) {
                continue;
            }
            let fine = FINE_FEES * self.loan_fee(p, owner, team);
            if fine <= 0 {
                continue;
            }
            self.pay_loan(team, owner, fine, true);
            // Jornal.noticia: the fine's news is not ported
        }
    }
}

/// `Emprestimos.jogou(I, F)`: started the match or came on (a substitution, event 6, names both).
fn played_in(world: &World, m: brasfoot_model::ids::MatchId, p: PlayerId) -> bool {
    let g = world.game_match(m);
    if g.home_starters.contains(&p) || g.away_starters.contains(&p) {
        return true;
    }
    g.events.iter().any(|e| e.kind == 6 && (e.player == Some(p) || e.other_player == Some(p)))
}

/// The players the windows moved, kept by the caller.
pub type Traded = HashSet<PlayerId>;
