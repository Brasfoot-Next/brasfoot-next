// Port of bfmod.Estadios: stadiums named after club legends, once in a club's history. A legend
// is a retired player with a long or prolific career at the club (the idols of `Clubes`). At each
// season end an AI club with a legend renames its ground now and then; the player's club gets the
// board's proposal and decides (now, later or never). The name lives in the game's stadium.

use std::collections::{HashMap, HashSet};

use brasfoot_model::brands;
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{AwardKind, FanEvent, Idol, StadiumRename};
use brasfoot_model::world::World;

use crate::awards::season::player_key;
use crate::config::Switch;
use crate::economy::{ai_clubs, year};
use crate::staff::my_club;
use crate::{Mod, sorte};

/// `JOGOS`, `GOLS`, `JOGOS_COM_BOLA`
const GAMES: i32 = 300;
const GOALS: i32 = 150;
const GAMES_WITH_BALLON_DOR: i32 = 120;
/// `CHANCE_IA_GENERICO`, `CHANCE_IA_COM_NOME`: a season's chance that an AI club with a legend
/// renames a nameless ground, or a named one.
const CHANCE_GENERIC: i32 = 10;
const CHANCE_NAMED: i32 = 3;
/// `TORCIDA`: the fans' approval of the player's homage.
const FANS: i32 = 8;

/// `Estadios.Lenda`
#[derive(Clone, Debug, PartialEq)]
pub struct Legend {
    pub idol: Idol,
    /// Every award he won.
    pub awards: i32,
    pub greatness: f64,
}

impl Legend {
    /// `Estadios.nome(Lenda)`
    pub fn stadium_name(&self) -> String {
        brands::legend_stadium(&self.idol.name)
    }
}

/// `Estadios.Dados`: what the legends are judged from, read once per check.
pub struct LegendData {
    idols: HashMap<i32, Vec<Idol>>,
    /// The awards of each player key, by kind.
    awards: HashMap<String, HashMap<AwardKind, i32>>,
    active: HashSet<String>,
}

impl LegendData {
    pub fn new(world: &World) -> Self {
        let mut idols: HashMap<i32, Vec<Idol>> = HashMap::new();
        for i in &world.mods.awards.idols {
            idols.entry(i.club).or_default().push(i.clone());
        }
        let mut awards: HashMap<String, HashMap<AwardKind, i32>> = HashMap::new();
        for a in &world.mods.awards.awards {
            *awards.entry(a.key.clone()).or_default().entry(a.kind).or_default() += 1;
        }
        let active = world.game.players.iter().chain(&world.game.youth_players).map(|&p| player_key(world, p)).collect();
        LegendData { idols, awards, active }
    }
}

/// `Estadios.lendas(ah, Dados)`: a club's retired legends, greatest first.
pub fn legends(world: &World, club: ClubId, data: &LegendData) -> Vec<Legend> {
    let mut out = Vec::new();
    for i in data.idols.get(&world.club(club).id).into_iter().flatten() {
        if data.active.contains(&i.key) {
            continue;
        }
        let his = data.awards.get(&i.key);
        let ballons = his.and_then(|m| m.get(&AwardKind::BallonDor)).copied().unwrap_or(0);
        let total: i32 = his.map_or(0, |m| m.values().sum());
        if i.games >= GAMES || i.goals >= GOALS || ballons >= 1 && i.games >= GAMES_WITH_BALLON_DOR {
            let greatness = i.games as f64 + 1.5 * i.goals as f64 + 60.0 * total as f64 + 150.0 * ballons as f64;
            out.push(Legend { idol: i.clone(), awards: total, greatness });
        }
    }
    out.sort_by(|a, b| b.greatness.total_cmp(&a.greatness));
    out
}

/// `Estadios.renomeado(ah)`
pub fn renamed<'a>(world: &'a World, club: ClubId) -> Option<&'a StadiumRename> {
    let id = world.club(club).id;
    world.mods.stadiums.renamed.iter().find(|r| r.club == id)
}

