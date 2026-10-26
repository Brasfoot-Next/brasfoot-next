//! The game's data files (read only): the team files and the league configurations, in the
//! order the game reads them.

use std::path::Path;

use brasfoot_model::templates::{LeagueConfig, PlayerTemplate, StateLeagueConfig, TeamTemplate};

use crate::javaser::{self, JavaSerError, Object, Value};

fn player(o: &Object) -> PlayerTemplate {
    PlayerTemplate {
        name: o.string("a").unwrap_or_default(),
        star: o.bool("b"),
        country: o.int("c"),
        age: o.int("d"),
        position: o.int("e"),
        status: o.int("f"),
        trait1: o.int("g"),
        trait2: o.int("h"),
        side: o.int("i"),
        hash: o.int("hash"),
        world_class: o.bool("j"),
        tid: o.int("tid"),
        sid: o.int("sid"),
        aid: o.int("aid"),
    }
}

fn players(v: Option<&Value>) -> Vec<PlayerTemplate> {
    v.map(|v| v.list().iter().filter_map(|p| p.as_object().map(player)).collect()).unwrap_or_default()
}

/// One team file (`e.t`), with the file reference it holds (`d`).
pub fn read_team(bytes: &[u8]) -> Result<TeamTemplate, JavaSerError> {
    let v = javaser::read(bytes)?;
    let o = v.as_object().ok_or_else(|| JavaSerError { message: "a team file holds an object".into(), position: 0 })?;
    Ok(TeamTemplate {
        file_ref: o.string("d").unwrap_or_default(),
        valid: o.bool("valid"),
        id: o.int("id"),
        country: o.int("a"),
        state: o.int("b"),
        level: o.int("c"),
        name: o.string("e").unwrap_or_default(),
        stadium: o.string("f").unwrap_or_default(),
        capacity: o.int("g"),
        coach: o.string("h").unwrap_or_default(),
        coach_country: o.int("i"),
        players: players(o.field("l")),
        youth: players(o.field("m")),
        reputation: o.int("n"),
        base_color: o.int("o"),
        tid: o.int("tid"),
        sid: o.int("sid"),
        aid: o.int("aid"),
        vid: o.int("vid"),
        color1: o.string("cor1").unwrap_or_default(),
        color2: o.string("cor2").unwrap_or_default(),
        rivals: Vec::new(),
    })
}

/// `U.w(false)`: the `.ban` files of a `teams` folder in the directory's own order (Java's
/// `File.listFiles`), those of format 185 that are valid; files that do not read are skipped.
pub fn read_teams(dir: &Path) -> Vec<TeamTemplate> {
    let mut teams = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return teams };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().ends_with(".ban") || !path.is_file() {
            continue;
        }
        let file_ref = &name[..name.rfind('.').unwrap_or(name.len())];
        let Ok(bytes) = std::fs::read(&path) else { continue };
        // U.a(String, String, boolean): the file's name is the team's reference
        if let Ok(mut team) = read_team(&bytes)
            && (team.vid == 185 || team.vid == 0)
            && team.valid
        {
            team.file_ref = file_ref.to_owned();
            teams.push(team);
        }
    }
    teams
}

fn flags(v: Option<&Value>, default: Vec<bool>) -> Vec<bool> {
    match v {
        Some(Value::Array(items)) => items.iter().map(|x| x.as_bool().unwrap_or(false)).collect(),
        _ => default,
    }
}

/// `est.ConfigLigaType`
fn league_config(o: &Object) -> LeagueConfig {
    let d = LeagueConfig::default();
    let int = |name: &str, default: i32| o.field(name).and_then(Value::as_int).unwrap_or(default);
    let flag = |name: &str, default: bool| o.field(name).and_then(Value::as_bool).unwrap_or(default);
    LeagueConfig {
        country: int("pais", d.country),
        division: int("divisao", d.division),
        formula: int("formula", d.formula),
        tiebreak: int("desempate", d.tiebreak),
        teams: int("nTimes", d.teams),
        two_rounds: flag("doisTurnos", d.two_rounds),
        groups: int("nGrupos", d.groups),
        games_within_group: flag("jogosDentroGrupo", d.games_within_group),
        relegated_by_group: flag("rebaixadoPeloGrupo", d.relegated_by_group),
        knockout_teams: int("numeroTimesMataMata", d.knockout_teams),
        valid: flag("valido", d.valid),
        relegated: int("nRebaixados", d.relegated),
        two_legged_knockout: flags(o.field("duasVoltasMataMata"), d.two_legged_knockout.clone()),
        qualify_by_overall_table: flag("classificaPeloGeral", d.qualify_by_overall_table),
        best_third_places: flag("melhoresTerceiros", d.best_third_places),
        name: o.string("nome"),
        division_name: o.string("nomeDivisao"),
        relegation_playoff: int("playoffRebaixamento", d.relegation_playoff),
        two_legged_relegation_playoff: flags(o.field("duasVoltasplayoffReb"), d.two_legged_relegation_playoff.clone()),
        two_legged_promotion_knockout: flags(o.field("duasVoltasMataMataSobe"), d.two_legged_promotion_knockout.clone()),
        promotion_places: int("vagasSobemPeloMataMata", d.promotion_places),
        relegated_directly: int("rebaixadosDireto", d.relegated_directly),
        file_version: int("versaoArquivo", d.file_version),
    }
}

