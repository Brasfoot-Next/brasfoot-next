//! Port of `at.mA()[0]` in every competition class: the picture of a competition's trophy, a
//! file stem of the game folder's `trofeus/` (`aq.x(name)`). `exists` stands for `aq.w(name)`,
//! whether that picture is there.

use brasfoot_model::countries;
use brasfoot_model::ids::CompetitionId;
use brasfoot_model::saved::CompetitionKind;
use brasfoot_model::world::World;

/// `aq.rX`: the Brazilian states' codes.
pub const STATE_CODES: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE", "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE",
    "SP", "TO",
];

/// The trophies a gallery leaves out of its header row (`a.dg.a(...)`).
pub const GENERIC: [&str; 3] = ["tr_nacionalgenerico", "tr_supercopa_generico", "tr_estadualgenerico"];

/// `ac.valueOf("P" + id).jA()`
fn country_code(id: i32) -> &'static str {
    countries::country(id).map(|c| c.code).unwrap_or("")
}

/// The picture of the competition's trophy, `""` for the classes without one (`best.at.mA()`).
pub fn trophy_picture(world: &World, c: CompetitionId, exists: &dyn Fn(&str) -> bool) -> String {
    let comp = world.competition(c);
    let pick = |own: String, generic: &str| if exists(&own) { own } else { generic.to_owned() };
    let league_country = || comp.country_ref.map(|k| world.country(k).country).unwrap_or(-1);
    let fixed = match comp.class {
        // f.f
        CompetitionKind::NationalCup => return pick(format!("tr_copa_{}", country_code(league_country())), "tr_copa"),
        // f.I
        CompetitionKind::NationalSuperCup => return pick(format!("tr_supercopa_{}", country_code(comp.country)), "tr_supercopa_generico"),
        // f.B: the division's own, the country's for the first division, or the generic one
        CompetitionKind::NationalLeague => {
            let code = country_code(league_country());
            return division_picture(&format!("tr_nacional_{code}"), comp.division, "tr_nacionalgenerico", exists);
        }
        // f.o
        CompetitionKind::StateChampionship => {
            let state = comp.state_league.and_then(|s| world.state_leagues.get(s.0 as usize)).map(|s| s.state).unwrap_or(-1);
            let code = usize::try_from(state).ok().and_then(|s| STATE_CODES.get(s)).copied().unwrap_or("");
            return division_picture(&format!("tr_estadual_{code}"), comp.division, "tr_estadualgenerico", exists);
        }
        // f.F: an array of five, the last empty
        CompetitionKind::RegionalCup => {
            let names = ["tr_riosaopaulo", "tr_sulminas", "tr_copanordeste", "tr_copaverde", ""];
            let own = comp.region.and_then(|r| usize::try_from(r).ok()).and_then(|r| names.get(r)).copied().unwrap_or("");
            return if exists(own) { own.to_owned() } else { "tr_nacionalgenerico".to_owned() };
        }
        CompetitionKind::AfricaCupOfNations => "tr_copaafrica",
        CompetitionKind::CopaAmerica => "tr_copaamerica",
        CompetitionKind::SuperCupD => "tr_recopaeuropa",
        CompetitionKind::AsianCup => "tr_copaasia",
        CompetitionKind::SuperCupE => "tr_recopasulamaericana",
        CompetitionKind::OceaniaNationsCup => "tr_copaofc",
        CompetitionKind::GoldCup => "tr_copaouro",
        CompetitionKind::CopaSudamericana => "tr_sulamericana",
        CompetitionKind::ConferenceLeague => "tr_conference",
        CompetitionKind::Finalissima => "tr_finalissima",
        CompetitionKind::NationsLeagueM => "tr_liganacoes",
        CompetitionKind::NationsLeagueN => "tr_liganacoesC",
        CompetitionKind::EuropeanChampionship => "tr_eurocopa",
        CompetitionKind::WorldCup => "tr_copamundo",
        CompetitionKind::Libertadores => "tr_libertadores",
        CompetitionKind::CafChampionsLeague => "tr_ligacaf",
        CompetitionKind::AfcChampionsLeague => "tr_ligaafc",
        CompetitionKind::ChampionsLeague => "tr_ligacampeoes",
        CompetitionKind::ConcacafChampionsCup => "tr_ligaconcacaf",
        CompetitionKind::EuropaLeague => "tr_ligaeuropa",
        CompetitionKind::OfcChampionsLeague => "tr_ligaofc",
        CompetitionKind::IntercontinentalCup => "tr_mundial",
        _ => "",
    };
    fixed.to_owned()
}

/// `f.B.mA()`, `f.o.mA()`: `<base>_d<division>`, or `<base>` for the first division, or the
/// generic trophy.
fn division_picture(base: &str, division: i32, generic: &str, exists: &dyn Fn(&str) -> bool) -> String {
    let own = format!("{base}_d{division}");
    if exists(&own) {
        own
    } else if division == 1 && exists(base) {
        base.to_owned()
    } else {
        generic.to_owned()
    }
}

