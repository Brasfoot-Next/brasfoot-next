//! A career open in the app: the world, the state kept outside the save and the mod systems,
//! loaded from this app's `.bfn` or migrated once from an original `.s22`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_persistence::native::SaveMeta;
#[cfg(not(target_arch = "wasm32"))]
use brasfoot_persistence::native::{read_save, write_save};
use brasfoot_systems::ModSystems;
use brasfoot_model::options::GameOptions;
use brasfoot_simulation::Sim;
use brasfoot_simulation::human::HumanDecisions;
use brasfoot_simulation::human::substitutions::MatchGrid;
use brasfoot_simulation::new_game::NewGame;
use brasfoot_systems::config::ModConfig;

use crate::game_files::GameFiles;
#[cfg(not(target_arch = "wasm32"))]
use crate::game_files::{data_dir, file_stem};

pub struct Career {
    pub world: World,
    pub runtime: Runtime,
    pub systems: ModSystems,
    /// The `.bfn` this career is saved to.
    pub path: PathBuf,
}

/// Why something could not be done, for the screens to explain.
#[derive(Debug)]
pub enum CareerError {
    Read(String),
    /// A part of the game not ported yet was reached; the day was undone.
    NotPortedYet(String),
}

impl Career {
    pub fn empty() -> Self {
        Career {
            world: World::default(),
            runtime: Runtime::new(RandomSource::from_clock()),
            systems: ModSystems::new(ModConfig::default()),
            path: PathBuf::new(),
        }
    }

    fn new(world: World, path: PathBuf, files: &GameFiles) -> Self {
        let mut runtime = Runtime::new(RandomSource::from_clock());
        runtime.names = files.name_lists();
        #[cfg(not(target_arch = "wasm32"))]
        let config = std::fs::read_to_string(files.dir.join("bfmod.config")).unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let config = String::new();
        Career { world, runtime, systems: ModSystems::new(ModConfig::parse(&config)), path }
    }

    /// Migrates an original save (read only) into a new `.bfn` of this app.
    pub fn import(s22: &Path, files: &GameFiles) -> Result<Self, CareerError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (s22, files);
            return Err(CareerError::Read("Importação não suportada na web".to_string()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bytes = std::fs::read(s22).map_err(|e| CareerError::Read(e.to_string()))?;
            // a save the importer does not understand yet must not take the app down
            let world = catch_unwind(AssertUnwindSafe(|| brasfoot_persistence::import::import_s22(bytes)))
                .map_err(|panic| CareerError::Read(panic.downcast_ref::<String>().cloned().unwrap_or_default()))?
                .map_err(|e| CareerError::Read(format!("{e:?}")))?;
            let path = data_dir().join("saves").join(format!("{}.bfn", file_stem(s22)));
            let mut career = Career::new(world, path, files);
            career.save(Some(s22)).map_err(CareerError::Read)?;
            Ok(career)
        }
    }

    pub fn open(bfn: &Path, files: &GameFiles) -> Result<Self, CareerError> {
        #[cfg(not(target_arch = "wasm32"))]
        let (_, mut world) = read_save::<World>(bfn).map_err(|e| CareerError::Read(format!("{e:?}")))?;
        #[cfg(target_arch = "wasm32")]
        let (_, mut world) = web_load(bfn).map_err(CareerError::Read)?;
        // the original's load relinks the transfer history by the numbers of the save (g.b.Aj)
        world.relink_transfers();
        Ok(Career::new(world, bfn.to_owned(), files))
    }

    pub fn save(&mut self, imported_from: Option<&Path>) -> Result<(), String> {
        let club = self.human_club().map(|c| self.world.club(c).name.clone()).unwrap_or_default();
        let coach = self.world.game.human_coaches.first().map(|&c| self.world.coach(c).name.clone()).unwrap_or_default();
        let f = self.runtime.clock.fields(self.world.today_millis());
        let meta = SaveMeta {
            format_version: brasfoot_persistence::native::FORMAT_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            club,
            coach,
            date: format!("{:04}-{:02}-{:02}", f.year, f.month + 1, f.day),
            season: self.world.season(),
            #[cfg(not(target_arch = "wasm32"))]
            saved_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            #[cfg(target_arch = "wasm32")]
            saved_at: web_time::SystemTime::now().duration_since(web_time::SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            imported_from: imported_from.map(|p| p.display().to_string()),
        };
        // the original's save numbers the players for the transfer history (g.b.Ai) and the
        // competitions for the players' records (g.b.Ag)
        self.world.number_players_for_save();
        self.world.number_competitions_for_save();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            write_save(&self.path, &meta, &self.world).map_err(|e| format!("{e:?}"))
        }

        #[cfg(target_arch = "wasm32")]
        {
            web_save(&self.path, &meta, &self.world)
        }
    }