/// `p.ds()`: the league configurations of a `conf_ligas_nacionais` folder (`*.cfg`, in the
/// directory's order).
pub fn read_league_configs(dir: &Path) -> Vec<LeagueConfig> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".cfg") {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        let Ok(v) = javaser::read(&bytes) else { continue };
        let Some(list) = v.as_object().and_then(|o| o.field("a").cloned()) else { continue };
        for item in list.list() {
            let Some(o) = item.as_object() else { continue };
            out.push(league_config(o));
        }
    }
    out
}

/// `w.ef()`: the state league configurations of a `conf_estadual` folder (`*.ces`, in the
/// directory's order; `w.g(int, int)` takes the first of a state and division).
pub fn read_state_league_configs(dir: &Path) -> Vec<StateLeagueConfig> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".ces") {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        let Ok(v) = javaser::read(&bytes) else { continue };
        let Some(list) = v.as_object().and_then(|o| o.field("a").cloned()) else { continue };
        for item in list.list() {
            let Some(o) = item.as_object() else { continue };
            out.push(state_league_config(o));
        }
    }
    out
}

/// `est.ConfigEstadualType`
fn state_league_config(o: &Object) -> StateLeagueConfig {
    let d = StateLeagueConfig::default();
    let int = |name: &str, default: i32| o.field(name).and_then(Value::as_int).unwrap_or(default);
    let mut two_legged_finals = d.two_legged_finals;
    if let Some(Value::Array(items)) = o.field("finaisIdaVolta") {
        for (slot, item) in two_legged_finals.iter_mut().zip(items.iter()) {
            *slot = item.as_int().unwrap_or(*slot);
        }
    }
    StateLeagueConfig {
        state: int("id", d.state),
        division: int("divisao", d.division),
        formula: int("formula", d.formula),
        relegated: int("nRebaixados", d.relegated),
        tiebreak: int("desempate", d.tiebreak),
        two_legged_finals,
    }
}

/// `options.bcf` (`est.Options`); the defaults when it is missing or does not read.
pub fn read_options(path: &Path) -> brasfoot_model::options::GameOptions {
    let mut o = brasfoot_model::options::GameOptions::default();
    let Ok(bytes) = std::fs::read(path) else { return o };
    let Ok(v) = javaser::read(&bytes) else { return o };
    let Some(f) = v.as_object() else { return o };
    let flag = |name: &str, default: bool| f.field(name).and_then(Value::as_bool).unwrap_or(default);
    let int = |name: &str, default: i32| f.field(name).and_then(Value::as_int).unwrap_or(default);
    let flags = |name: &str, default: &Vec<bool>| match f.field(name) {
        Some(Value::Array(items)) => items.iter().map(|x| x.as_bool().unwrap_or(false)).collect(),
        _ => default.clone(),
    };
    o.plays_state = flag("jogaEstadual", o.plays_state);
    o.plays_regionals = flag("jogaRegionais", o.plays_regionals);
    o.default_state_groups = flag("usaGrupoPadraoEstadual", o.default_state_groups);
    o.plays_international_clubs = flag("jogaIntClubes", o.plays_international_clubs);
    o.plays_all_national_teams = flag("jogaSelecoesAll", o.plays_all_national_teams);
    o.monthly_salary = flag("salarioMensal", o.monthly_salary);
    o.individual_skills = flag("habilidadeIndividual", o.individual_skills);
    o.international_in_first_year = flag("jogaIntano1", o.international_in_first_year);
    o.default_international_groups = flag("gruposIntPadrao", o.default_international_groups);
    o.show_international = flags("verJint", &o.show_international);
    o.autosave = int("autoSalvar", o.autosave);
    o.speed = int("velocidade", o.speed);
    o.speed_non_human = int("velocidadeNH", o.speed_non_human);
    o.show_ai_penalties = int("verDecisaoPenNaoHumano", o.show_ai_penalties);
    o.show_coach_changes = int("verMudancaTecnicos", o.show_coach_changes);
    o.show_substitutions_window = int("verJanelaSubs", o.show_substitutions_window);
    o.contract_end_warning = int("avisoTerminoContrato", o.contract_end_warning);
    o.auto_renew_contracts = flag("autoRenovaContrato", o.auto_renew_contracts);
    o.use_scoreboard_color = flag("usaCorPlacar", o.use_scoreboard_color);
    o.scoreboard_color = int("corPlacar", o.scoreboard_color);
    o.use_sounds = flag("usaSons", o.use_sounds);
    o.bold_home = flag("negritoCasa", o.bold_home);
    o.use_list_colors = flag("usaCoresLista", o.use_list_colors);
    o.show_states_grouped = flag("verEstaduaisAgrupados", o.show_states_grouped);
    o.ignore_leagues = flag("ignoraLigas", o.ignore_leagues);
    o.ignore_state = flag("ignoraEstadual", o.ignore_state);
    o.use_real_cup_groups = flag("UsarGruposReaisCopa", o.use_real_cup_groups);
    o.regionals_all = flags("jogaRegionaisTodos", &o.regionals_all);
    o.regionals_invitation = flag("conviteRegionais", o.regionals_invitation);
    o.regionals_without_humans = int("regionaisSemHumanos", o.regionals_without_humans);
    o.new_cup_format = flag("novoFormatoCopa", o.new_cup_format);
    o.theme_color = int("corTema", o.theme_color);
    o.show_auctions = flag("verLeiloes", o.show_auctions);
    o
}