/// A card of the club's gallery (`a.di`, `components.k`): one kind of title, how many seasons
/// and which.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryCard {
    /// The competition whose name and trophy the card shows: the oldest title of the kind.
    pub competition: Option<CompetitionId>,
    pub count: i32,
    /// Seasons (`H()`), newest first.
    pub seasons: Vec<i32>,
}

/// Port of `a.di.py()`: the club's titles (`ah.cT()`) grouped as the gallery shows them, at
/// most sixteen cards.
pub fn club_gallery(world: &World, club: brasfoot_model::ids::ClubId) -> Vec<GalleryCard> {
    let c = world.club(club);
    let trophies = &c.trophies;
    let mut cards = Vec::new();
    // a.di.M(kind, _) and N(kind, sub): newest first, one entry per season
    let mut add = |kind: i32, sub: Option<i32>| {
        let mut card = GalleryCard { competition: None, count: 0, seasons: Vec::new() };
        let mut last_season = -1;
        for t in trophies.iter().rev() {
            if t.competition_kind == kind && sub.is_none_or(|s| t.competition_subkind == s) && t.season != last_season {
                card.count += 1;
                card.seasons.push(t.season);
                card.competition = t.competition;
                last_season = t.season;
            }
        }
        if card.count > 0 && cards.len() < 16 {
            cards.push(card);
        }
    };
    let national_team = world.game.countries.iter().any(|&k| world.country(k).national_team == Some(club));
    let confederation = countries::confederation(c.country);
    if national_team {
        // a.di.AD()
        add(7, Some(7));
        add(7, Some(confederation));
        if confederation == 0 {
            add(14, Some(0));
        } else if confederation == 4 {
            add(14, Some(4));
        }
        if confederation == 0 || confederation == 1 {
            add(13, None);
        }
    } else {
        add(5, None);
        for kind in [4, 6, 12, 8] {
            add(kind, None);
        }
        add(1, Some(1));
        add(2, None);
        add(11, None);
        for division in 2..=4 {
            add(1, Some(division));
        }
        add(10, None);
        for division in 1..=4 {
            add(3, Some(division));
        }
    }
    cards
}

/// A line of the coach's gallery (`a.dg`, `components.aC`).
#[derive(Clone, Debug, PartialEq)]
pub struct CoachTitle {
    pub season: i32,
    pub competition: CompetitionId,
    pub club: Option<brasfoot_model::ids::ClubId>,
}

/// Port of `a.dg.pA()`, `pB()`, `pC()`: the coach's titles in the three columns
/// (international, national, state and regional).
pub fn coach_gallery(world: &World, coach: brasfoot_model::ids::CoachId) -> [Vec<CoachTitle>; 3] {
    let co = world.coach(coach);
    let mut columns: [Vec<CoachTitle>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    // a.dg.d(at, _): past editions of the national teams' competitions won by this coach
    for class in [
        CompetitionKind::WorldCup,
        CompetitionKind::NationsLeagueM,
        CompetitionKind::NationsLeagueN,
        CompetitionKind::EuropeanChampionship,
        CompetitionKind::CopaAmerica,
        CompetitionKind::AfricaCupOfNations,
        CompetitionKind::AsianCup,
        CompetitionKind::GoldCup,
        CompetitionKind::OceaniaNationsCup,
        CompetitionKind::Finalissima,
    ] {
        let Some(i) = world.competitions.iter().position(|x| x.class == class) else { continue };
        let id = CompetitionId(i as u32);
        for edition in &world.competitions[i].titles {
            if edition.champion_coach >= 0 && edition.champion_coach == co.id {
                columns[0].push(CoachTitle { season: edition.season, competition: id, club: world.club_by_game_id(edition.champion) });
            }
        }
    }
    // a.dg.a(kind, -1, list) and b(kind, division, list): the career record, newest first
    let mut add = |column: usize, kind: i32, division: Option<i32>| {
        for t in co.trophies.iter().rev() {
            if t.competition_kind == kind && division.is_none_or(|d| t.competition_subkind == d)
                && let Some(competition) = t.competition
            {
                columns[column].push(CoachTitle { season: t.season, competition, club: world.club_by_game_id(t.club_id) });
            }
        }
    };
    for kind in [5, 4, 6, 12, 8] {
        add(0, kind, None);
    }
    add(1, 1, Some(1));
    add(1, 2, None);
    add(1, 11, None);
    for division in 2..=4 {
        add(1, 1, Some(division));
    }
    add(2, 10, None);
    for division in 1..=4 {
        add(2, 3, Some(division));
    }
    columns
}