    /// The player's club (`best.f.B()`).
    pub fn human_club(&self) -> Option<ClubId> {
        self.world.game.human_coaches.iter().find_map(|&c| self.world.coach(c).club_id)
    }

    /// Days can be played while every club is the AI's: the player's side of a match day is
    /// not ported yet.
    pub fn can_advance(&self) -> bool {
        self.world.game.human_clubs.is_empty() && self.human_club().is_none()
    }

    /// One match day (`c.a.SR.V()`). A part not ported yet undoes the day.
    /// `best.p.dg()` from the game folder's teams: the new career's world, before its coaches.
    pub fn create(new_game: &NewGame, options: &GameOptions, year: i32, files: &GameFiles) -> Result<Career, CareerError> {
        let mut career = Career::new(World::default(), PathBuf::new(), files);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut sim = Sim { world: &mut career.world, runtime: &mut career.runtime, hooks: &mut career.systems, human: None };
            sim.start_new_world(options);
            sim.create_world(new_game, options, year);
        }));
        result.map_err(|panic| CareerError::NotPortedYet(panic_text(panic)))?;
        Ok(career)
    }

    /// `ah.b(String, int)`: a human coach takes the club.
    pub fn add_human_coach(&mut self, club: ClubId, name: &str, nationality: i32) {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.hire_new_human_coach(club, name, nationality);
    }

    /// `bV.AB()`: the first match day (`f.az()`), and the career's first save, named after its
    /// first human coach.
    pub fn begin(&mut self) -> Result<(), String> {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.find_next_day();
        let coach = self.world.game.human_coaches.first().map(|&c| self.world.coach(c).name.clone()).unwrap_or_default();
        self.path = free_save_path(&coach);
        self.save(None)
    }

    pub fn advance(&mut self) -> Result<(), CareerError> {
        let before = self.world.clone();
        let result = catch_unwind(AssertUnwindSafe(|| brasfoot_simulation::advance(&mut self.world, &mut self.runtime, &mut self.systems)));
        result.map_err(|panic| {
            self.world = before;
            let what = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            CareerError::NotPortedYet(what)
        })
    }

    /// `BolaDeOuro.tick` when the main screen comes back. A part not ported yet undoes it and
    /// keeps only the transfer windows' calendar, which the purchases depend on.
    pub fn mod_tick(&mut self) -> Result<(), CareerError> {
        let before = self.world.clone();
        let result = catch_unwind(AssertUnwindSafe(|| self.systems.main_screen_tick(&mut self.world, &mut self.runtime)));
        result.map_err(|panic| {
            self.world = before;
            brasfoot_systems::windows::calendar_tick(&mut self.world, &self.runtime.clock);
            CareerError::NotPortedYet(panic_text(panic))
        })
    }

    /// The human's answer on the offer screen (`a.jm`) to an offer of the mod's windows; a sale
    /// or a loan brings the main screen back, and with it the mod's clock (`best.S.aY(2)`).
    pub fn answer_squad_offer(&mut self, offer: brasfoot_systems::windows::SquadOffer, accept: bool) {
        let done = self.systems.with(&mut self.world, &mut self.runtime, |m| m.answer_squad_offer(offer, accept));
        if done {
            let _ = self.mod_tick();
        }
    }

    pub fn start_live_round(&mut self) -> Result<brasfoot_simulation::human::round::LiveRound, CareerError> {
        let before = self.world.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.start_human_day()
        }));
        result.map_err(|panic| {
            self.world = before;
            CareerError::NotPortedYet(panic_text(panic))
        })
    }

    /// One tick of the round. When the game opened the substitutions window (`a.cD`, at half
    /// time or after a red card or an injury of the human's club) the match and whether it is
    /// half time come back: the screen shows the window before the next tick.
    pub fn step_live_round(&mut self, round: &mut brasfoot_simulation::human::round::LiveRound) -> Option<(MatchId, bool)> {
        let mut screen = ScreenDecisions::default();
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: Some(&mut screen) }.round_tick_step(round);
        screen.substitutions
    }

    /// `a.ji.qW()`: a player of the human's club retires now; true when he did.
    pub fn retire_player(&mut self, player: brasfoot_model::ids::PlayerId, name: Option<String>, position: i32) -> bool {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.retire_by_request(player, name, position)
    }

    /// `bn.a(as)`: a message shown is read.
    pub fn read_message(&mut self, coach: brasfoot_model::ids::CoachId, index: usize) {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.read_message(coach, index);
    }

    /// `bn.cQ(int)`: the answer to the board about fining a player; true when fined.
    pub fn answer_fine(&mut self, club: ClubId, coach: brasfoot_model::ids::CoachId, index: usize, fine: bool) -> bool {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.answer_fine(club, coach, index, fine)
    }

    /// `cD.H(int, int)`: a move of the human in the substitutions window.
    pub fn move_in_match(&mut self, grid: &mut MatchGrid, from: usize, to: usize, half: i32, minute: i32, half_time: bool) -> bool {
        Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.move_in_match(grid, from, to, half, minute, half_time)
    }

    pub fn finish_live_round(&mut self, round: &mut brasfoot_simulation::human::round::LiveRound) {
        while round.on {
            Sim { world: &mut self.world, runtime: &mut self.runtime, hooks: &mut self.systems, human: None }.round_tick_step(round);
        }
    }
}

