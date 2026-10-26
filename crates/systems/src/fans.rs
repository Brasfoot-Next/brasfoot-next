// Port of bfmod.Torcida: the fans' mood on top of the game's own "Confiança torcida" bar (which
// already reacts to results): big signings, stars leaving and academy kids breaking through move
// it too, and it drives the monthly membership income. The last reasons show as the bar's
// tooltip.

use std::collections::HashSet;

use brasfoot_model::ids::{ClubId, CoachId, PlayerId};
use brasfoot_model::java_hash::{JavaHash, JavaHashMap};
use brasfoot_model::mods::{FanEvent, FanReason};
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;

use crate::Mod;
use crate::awards::season::player_key;
use crate::config::Switch;

/// `MAX_MOTIVOS`
const MAX_REASONS: usize = 10;

/// `Torcida.tecnico()`: the first human coach.
fn coach(world: &World) -> Option<CoachId> {
    world.game.human_coaches.first().copied()
}

/// `Torcida.confianca()`: the game's fan confidence (0-100); 60 when unknown.
pub fn confidence(world: &World) -> i32 {
    coach(world).map_or(60, |c| world.coach(c).fan_support)
}

/// `Torcida.socioMensal()`: the monthly membership income, from 0 to ~12% of the monthly wage
/// bill depending on the mood, for the screens.
pub fn monthly_members(world: &World, config: &crate::config::ModConfig) -> i64 {
    let Some(club) = coach(world).and_then(|c| world.coach(c).club_id) else { return 0 };
    members_value(world, club, crate::assets::members_factor_now(world, config, club))
}

fn members_value(world: &World, club: ClubId, factor: f64) -> i64 {
    java_round(members_before_factor(world, club) * factor)
}

/// `meu.kK() * 0.06 * confianca() / 50.0`, read before `Patrimonio.fatorSocio`.
fn members_before_factor(world: &World, club: ClubId) -> f64 {
    world.club_wages(club) as f64 * 0.06 * confidence(world) as f64 / 50.0
}

/// `Emprestimos.emprestimoDe(F)`: the player is out on loan.
fn on_loan(world: &World, p: PlayerId) -> bool {
    world.game.loans.iter().any(|l| l.player == Some(p))
}

/// `Torcida.idolos(ah)`: the hashes of the keys of the club's three most capped idols.
fn idols(world: &World, club: ClubId) -> HashSet<i32> {
    let id = world.club(club).id;
    let mut list: Vec<_> = world.mods.awards.idols.iter().filter(|i| i.club == id).collect();
    list.sort_by(|a, b| b.games.cmp(&a.games));
    list.iter().take(3).map(|i| i.key.java_hash()).collect()
}

impl Mod<'_> {
    /// `Torcida.carregar()`: true on the first load of a save with no state yet.
    fn load_fans(&mut self) -> bool {
        if self.sys.fans_loaded {
            return false;
        }
        self.sys.fans_loaded = true;
        self.world.mods.fans.is_empty()
    }

    /// `Torcida.tick()`
    pub(crate) fn fans_tick(&mut self) {
        let Some(me) = coach(self.world) else { return };
        let Some(club) = self.world.coach(me).club_id else { return };
        if !self.sys.config.on(Switch::Fans) {
            return;
        }
        let first = self.load_fans();
        let mut now: JavaHashMap<String, PlayerId> = JavaHashMap::new();
        for &p in &self.world.club(club).squad {
            now.insert(player_key(self.world, p), p);
        }
        let mut academy_now: JavaHashMap<String, ()> = JavaHashMap::new();
        for &p in &self.world.game.youth_players {
            if self.world.player(p).club == Some(club) {
                academy_now.insert(player_key(self.world, p), ());
            }
        }
        let state = self.world.mods.fans.clone();
        let mut before: JavaHashMap<String, i32> = JavaHashMap::new();
        for (k, v) in &state.squad {
            before.insert(k.clone(), *v);
        }
        let academy_before: HashSet<&String> = state.academy.iter().collect();

        if !first {
            let mut strengths: Vec<i32> = before.values().copied().collect();
            strengths.sort_by(|a, b| b.cmp(a));
            let n = strengths.len().min(11);
            let top11 = strengths[..n].iter().map(|&s| s as f64).sum::<f64>() / n.max(1) as f64;
            let third = if strengths.len() >= 3 { strengths[2] } else { 999 };

            for (key, &p) in now.iter() {
                if before.contains_key(key) {
                    continue;
                }
                let player = self.world.player(p);
                let name = player.name.clone();
                if academy_before.contains(key) {
                    if crate::academy::ceiling(self.world, p).unwrap_or(-1) >= 75 {
                        self.fans_change(me, 2, FanEvent::AcademyPromotion { player: name });
                    }
                } else if player.world_class {
                    self.fans_change(me, 7, FanEvent::WorldStarSigned { player: name });
                } else if player.strength as f64 >= top11 + 3.0 {
                    self.fans_change(me, 4, FanEvent::ImpactSigning { player: name });
                }
            }

            let idols = idols(self.world, club);
            let mut everyone: Option<JavaHashMap<String, PlayerId>> = None;
            for (key, &strength) in before.iter() {
                if now.contains_key(key) {
                    continue;
                }
                let all = everyone.get_or_insert_with(|| {
                    let mut m = JavaHashMap::new();
                    for &p in &self.world.game.players {
                        m.insert(player_key(self.world, p), p);
                    }
                    m
                });
                let Some(&p) = all.get(key) else { continue };
                let Some(to) = self.world.player(p).club else { continue };
                if to == club || on_loan(self.world, p) {
                    continue;
                }
                let name = self.world.player(p).name.clone();
                if idols.contains(&key.java_hash()) {
                    let to = self.world.club(to).name.clone();
                    self.fans_change(me, -7, FanEvent::IdolSold { player: name, club: to });
                } else if strength >= third {
                    self.fans_change(me, -5, FanEvent::TopPlayerLeft { player: name });
                }
            }
        }

        let squad = now.iter().map(|(k, &p)| (k.clone(), self.world.player(p).strength)).collect();
        let fans = &mut self.world.mods.fans;
        fans.squad = squad;
        fans.academy = academy_now.iter().map(|(k, _)| k.clone()).collect();
    }

    /// `Torcida.mudar(al, int, String)`
    fn fans_change(&mut self, coach: CoachId, change: i32, why: FanEvent) {
        let c = self.world.coach_mut(coach);
        c.fan_support = (c.fan_support + change).clamp(0, 100);
        let day = self.world.today_millis();
        let reasons = &mut self.world.mods.fans.reasons;
        reasons.push(FanReason { day, change, why });
        while reasons.len() > MAX_REASONS {
            reasons.remove(0);
        }
    }

    /// `Torcida.evento(int, String)`: a mood swing caused by another part of the mod.
    pub fn fans_event(&mut self, change: i32, why: FanEvent) {
        let Some(me) = coach(self.world) else { return };
        if !self.sys.config.on(Switch::Fans) {
            return;
        }
        // the first load takes no snapshot: the next tick of the run is not a first one, so
        // every player then looks like a new signing, as in the original
        self.load_fans();
        self.fans_change(me, change, why);
    }

    /// `Torcida.pagarMes()`
    pub(crate) fn pay_fans_month(&mut self) {
        let Some(club) = crate::staff::my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Fans) {
            return;
        }
        // socioMensal(): the first human coach's club, which is meuClube()
        let before = members_before_factor(self.world, club);
        let members = java_round(before * self.members_factor(club));
        self.world.club_income(club, members.min(i32::MAX as i64) as i32, 6);
    }
}
