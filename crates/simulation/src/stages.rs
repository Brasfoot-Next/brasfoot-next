// Port of the league stages of the game outside the national leagues: the f.s constructor with
// est.LoadLigaOptions (a table, or groups with a knockout after), f.s.j(int, boolean) and
// f.s.aN(boolean), as the continental cups and the national teams' tournaments build them.

use brasfoot_model::ids::{ClubId, CompetitionId, StageId};
use brasfoot_model::saved::{Group, Stage, StageKind};

use crate::Sim;
use crate::sorting::standings;

/// `f.s.Zm` .. `Zz`: the knockout's pairings of the qualified, by their number and legs.
const ZM: [usize; 2] = [0, 1];
const ZN: [usize; 2] = [1, 0];
const ZO: [usize; 4] = [0, 3, 1, 2];
const ZP: [usize; 4] = [3, 0, 2, 1];
const ZQ: [usize; 4] = [0, 3, 2, 1];
const ZR: [usize; 4] = [3, 0, 1, 2];
const ZS: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
const ZT: [usize; 8] = [1, 0, 3, 2, 5, 4, 7, 6];
const ZU: [usize; 8] = [1, 6, 3, 4, 0, 7, 2, 5];
const ZV: [usize; 8] = [6, 1, 4, 3, 7, 0, 5, 2];
const ZW: [usize; 16] = [0, 3, 4, 7, 8, 11, 12, 15, 2, 1, 6, 5, 10, 9, 14, 13];
const ZX: [usize; 16] = [3, 0, 7, 4, 11, 8, 15, 12, 1, 2, 5, 6, 9, 10, 13, 14];
const ZZ: [usize; 32] = [31, 0, 25, 6, 24, 7, 27, 4, 23, 8, 22, 9, 29, 2, 21, 10, 26, 5, 20, 11, 19, 12, 28, 3, 18, 13, 17, 14, 16, 15, 30, 1];
const ZY: [usize; 32] = [7, 0, 2, 5, 3, 4, 6, 1, 15, 8, 10, 13, 11, 12, 14, 9, 23, 16, 18, 21, 19, 20, 22, 17, 31, 24, 26, 29, 27, 28, 30, 25];

/// `est.LoadLigaOptions`, with its initialisers.
#[derive(Clone, Debug)]
pub struct StageOptions {
    /// `nTimes`
    pub teams: usize,
    /// `doisTurnos`
    pub two_rounds: bool,
    /// `nGrupos`
    pub groups: i32,
    /// `gruposNumeroDiferenteTimes`, `numeroDefinidoTimesPorGrupo`
    pub group_sizes: Option<Vec<i32>>,
    /// `jogosDentroGrupo`
    pub games_within_group: bool,
    /// `numeroTimesMataMata`: the clubs of each group (or of the table) that go on.
    pub knockout_teams: i32,
    /// `duasVoltasMataMata`
    pub two_legged_knockout: Vec<bool>,
    /// `desempateEstadual`
    pub tiebreak: i32,
    /// `var0`
    pub format_code: i32,
    /// `classificaPeloGeral`
    pub qualify_by_overall_table: bool,
    /// `pulaDuasDatas`
    pub skip_two_dates: bool,
    /// `melhoresTerceiros`
    pub best_third_places: bool,
    /// `numeroTurnos`
    pub round_count: i32,
}

impl Default for StageOptions {
    fn default() -> Self {
        StageOptions {
            teams: 0,
            two_rounds: true,
            groups: 0,
            group_sizes: None,
            games_within_group: true,
            knockout_teams: 0,
            two_legged_knockout: vec![true; 7],
            tiebreak: 0,
            format_code: -1,
            qualify_by_overall_table: false,
            skip_two_dates: false,
            best_third_places: false,
            round_count: 0,
        }
    }
}

