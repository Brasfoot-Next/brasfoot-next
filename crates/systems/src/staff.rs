// Port of bfmod.Comissao: the coaching staff the player hires (performance analyst, assistant
// coach, fitness coach, levels 1 to 5). They don't touch the match engine; their levels decide
// how deep the performance analysis goes (Desempenho) and whether the automatic tactics run.
// Salaries are paid monthly under "Diversos". New candidates every month. The staff follows the
// coach, not the club.

use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{StaffMember, StaffRole};
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;

use crate::config::Switch;
use crate::{Mod, sorte};

/// `NIVEL_MAX`
pub const MAX_LEVEL: i32 = 5;
/// `SALARIO_BASE`, by level.
const BASE_SALARY: [i64; 6] = [0, 15_000, 35_000, 70_000, 130_000, 220_000];
/// The sizes of `NOMES`, `SOBRENOMES` and `PERFIS` (the words are in the catalog).
pub const FIRST_NAMES: i32 = 30;
pub const SURNAMES: i32 = 30;
pub const PROFILES: i32 = 6;

/// `Diretor.meuClube()`: the first human coach's club.
pub fn my_club(world: &World) -> Option<ClubId> {
    world.game.human_coaches.first().and_then(|&c| world.coach(c).club_id)
}

/// `Comissao.reputacaoMinima(int)`: the lowest club reputation a professional of this level
/// accepts.
pub fn minimum_reputation(level: i32) -> i32 {
    (level - 2).max(0)
}

/// Why `Comissao.contratar` refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HireError {
    /// "Candidato indisponível."
    Unavailable,
    /// The club's reputation is below the professional's minimum.
    Reputation { minimum: i32 },
}

impl Mod<'_> {
    /// `Comissao.nivel(Cargo)`: 0 when nobody is hired; without the staff system the analysis is
    /// shown whole, as if nothing limited it.
    pub fn staff_level(&self, role: StaffRole) -> i32 {
        staff_level(self.world, &self.sys.config, role)
    }

    /// `Comissao.candidatos()`: three per role, drawn again each month.
    pub fn staff_candidates(&mut self) -> Vec<StaffMember> {
        let month = self.today_date() / 100;
        if self.world.mods.staff.candidates.is_empty() || month != self.world.mods.staff.candidates_month {
            self.draw_candidates(month);
        }
        self.world.mods.staff.candidates.clone()
    }

    /// `Comissao.contratar(Profissional)`: a previous one in the same role is let go.
    pub fn hire_staff(&mut self, candidate: usize) -> Result<(), HireError> {
        let Some(club) = my_club(self.world) else { return Err(HireError::Unavailable) };
        let Some(p) = self.world.mods.staff.candidates.get(candidate).cloned() else { return Err(HireError::Unavailable) };
        let minimum = minimum_reputation(p.level);
        if self.world.club(club).reputation < minimum {
            return Err(HireError::Reputation { minimum });
        }
        let year = crate::economy::year(self.world);
        let staff = &mut self.world.mods.staff;
        staff.candidates.remove(candidate);
        staff.hired.retain(|m| m.role != p.role);
        staff.hired.push(StaffMember { since: year, ..p });
        staff.hired.sort_by_key(|m| m.role);
        // Jornal.noticia: the news of the hiring is not ported
        Ok(())
    }

    /// `Comissao.demitir(Cargo)`
    pub fn fire_staff(&mut self, role: StaffRole) {
        self.world.mods.staff.hired.retain(|m| m.role != role);
    }

    /// `Comissao.pagarMes()`: the month's salaries, as a payment under "Diversos".
    pub(crate) fn pay_staff_month(&mut self) {
        let wages = staff_wages(self.world);
        if let Some(club) = my_club(self.world)
            && wages > 0
            && self.sys.config.on(Switch::Staff)
        {
            self.world.club_payment(club, wages.min(i32::MAX as i64) as i32, 0);
        }
    }

    /// `Comissao.gerarCandidatos(int)`: one of each tier per role (modest, middle and often
    /// top); better clubs see better professionals more often.
    fn draw_candidates(&mut self, month: i32) {
        let reputation = my_club(self.world).map_or(2, |c| self.world.club(c).reputation);
        let scale = 0.6 + 0.2 * reputation.clamp(0, 6) as f64;
        let mut candidates = Vec::new();
        for role in StaffRole::ALL {
            for i in 0..3 {
                let r = sorte!(self, "Comissao");
                let name = (r.next_int_bounded(FIRST_NAMES) as u16, r.next_int_bounded(SURNAMES) as u16);
                let base = match i {
                    0 => 1,
                    1 => 2 + r.next_int_bounded(2),
                    _ => 3 + r.next_int_bounded(3),
                };
                let better = r.next_int_bounded(100) < 15 * (reputation - 2).max(0);
                let level = (base + better as i32).clamp(1, MAX_LEVEL);
                let salary = java_round(BASE_SALARY[level as usize] as f64 * scale * (0.85 + r.next_double() * 0.3) / 1000.0) * 1000;
                let profile = r.next_int_bounded(PROFILES) as u16;
                candidates.push(StaffMember { role, name, level, salary, profile, since: 0 });
            }
        }
        let staff = &mut self.world.mods.staff;
        staff.candidates_month = month;
        staff.candidates = candidates;
    }

    /// `Caixa.hoje()`: today as yyyymmdd.
    pub fn today_date(&self) -> i32 {
        let f = self.runtime.clock.fields(self.world.today_millis());
        f.year * 10_000 + (f.month + 1) * 100 + f.day
    }
}

/// `Comissao.nivel(Cargo)` for the systems that only read the world.
pub fn staff_level(world: &World, config: &crate::config::ModConfig, role: StaffRole) -> i32 {
    if !config.on(Switch::Staff) {
        return MAX_LEVEL;
    }
    world.mods.staff.hired(role).map_or(0, |m| m.level)
}

/// `Comissao.folhaMensal()`
pub fn staff_wages(world: &World) -> i64 {
    world.mods.staff.hired.iter().map(|m| m.salary).sum()
}