/// The human of the screens: the game's questions the screens answer later. The window `a.cD`
/// opens after the tick that asked for it; its changes count from the next minute on, where
/// the original made them inside the tick.
#[derive(Default)]
struct ScreenDecisions {
    substitutions: Option<(MatchId, bool)>,
}

impl HumanDecisions for ScreenDecisions {
    fn substitutions(&mut self, _world: &mut World, game_match: MatchId, half_time: bool) {
        self.substitutions.get_or_insert((game_match, half_time));
    }
}

fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// A save file of this app for a new career, named after its coach.
fn free_save_path(name: &str) -> PathBuf {
    let base: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect::<String>().trim().to_owned();
    let base = if base.is_empty() { "carreira".to_owned() } else { base };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = data_dir().join("saves");
        let mut path = dir.join(format!("{base}.bfn"));
        let mut n = 2;
        while path.exists() {
            path = dir.join(format!("{base} {n}.bfn"));
            n += 1;
        }
        path
    }

    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok()).flatten();
        let exists = |stem: &str| -> bool {
            storage.as_ref().and_then(|s| s.get_item(&format!("bfn_save:{stem}")).ok().flatten()).is_some()
        };
        if !exists(&base) {
            return PathBuf::from(format!("{base}.bfn"));
        }
        let mut n = 2;
        loop {
            let stem = format!("{base} {n}");
            if !exists(&stem) {
                return PathBuf::from(format!("{stem}.bfn"));
            }
            n += 1;
        }
    }
}

/// This app's saves, newest first.
pub fn own_saves() -> Vec<(PathBuf, SaveMeta)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut saves: Vec<(PathBuf, SaveMeta)> = std::fs::read_dir(data_dir().join("saves"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "bfn"))
            .filter_map(|p| brasfoot_persistence::native::read_meta(&p).ok().map(|m| (p, m)))
            .collect();
        saves.sort_by_key(|(_, m)| std::cmp::Reverse(m.saved_at));
        saves
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut saves = Vec::new();
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() else {
            return saves;
        };
        let Ok(len) = storage.length() else {
            return saves;
        };
        for i in 0..len {
            let Ok(Some(key)) = storage.key(i) else { continue };
            if let Some(stem) = key.strip_prefix("bfn_save:") {
                if let Ok(Some(b64)) = storage.get_item(&key) {
                    if let Some(bytes) = b64_decode(&b64) {
                        if let Ok(meta) = brasfoot_persistence::native::decode_meta(&mut &bytes[..]) {
                            saves.push((PathBuf::from(format!("{stem}.bfn")), meta));
                        }
                    }
                }
            }
        }
        saves.sort_by_key(|(_, m)| std::cmp::Reverse(m.saved_at));
        saves
    }
}

#[cfg(target_arch = "wasm32")]
fn save_storage_key(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("carreira");
    format!("bfn_save:{stem}")
}

#[cfg(target_arch = "wasm32")]
fn web_save(path: &Path, meta: &SaveMeta, world: &World) -> Result<(), String> {
    match brasfoot_persistence::native::encode_save(meta, world) {
        Ok(bytes) => {
            let key = save_storage_key(path);
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
                let b64 = b64_encode(&bytes);
                if let Err(e) = storage.set_item(&key, &b64) {
                    log::warn!("Falha ao salvar no localStorage: {e:?}");
                }
            }
            Ok(())
        }
        Err(e) => {
            log::warn!("Falha ao codificar save: {e:?}");
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn web_load(path: &Path) -> Result<(SaveMeta, World), String> {
    let key = save_storage_key(path);
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .ok_or_else(|| "Local storage não disponível".to_string())?;
    let b64 = storage.get_item(&key)
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| "Save não encontrado".to_string())?;
    let bytes = b64_decode(&b64)
        .ok_or_else(|| "Dados corrompidos do save".to_string())?;
    brasfoot_persistence::native::decode_save(&bytes[..])
        .map_err(|e| format!("{e:?}"))
}

#[cfg(target_arch = "wasm32")]
fn b64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(target_arch = "wasm32")]
fn b64_decode(input: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
        table[c as usize] = i as u8;
    }
    let input = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for &b in input.as_bytes() {
        if b == b'=' || b.is_ascii_whitespace() { continue; }
        let val = table[b as usize];
        if val == 255 { return None; }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}