impl Sim<'_> {
    /// `new f.s(LoadLigaOptions, clubs, from, null, null, null, kind, null, false, null,
    /// by_rows, competition)`: the stage of `teams` clubs from `from` on, its table or groups
    /// drawn (`by_rows`: each group takes the next clubs; else they are dealt one to each group).
    pub fn new_league_stage(&mut self, options: &StageOptions, clubs: &[ClubId], from: usize, kind: i32, competition: CompetitionId, by_rows: bool) -> StageId {
        if options.knockout_teams == 1020 || options.teams == 68 {
            crate::unported!("f.s(LoadLigaOptions, ...) of format 1020 or 1068");
        }
        let members: Vec<ClubId> = if clubs.is_empty() { Vec::new() } else { clubs[from..from + options.teams].to_vec() };
        let stage = StageId(self.world.stages.len() as u32);
        self.world.stages.push(Stage {
            class: StageKind::League,
            competition: Some(competition),
            kind,
            format_code: options.format_code,
            qualify_by_overall_table: options.qualify_by_overall_table,
            skip_two_dates: options.skip_two_dates,
            knockout_teams: options.knockout_teams,
            two_legged_rounds: options.two_legged_knockout.clone(),
            group_count: options.groups,
            games_within_group: options.games_within_group,
            double_round: options.two_rounds,
            team_count: options.teams as i32,
            state_tiebreak: options.tiebreak,
            best_third_places: options.best_third_places,
            round_count: options.round_count,
            clubs: members.clone(),
            ..Default::default()
        });
        if !clubs.is_empty() {
            self.stage_fixtures(stage, options.group_sizes.as_deref(), by_rows);
        }
        // f.s.yH()
        let days = self.world.stage(stage).numbers.len() as i32;
        self.world.stage_mut(stage).value_ze = days;
        stage
    }

    /// The fixtures of a stage's table or groups (the end of the constructor and of `aN`).
    fn stage_fixtures(&mut self, stage: StageId, group_sizes: Option<&[i32]>, by_rows: bool) {
        let s = self.world.stage(stage);
        if s.group_count <= 1 {
            let clubs = s.clubs.clone();
            let both = s.double_round;
            self.league_fixtures(stage, &clubs, both);
        } else if let Some(sizes) = group_sizes {
            // f.s.a(int, int[]): each group the next clubs, its own table
            let (clubs, groups, both) = (s.clubs.clone(), s.group_count as usize, s.double_round);
            let mut next = 0;
            for &size in &sizes[..groups] {
                let row = clubs[next..next + size as usize].to_vec();
                next += size as usize;
                self.world.stage_mut(stage).groups.push(Group { clubs: row.clone() });
                self.league_fixtures(stage, &row, both);
            }
        } else {
            self.group_fixtures(stage, by_rows);
        }
    }

    /// `f.s.j(int, boolean)`: the clubs in groups, each group's table drawn.
    pub(crate) fn group_fixtures(&mut self, stage: StageId, by_rows: bool) {
        let s = self.world.stage(stage);
        let groups = s.group_count as usize;
        let clubs = s.clubs.clone();
        let per = clubs.len() / groups;
        let mut table = vec![Vec::with_capacity(per); groups];
        if by_rows {
            for (g, row) in table.iter_mut().enumerate() {
                row.extend_from_slice(&clubs[g * per..(g + 1) * per]);
            }
        } else {
            for p in 0..per {
                for (g, row) in table.iter_mut().enumerate() {
                    row.push(clubs[p * groups + g]);
                }
            }
        }
        let (within, both, kind) = (s.games_within_group, s.double_round, s.kind);
        for row in table {
            self.world.stage_mut(stage).groups.push(Group { clubs: row.clone() });
            if within {
                self.league_fixtures(stage, &row, both);
            }
        }
        if (kind != 7 || self.world.season() != 1) && !within {
            crate::unported!("f.s.a(ArrayList, boolean, true, ArrayList): the games across groups");
        }
    }

    /// `f.s.aN(boolean)` of a stage outside the national leagues: everything of the last
    /// season goes, and with `clear` the clubs too; the clubs left get new fixtures.
    pub(crate) fn restart_stage(&mut self, stage: StageId, clear: bool) {
        let s = self.world.stage(stage);
        if s.format_code == 1020 || s.format_code == 1068 {
            crate::unported!("f.s.aN(boolean) of format 1020 or 1068");
        }
        if s.country_ref.is_some() {
            crate::unported!("f.s.aN(boolean) of a national league's table");
        }
        let competition = s.competition.expect("a stage belongs to a competition");
        let s = self.world.stage_mut(stage);
        s.club_ns = None;
        s.groups.clear();
        s.knockout = None;
        s.numbers.clear();
        s.round = 1;
        if clear {
            s.clubs.clear();
        }
        // at.mw(): the competition's scorers
        self.world.competition_mut(competition).scorers.clear();
        if !self.world.stage(stage).clubs.is_empty() {
            self.stage_fixtures(stage, None, false);
        }
    }
}