impl Mod<'_> {
    /// `Estadios.fimDeTemporada()`: the AI clubs decide; the player's club gets its proposal on
    /// the next main screen.
    pub(crate) fn stadiums_season_end(&mut self) {
        if !self.sys.config.on(Switch::Stadiums) {
            return;
        }
        let year = year(self.world);
        let data = LegendData::new(self.world);
        for club in ai_clubs(self.world) {
            let Some(s) = self.world.club(club).stadium else { continue };
            if renamed(self.world, club).is_some() {
                continue;
            }
            let generic = self.world.stadium(s).name.starts_with(brands::MUNICIPAL_STADIUM);
            let chance = if generic { CHANCE_GENERIC } else { CHANCE_NAMED };
            if sorte!(self, "Estadios").next_int_bounded(100) >= chance {
                continue;
            }
            let Some(first) = legends(self.world, club, &data).into_iter().next() else { continue };
            self.rename_stadium(club, &first, year);
            // the rename's news is not ported
        }
        if let Some(club) = my_club(self.world)
            && !self.world.mods.stadiums.never
            && renamed(self.world, club).is_none()
            && self.world.club(club).stadium.is_some()
            && !legends(self.world, club, &data).is_empty()
        {
            self.world.mods.stadiums.pending = true;
        }
    }

    /// `Estadios.tick()`: on the main screen after the season end, the board's proposal is shown
    /// once ("Agora não" brings it back at the next season end).
    pub(crate) fn stadiums_tick(&mut self) {
        if !self.sys.config.on(Switch::Stadiums) {
            return;
        }
        let state = &mut self.world.mods.stadiums;
        if state.pending {
            state.pending = false;
            // proposta(): the dialog opens only while the club can still rename
            if let Some(club) = my_club(self.world)
                && self.world.club(club).stadium.is_some()
                && renamed(self.world, club).is_none()
                && !legends(self.world, club, &LegendData::new(self.world)).is_empty()
            {
                self.world.mods.stadiums.shown = true;
            }
        }
    }

    /// The proposal's legends (`proposta()`: the three greatest).
    pub fn stadium_proposal(&self) -> Vec<Legend> {
        let Some(club) = my_club(self.world) else { return Vec::new() };
        let mut l = legends(self.world, club, &LegendData::new(self.world));
        l.truncate(3);
        l
    }

    /// "Rebatizar" with the legend at that place of the proposal.
    pub fn accept_stadium_proposal(&mut self, legend: usize) {
        self.world.mods.stadiums.shown = false;
        let Some(club) = my_club(self.world) else { return };
        let legends = legends(self.world, club, &LegendData::new(self.world));
        if legends.is_empty() {
            return;
        }
        let l = legends[legend.min(legends.len() - 1)].clone();
        let year = year(self.world);
        let stadium = self.rename_stadium(club, &l, year);
        self.fans_event(FANS, FanEvent::StadiumRenamed { stadium });
        // Jornal.noticia: the homage's news is not ported
    }

    /// "Agora não"
    pub fn postpone_stadium_proposal(&mut self) {
        self.world.mods.stadiums.shown = false;
    }

    /// "Nunca rebatizar"
    pub fn refuse_stadium_proposal(&mut self) {
        let state = &mut self.world.mods.stadiums;
        state.shown = false;
        state.never = true;
    }

    /// `Estadios.renomear(ah, Lenda, int)`: the new name, kept by `Patrimonio` as the base
    /// (which starts it, even with its switch off). Returns the new name.
    fn rename_stadium(&mut self, club: ClubId, legend: &Legend, year: i32) -> String {
        let s = self.world.club(club).stadium.expect("a club with a stadium");
        let new = legend.stadium_name();
        let rename = StadiumRename { club: self.world.club(club).id, year, old: self.world.stadium(s).name.clone(), new: new.clone(), legend: legend.idol.key.clone() };
        self.world.stadium_mut(s).name = new.clone();
        self.on_stadium_renamed(club, new.clone());
        let renamed = &mut self.world.mods.stadiums.renamed;
        match renamed.iter().position(|r| r.club == rename.club) {
            Some(i) => renamed[i] = rename,
            None => renamed.push(rename),
        }
        new
    }
}