impl Sim<'_> {
    /// `f.s.yP()`: the stage's qualified to the knockout after it: the continental cups pair
    /// group winners with runners-up (by the table in South America and the rest, shuffled in
    /// Europe); else the first of each group (or of the table) in the pairing order of their
    /// number.
    pub(crate) fn knockout_after_stage(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        let competition = s.competition.expect("a stage belongs to a competition");
        let (kind, subkind) = (s.kind, self.world.competition(competition).subkind);
        let two_legged = s.two_legged_rounds[0];
        let mut away_goals = false;
        let mut drawn: Vec<ClubId> = Vec::new();
        if !s.groups.is_empty() && !s.qualify_by_overall_table {
            let by_table = kind == 4 && (1..=3).contains(&subkind);
            let shuffled = (kind == 4 || kind == 6) && subkind == 0;
            if (kind == 4 || kind == 6) && subkind == 1 {
                away_goals = true;
            }
            if shuffled || by_table {
                let mut firsts: Vec<ClubId> = s.groups.iter().map(|g| g.clubs[0]).collect();
                let mut seconds: Vec<ClubId> = s.groups.iter().map(|g| g.clubs[1]).collect();
                if by_table {
                    firsts.sort_by(|&a, &b| standings(self.world, stage, a, b));
                    seconds.sort_by(|&a, &b| standings(self.world, stage, a, b));
                } else {
                    self.runtime.rng.shuffle(&mut firsts);
                    self.runtime.rng.shuffle(&mut seconds);
                }
                const FIRSTS: [usize; 8] = [0, 7, 3, 4, 1, 6, 2, 5];
                const SECONDS: [usize; 8] = [7, 0, 4, 3, 6, 1, 5, 2];
                for i in 0..firsts.len() {
                    drawn.push(seconds[SECONDS[i]]);
                    drawn.push(firsts[FIRSTS[i]]);
                }
            } else {
                let per = s.knockout_teams as usize;
                let qualified: Vec<ClubId> = s.groups.iter().flat_map(|g| g.clubs[..per].to_vec()).collect();
                let order: Option<&[usize]> = if s.format_code != 7 && s.format_code != 10 {
                    match qualified.len() {
                        32 => Some(&ZY),
                        16 => Some(if two_legged { &ZX[..] } else { &ZW[..] }),
                        8 => Some(if two_legged { &ZV[..] } else { &ZU[..] }),
                        4 => Some(if two_legged { &ZR[..] } else { &ZQ[..] }),
                        2 => Some(if two_legged { &ZN[..] } else { &ZM[..] }),
                        _ => None,
                    }
                } else {
                    Some(if two_legged { &ZT[..] } else { &ZS[..] })
                };
                drawn = match order {
                    Some(o) => (0..qualified.len()).map(|i| qualified[o[i]]).collect(),
                    None => qualified,
                };
            }
        } else {
            let qualified: Vec<ClubId> = s.clubs[..s.knockout_teams as usize].to_vec();
            let order: Option<&[usize]> = match qualified.len() {
                16 => Some(&ZW),
                8 => Some(if two_legged { &ZV[..] } else { &ZU[..] }),
                4 => Some(if two_legged { &ZP[..] } else { &ZO[..] }),
                2 => Some(if two_legged { &ZN[..] } else { &ZM[..] }),
                _ => None,
            };
            drawn = match order {
                Some(o) => (0..qualified.len()).map(|i| qualified[o[i]]).collect(),
                None => qualified,
            };
        }
        let s = self.world.stage(stage);
        if s.format_code == 1020 {
            crate::unported!("f.B.a(ArrayList, f.s, int): the knockout of a league of format 1020");
        }
        let (legs, format) = (s.two_legged_rounds.clone(), s.format_code);
        let knockout = self.new_knockout_stage(competition, Some(stage), drawn.len() as i32, kind, 0, legs, -1);
        self.world.stage_mut(stage).knockout = Some(knockout);
        self.new_knockout_round(knockout, &drawn, 0, two_legged, 0, kind, away_goals);
        if format == 1802 || format == 1803 {
            self.world.stage_mut(knockout).phase_code = format;
        }
    }

    /// `f.s.yO()`: the first of each group and the best of the thirds (or of the seconds, in
    /// the formats that keep fewer per group), paired by their number.
    pub(crate) fn knockout_with_best_thirds(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        let competition = s.competition.expect("a stage belongs to a competition");
        let per = s.knockout_teams as usize;
        let (groups, format, division) = (s.group_count, s.format_code, s.division);
        let mut qualified: Vec<ClubId> = s.groups.iter().flat_map(|g| g.clubs[..per].to_vec()).collect();
        let place = if groups == 10 && format == 1403 || groups == 17 { 1 } else { 2 };
        let need = if groups == 10 && format == 1403 { 2 } else { 3 };
        let mut others: Vec<ClubId> = s.groups.iter().filter(|g| g.clubs.len() >= need).map(|g| g.clubs[place]).collect();
        if division == 4 {
            qualified.sort_by(|&a, &b| standings(self.world, stage, a, b));
        }
        others.sort_by(|&a, &b| standings(self.world, stage, a, b));
        let best = match groups {
            10 if format == 1403 => 6,
            3 => 2,
            6 => 4,
            12 => 8,
            17 => 15,
            _ => 0,
        };
        if others.len() >= best {
            qualified.extend_from_slice(&others[..best]);
        }
        let two_legged = self.world.stage(stage).two_legged_rounds[0];
        let order: Option<&[usize]> = match qualified.len() {
            32 => Some(&ZZ),
            16 => Some(&ZW),
            8 => Some(if two_legged { &ZV[..] } else { &ZU[..] }),
            _ => None,
        };
        let Some(order) = order else { return };
        let drawn: Vec<ClubId> = (0..qualified.len()).map(|i| qualified[order[i]]).collect();
        let s = self.world.stage(stage);
        let (legs, kind) = (s.two_legged_rounds.clone(), s.kind);
        let knockout = self.new_knockout_stage(competition, Some(stage), drawn.len() as i32, kind, 0, legs, -1);
        self.world.stage_mut(stage).knockout = Some(knockout);
        self.new_knockout_round(knockout, &drawn, 0, two_legged, 0, kind, false);
    }
}
