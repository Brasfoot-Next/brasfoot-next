//! State the mod systems keep next to a save. The original writes it to files beside the save
//! (`sav/<save>.economia` and so on); here it travels with the world.

use serde::{Deserialize, Serialize};

use crate::labels::CompetitionLabel;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModState {
    /// `bfmod.Economia`: None until the economy starts on this save (`clubes == null`).
    pub economy: Option<EconomyState>,
    /// `bfmod.Premios`, `HallDaFama` and `Clubes`: what the season-end awards keep.
    #[serde(default)]
    pub awards: AwardsState,
    /// `bfmod.Janelas`: the transfer windows.
    #[serde(default)]
    pub windows: WindowsState,
    /// `bfmod.Jornal`'s clock (`sav/<save>.jornal`): None until it first runs on the save.
    #[serde(default)]
    pub news: Option<NewsState>,
    /// `bfmod.Mensal.ultimoMes` (`sav/<save>.mensal`): the last month paid, as year * 100 + month.
    #[serde(default)]
    pub last_month_paid: Option<i32>,
    /// `bfmod.Joias` (`sav/<save>.joias`).
    #[serde(default)]
    pub gems: Vec<Gem>,
    /// `bfmod.Comissao` (`sav/<save>.comissao`): the coaching staff.
    #[serde(default)]
    pub staff: StaffState,
    /// `bfmod.Base.garotos` (`sav/<save>.base`): the kids the youth academy follows, in the order
    /// they were first seen (`LinkedHashMap` by key). None before the academy first runs.
    #[serde(default)]
    pub academy: Option<Vec<AcademyKid>>,
    /// `bfmod.Torcida` (`sav/<save>.torcida`): the fans' mood beyond the game's own bar.
    #[serde(default)]
    pub fans: FansState,
    /// `bfmod.Patrocinio` (`sav/<save>.patrocinio`): the human club's master sponsor.
    #[serde(default)]
    pub master_sponsor: MasterSponsorState,
    /// `bfmod.Cotas` (`sav/<save>.cotas`): the other sponsorship slots.
    #[serde(default)]
    pub sponsor_slots: SponsorSlotsState,
    /// `bfmod.Investidor` (`sav/<save>.investidor`): investors who want the human club.
    #[serde(default)]
    pub investor: InvestorState,
    /// `bfmod.Patrimonio` (`sav/<save>.patrimonio`): the clubs' infrastructure. None before it
    /// first starts (`estruturas == null`).
    #[serde(default)]
    pub assets: Option<AssetsState>,
    /// `bfmod.Estadios` (`sav/<save>.estadios`): stadiums named after club legends.
    #[serde(default)]
    pub stadiums: StadiumsState,
    /// `bfmod.Emprestimos` (`sav/<save>.emprestimos`): the last match day checked for fines.
    /// None before it first runs on the save.
    #[serde(default)]
    pub loans: Option<LoansState>,
    /// `bfmod.Satelites` (`sav/<save>.satelites`): the multi-club networks. None before the
    /// system first loads on the save.
    #[serde(default)]
    pub satellites: Option<SatellitesState>,
    /// `bfmod.Credito` (`sav/<save>.credito`): the bank's loans. None before it first loads.
    #[serde(default)]
    pub credit: Option<CreditState>,
    /// `bfmod.SuperMundial` (`sav/<save>.supermundial`): the 32-club world cup every four years.
    #[serde(default)]
    pub super_mundial: SuperMundialState,
    /// `bfmod.CampeonatoBase` (`sav/<save>.base_camp`, `.base_hist`): the national U-20 league.
    #[serde(default)]
    pub youth_league: YouthLeagueState,
    /// `bfmod.Olheiros.observacoes` (`sav/<save>.olheiros`): the players the human sent a scout
    /// to watch, in the order sent (`LinkedHashMap` by key).
    #[serde(default)]
    pub scouting: Vec<ScoutObservation>,
    /// `bfmod.Caixa` (`sav/<save>.caixa`): the human club's finances month by month and its
    /// cash day by day, for the finances window.
    #[serde(default)]
    pub cash_book: CashBook,
    /// `bfmod.Jornal.noticias` (`sav/<save>.noticias`): the newspaper's articles.
    #[serde(default)]
    pub articles: Vec<Article>,
}

/// An article in `bfmod.Jornal` ("Gazeta do Brasfoot").
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub id: i32,
    pub date: String,
    pub section: String,
    pub title: String,
    pub body: String,
    pub read: bool,
}

/// An entry of `Olheiros.observacoes`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoutObservation {
    /// `Temporada.chave` of the player.
    pub key: String,
    /// The report came (status 1); else the scout is still watching (0).
    pub done: bool,
    /// When the scout set off (`c.a.SR.bc()`).
    pub since: i64,
}

/// `bfmod.CampeonatoBase`'s state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YouthLeagueState {
    /// `temporada`: the season of the league; -1 before the first.
    pub season: i32,
    /// `pais`: its country.
    pub country: i32,
    /// `encerrado`
    pub finished: bool,
    /// `clubesIds`, `clubesNomes`: the clubs (game id, name), in the order they were picked.
    pub clubs: Vec<(i32, String)>,
    /// `jogos`
    pub matches: Vec<YouthLeagueMatch>,
    /// `estatisticas`: each player's season, in the order they first played (`LinkedHashMap`).
    pub stats: Vec<YouthLeagueStats>,
    /// `titularesPrioritarios`: the juniors the player always wants in his XI (a `HashSet`).
    pub priority_starters: Vec<String>,
    /// `estiloBase`: -1 automatic, 0 balanced, 1 all-out attack, 2 counter-attack.
    pub style: i32,
    /// `marcacaoBase`
    pub marking: i32,
    /// `ladoBase`
    pub side: i32,
    /// `.base_hist`: the champions, newest first.
    pub history: Vec<YouthLeagueChampion>,
}

impl Default for YouthLeagueState {
    fn default() -> Self {
        YouthLeagueState {
            season: -1,
            country: -1,
            finished: false,
            clubs: Vec::new(),
            matches: Vec::new(),
            stats: Vec::new(),
            priority_starters: Vec::new(),
            style: -1,
            marking: 0,
            side: 0,
            history: Vec::new(),
        }
    }
}

/// `CampeonatoBase.JogoBase`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YouthLeagueMatch {
    pub round: i32,
    /// The calendar day it goes with.
    pub day: i32,
    /// The date shown: day, month (from 1), year.
    pub date: (i32, i32, i32),
    /// The clubs' game ids and names.
    pub home: i32,
    pub away: i32,
    pub home_name: String,
    pub away_name: String,
    pub played: bool,
    pub home_goals: i32,
    pub away_goals: i32,
    pub home_shots: i32,
    pub away_shots: i32,
    pub home_possession: i32,
    pub goals: Vec<YouthLeagueGoal>,
}

/// A goal of `JogoBase.gols`: "14' Dardo Salas (Ass: Patricio Marini) [Paris FC]".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YouthLeagueGoal {
    pub minute: i32,
    /// None for a stand-in (`Aspirante`).
    pub scorer: Option<String>,
    /// The engine's goal kind (`A.dz`): 2 own goal, 3 penalty, 4 free kick.
    pub kind: i32,
    pub assist: Option<String>,
    pub club: String,
}

/// `CampeonatoBase.Estatistica`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YouthLeagueStats {
    pub key: String,
    pub name: String,
    pub country: i32,
    pub position: i32,
    pub age: i32,
    /// The club's game id and name.
    pub club: i32,
    pub club_name: String,
    pub games: i32,
    pub goals: i32,
    pub assists: i32,
    pub rating_sum: f64,
}

impl YouthLeagueStats {
    /// `media()`
    pub fn average(&self) -> f64 {
        if self.games > 0 { self.rating_sum / self.games as f64 } else { 0.0 }
    }
}

/// `CampeonatoBase.HistoricoCampeao`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YouthLeagueChampion {
    pub year: i32,
    pub country: i32,
    pub champion: String,
    pub runner_up: String,
    /// Name, club and goals; None for "-".
    pub top_scorer: Option<(String, String, i32)>,
    /// Name, club and average rating; None for "-".
    pub best_player: Option<(String, String, f64)>,
    /// The player's club's place; -1 for none.
    pub player_position: i32,
}

/// `bfmod.SuperMundial`'s state. The maps are by year.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SuperMundialState {
    /// `coeficientes`: each recorded season's points, by club game id.
    pub coefficients: std::collections::BTreeMap<i32, std::collections::BTreeMap<i32, f64>>,
    /// `sedes`, `cidades`: each edition's host country and the final's city.
    pub hosts: std::collections::BTreeMap<i32, (i32, String)>,
    /// `classificados`: each edition's clubs, in the draw's order once drawn.
    pub qualified: std::collections::BTreeMap<i32, Vec<SuperMundialSlot>>,
    /// `dias`: the calendar days of an edition's seven match days.
    pub days: std::collections::BTreeMap<i32, Vec<i32>>,
    /// `pagos`: the match days already paid.
    pub paid: std::collections::BTreeMap<i32, std::collections::BTreeSet<i32>>,
    /// `edicoes`
    pub editions: std::collections::BTreeMap<i32, SuperMundialEdition>,
    /// `renomeado`: the game's yearly world club cup is the Copa Intercontinental now.
    pub renamed: bool,
    /// The game's friendly tournament has the Super Mundial's name (`f.L.getNome()` equal to
    /// `SuperMundial.NOME`, which the original keeps in the save).
    pub tournament_named: bool,
}

/// `SuperMundial.Vaga`: a club's place in an edition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuperMundialSlot {
    /// The club's game id.
    pub club: i32,
    pub name: String,
    pub country: i32,
    /// The confederation, in `SuperMundial.CONFEDERACOES`' order.
    pub confederation: i32,
    pub route: SuperMundialRoute,
    pub coefficient: f64,
    /// The group, 0 for A; -1 before the draw.
    pub group: i32,
}

/// `Vaga.via`: how a club got its place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SuperMundialRoute {
    /// "Campeão <competition> <year>"
    Champion { competition: CompetitionLabel, year: i32 },
    /// "Ranking"
    Ranking,
    /// "País-sede (<country>)": the host's national champion.
    Host { country: i32 },
    /// "Vaga remanejada": a slot a confederation could not fill.
    Reallocated,
}

/// `SuperMundial.Edicao`: an edition played.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SuperMundialEdition {
    pub year: i32,
    /// The host country; -1 for none.
    pub host: i32,
    pub city: String,
    pub champion: String,
    /// The champion's game id.
    pub champion_id: i32,
    pub runner_up: String,
    /// -1 for none.
    pub runner_up_id: i32,
    /// The best player, top scorer and goalkeeper.
    pub best_player: Option<SuperMundialAward>,
    pub top_scorer: Option<SuperMundialAward>,
    pub goalkeeper: Option<SuperMundialAward>,
}

/// A player the edition names (`Edicao.bola`, `chuteira`, `luva`), with his club then.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SuperMundialAward {
    pub name: String,
    pub club: Option<String>,
    pub goals: i32,
    pub assists: i32,
}

/// `Credito.EstagioPenalidade`, in its order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditStage {
    #[default]
    Normal,
    /// `ADVERTENCIA`: late, rating down.
    Warning,
    /// `TRANSFER_BAN`: no signings, 30% of the income seized.
    TransferBan,
    /// `INTERVENCAO`: points lost and forced sales.
    Intervention,
}

/// `Credito.Emprestimo`: a bank loan of a club.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BankLoan {
    pub id: i32,
    pub club: i32,
    /// `valorOriginal`
    pub original: i64,
    /// `saldoDevedor`
    pub balance: i64,
    /// `parcelasTotais`
    pub instalments: i32,
    /// `parcelasPagas`
    pub paid: i32,
    /// `valorParcela`
    pub instalment: i64,
    /// `taxaJurosMensal`
    pub monthly_rate: f64,
    /// `mesesAtraso`: months late.
    pub months_late: i32,
    /// `anoContratacao`
    pub year: i32,
    /// `mesContratacao`
    pub month: i32,
}

/// `Credito.StatusClube`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CreditStatus {
    pub club: i32,
    pub stage: CreditStage,
    /// `pontosDeduzidosAno`
    pub points_deducted: i32,
    /// `anoUltimaDeducao`
    pub last_deduction_year: i32,
    /// `totalRetido`: what the bank took from the club's income.
    pub withheld: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreditState {
    /// `emprestimosPorClube`, by club in insertion order (`LinkedHashMap`).
    pub loans: Vec<(i32, Vec<BankLoan>)>,
    /// `statusPorClube`
    pub statuses: Vec<CreditStatus>,
    /// `proximoId`
    pub next_id: i32,
}

impl Default for CreditState {
    fn default() -> Self {
        CreditState { loans: Vec::new(), statuses: Vec::new(), next_id: 1 }
    }
}

/// `Satelites.Politica`, in its order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SatellitePolicy {
    #[default]
    YoungPlayers,
    Promotion,
    Profit,
}

/// `Satelites.PapelEmprestimo`, in its order; `metaPct` is [`LoanRole::target`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoanRole {
    Starter,
    #[default]
    Regular,
    Reserve,
}

impl LoanRole {
    /// `metaPct`: the share of the games the loanee expects.
    pub fn target(self) -> f64 {
        match self {
            LoanRole::Starter => 0.80,
            LoanRole::Regular => 0.60,
            LoanRole::Reserve => 0.0,
        }
    }
}

/// `Satelites.Vinculo`: a satellite and its parent club, by partnership or ownership.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SatelliteLink {
    pub satellite: i32,
    pub parent: i32,
    /// `comprado`: owned, not a partnership.
    pub owned: bool,
    pub policy: SatellitePolicy,
    /// `taxaAnual`
    pub yearly_fee: i64,
    /// `fimContrato`: the year the partnership ends.
    pub until: i32,
    /// `desdeAno`
    pub since: i32,
    /// `precoPago`
    pub price_paid: i64,
}

/// `Satelites.TaticaSatelite` of an owned satellite.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SatelliteTactic {
    /// `espelhar`: plays the parent's tactic.
    pub mirror: bool,
    /// 0 balanced, 1 all-out attack, 2 counter-attack
    pub style: i32,
    /// 0 light, 1 heavy, 2 very heavy
    pub marking: i32,
    /// 0 through the middle, 1 down the flanks
    pub side: i32,
}

impl Default for SatelliteTactic {
    fn default() -> Self {
        SatelliteTactic { mirror: true, style: 0, marking: 1, side: 0 }
    }
}

/// What a line of a network's history records (`registrarHistorico`); the screens word it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SatelliteEvent {
    Partnership { fee: i64 },
    Renewal { until: i32, fee: i64 },
    Termination { fine: i64 },
    Acquisition { price: i64 },
    ClubSale { price: i64 },
    CashInjection { amount: i64 },
    ProfitWithdrawal { amount: i64, net: i64 },
    Assets { pillar: crate::mods::Pillar, level: i32 },
    Loan { player: String, age: i32, strength: i32, role: LoanRole },
    Return { player: String },
    ToParent { player: String, strength: i32, cost: i64 },
    Definitive { player: String, strength: i32 },
    YouthPull { player: String, age: i32, professional: bool, cost: i64 },
    SellOn { amount: i64, player: Option<String>, buyer: Option<String> },
    AiAcquisition { price: i64, policy: SatellitePolicy },
    AiPartnership { fee: i64 },
    AiLoan { player: String, age: Option<i32> },
    AiPromotion { player: String, strength: i32, cost: i64 },
    AiCession { player: String },
}

/// One line of the networks' history: year, the clubs' names (data) and the event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SatelliteNote {
    pub year: i32,
    pub parent: Option<String>,
    pub satellite: Option<String>,
    pub event: SatelliteEvent,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SatellitesState {
    /// `vinculos`, by satellite, in insertion order (`LinkedHashMap`).
    pub links: Vec<SatelliteLink>,
    /// `taticas`, by satellite id.
    pub tactics: Vec<(i32, SatelliteTactic)>,
    /// `papeis`, by player key (`Temporada.chave`).
    pub roles: Vec<(String, LoanRole)>,
    /// `historico`, at most 120 lines.
    pub history: Vec<SatelliteNote>,
    /// `ultimaTemporada`
    pub last_season: i32,
    /// `iniciadoRedesIA`
    pub ai_started: bool,
}

impl SatellitesState {
    pub fn link(&self, satellite: i32) -> Option<&SatelliteLink> {
        self.links.iter().find(|l| l.satellite == satellite)
    }

    pub fn link_mut(&mut self, satellite: i32) -> Option<&mut SatelliteLink> {
        self.links.iter_mut().find(|l| l.satellite == satellite)
    }

    /// `vinculos.put`: replaces in place or adds at the end.
    pub fn put(&mut self, link: SatelliteLink) {
        match self.links.iter_mut().find(|l| l.satellite == link.satellite) {
            Some(l) => *l = link,
            None => self.links.push(link),
        }
    }

    pub fn remove(&mut self, satellite: i32) {
        self.links.retain(|l| l.satellite != satellite);
    }
}

/// `Emprestimos.temporadaVista`, `diaVisto`
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoansState {
    pub season_seen: i32,
    pub day_seen: i32,
}

/// `Estadios.Renomeado`: a club's stadium renamed after a legend, once in its history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StadiumRename {
    /// The club's game id.
    pub club: i32,
    pub year: i32,
    pub old: String,
    pub new: String,
    /// The legend's player key.
    pub legend: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StadiumsState {
    /// `renomeados`, by club, in the order they were renamed (`LinkedHashMap`).
    pub renamed: Vec<StadiumRename>,
    /// `nunca`: the player said "never".
    pub never: bool,
    /// `pendente`: the board's proposal waits for the next main screen.
    pub pending: bool,
    /// The proposal on the screen (the original's modal dialog), waiting for an answer.
    #[serde(default)]
    pub shown: bool,
}

/// `Patrimonio.Pilar`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Pillar {
    /// `ARENA`: the arena's modernization.
    Arena,
    /// `CT`: the training centre and medical department.
    TrainingCentre,
    /// `ACADEMIA`: the youth academy.
    Academy,
    /// `MARKETING`: global marketing.
    Marketing,
}

impl Pillar {
    pub const ALL: [Pillar; 4] = [Pillar::Arena, Pillar::TrainingCentre, Pillar::Academy, Pillar::Marketing];
}

/// `Patrimonio.Estrutura`: a club's infrastructure.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClubAssets {
    /// The club's game id.
    pub id: i32,
    pub arena: i32,
    pub training_centre: i32,
    pub academy: i32,
    pub marketing: i32,
    /// `nomeBaseEstadio`: the stadium's name without its naming-rights brand.
    pub stadium_base_name: String,
    /// `marcaNaming`: the naming-rights brand, empty for none.
    pub naming_brand: String,
    pub naming_yearly: i64,
    /// `fimNaming`: the last year of the naming rights.
    pub naming_until: i32,
    /// `campanhaTemporada`: the season of the last marketing campaign.
    pub campaign_season: i32,
    /// `tipoCampanha`: 1 to 3.
    pub campaign_kind: i32,
    /// `ultimaPeneira`: the season of the last youth trials.
    pub last_trials: i32,
}

impl ClubAssets {
    pub fn level(&self, pillar: Pillar) -> i32 {
        match pillar {
            Pillar::Arena => self.arena,
            Pillar::TrainingCentre => self.training_centre,
            Pillar::Academy => self.academy,
            Pillar::Marketing => self.marketing,
        }
    }

    pub fn set_level(&mut self, pillar: Pillar, level: i32) {
        match pillar {
            Pillar::Arena => self.arena = level,
            Pillar::TrainingCentre => self.training_centre = level,
            Pillar::Academy => self.academy = level,
            Pillar::Marketing => self.marketing = level,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetsState {
    /// `estruturas`, in the order the clubs joined (`LinkedHashMap`).
    pub clubs: Vec<ClubAssets>,
    /// `ultimaTemporada`
    pub last_season: i32,
}

impl AssetsState {
    pub fn club(&self, id: i32) -> Option<&ClubAssets> {
        self.clubs.iter().find(|c| c.id == id)
    }
}

/// `Patrocinio.Contrato.tipo`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasterKind {
    /// `Fixo`
    Fixed,
    /// `Misto`
    Mixed,
    /// `Arriscado`
    Risky,
}

/// `Patrocinio.Contrato`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MasterContract {
    /// Index into `Patrocinio.MARCAS` (the names are in the catalog).
    pub brand: u16,
    pub kind: MasterKind,
    pub yearly: i64,
    pub bonus_top4: i64,
    pub bonus_league: i64,
    pub bonus_continental: i64,
    /// Seasons left.
    pub years: i32,
}

/// What a master sponsor's season-end bonus paid for (`Patrocinio.fimDeTemporada`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MasterBonusReason {
    /// "{n}º lugar"
    Place(i32),
    /// "título da liga"
    LeagueTitle,
    /// "título continental"
    ContinentalTitle,
}

/// `Patrocinio.historico`: the original keeps the Portuguese sentences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MasterNote {
    /// "{year}: assinou com {brand} ({kind}) - {description}"
    Signed { year: i32, contract: MasterContract },
    /// "{year}: bônus de {amount} ({reasons})"
    Bonus { year: i32, amount: i64, reasons: Vec<MasterBonusReason> },
    /// "{year}: rescindiu com {brand} (multa de {fine})"
    Ended { year: i32, brand: u16, fine: i64 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MasterSponsorState {
    /// `atual`: kept after its last season (`anos` 0) until the next one is signed.
    pub contract: Option<MasterContract>,
    pub history: Vec<MasterNote>,
    /// The three offers the player has to choose from (the original's modal dialog, opened by
    /// the tick that drew them); empty when none is waiting.
    #[serde(default)]
    pub offers: Vec<MasterContract>,
}

impl MasterSponsorState {
    /// `Patrocinio.contrato()`: the contract while it has seasons left.
    pub fn active(&self) -> Option<&MasterContract> {
        self.contract.as_ref().filter(|k| k.years > 0)
    }
}

/// `Cotas.Cota`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SponsorSlot {
    /// `MANGA`
    Sleeve,
    /// `COSTAS`
    Back,
    /// `CALCAO`
    Shorts,
    /// `MATERIAL`: the kit supplier.
    Kit,
    /// `ESTADIO`: the stadium's naming rights.
    Stadium,
    /// `PARCEIRO`: the official partner.
    Partner,
}

impl SponsorSlot {
    pub const ALL: [SponsorSlot; 6] = [SponsorSlot::Sleeve, SponsorSlot::Back, SponsorSlot::Shorts, SponsorSlot::Kit, SponsorSlot::Stadium, SponsorSlot::Partner];

    /// `Cota.peso`: the share of what a master sponsor of the club is worth.
    pub fn weight(self) -> f64 {
        match self {
            SponsorSlot::Sleeve => 0.22,
            SponsorSlot::Back => 0.18,
            SponsorSlot::Shorts => 0.10,
            SponsorSlot::Kit => 0.30,
            SponsorSlot::Stadium => 0.35,
            SponsorSlot::Partner => 0.08,
        }
    }
}

/// `Cotas.Contrato.tipo`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    /// `Fixo`
    Fixed,
    /// `Curto`
    Short,
    /// `Com bônus`
    WithBonus,
}

/// `Cotas.Contrato`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlotContract {
    pub slot: SponsorSlot,
    /// Indices into `Economia.MARCA_A` and `Cotas.RAMOS` (the words are in the catalog).
    pub brand: (u16, u16),
    pub kind: SlotKind,
    pub yearly: i64,
    pub bonus: i64,
    pub years: i32,
    /// Offers only: the last day to sign (yyyymmdd); 0 once signed.
    pub valid_until: i32,
}

/// `Cotas.historico`: the original keeps the Portuguese sentences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SlotNote {
    /// "{year}: {slot} com {brand} ({kind}) - {description}"
    Signed { year: i32, contract: SlotContract },
    /// "{year}: bônus de título das cotas, {amount}"
    TitleBonus { year: i32, amount: i64 },
    /// "{year}: rescindiu {slot} com {brand} (multa de {fine})"
    Ended { year: i32, slot: SponsorSlot, brand: (u16, u16), fine: i64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SponsorSlotsState {
    /// `clube`: the game id of the club the deals are for, -1 for none.
    pub club: i32,
    pub deals: Vec<SlotContract>,
    pub offers: Vec<SlotContract>,
    /// `buscas`: the slots the commercial department is looking for, with the day the answer
    /// arrives (yyyymmdd), in the order of the slots (`EnumMap`).
    pub searches: Vec<(SponsorSlot, i32)>,
    pub history: Vec<SlotNote>,
}

impl Default for SponsorSlotsState {
    fn default() -> Self {
        SponsorSlotsState { club: -1, deals: Vec::new(), offers: Vec::new(), searches: Vec::new(), history: Vec::new() }
    }
}

/// `Investidor`'s pending offer: day (yyyymmdd), investor, money now, yearly injection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InvestorOffer {
    pub day: i32,
    /// Indices into `Economia.DONO_A` and `DONO_B`.
    pub investor: (u16, u16),
    pub now: i64,
    pub yearly: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InvestorState {
    /// `anoSorteado`: the last year the yearly roll was made.
    pub year_drawn: i32,
    /// `anoPago`: the last year the owner's injection was paid.
    pub year_paid: i32,
    /// `anoAvaliado`: the last season the owner judged.
    pub year_judged: i32,
    /// `clube`: the club the patience refers to (game id, -1 for none).
    pub club: i32,
    /// `cobrancas`: seasons in a row below expectations.
    pub complaints: i32,
    /// `dia`, `investidor`, `injecao`, `aporte` (the file writes the offer while `dia` > 0).
    pub offer: Option<InvestorOffer>,
    /// The offer shown to the player (the original's modal dialog), waiting for an answer.
    #[serde(default)]
    pub shown: Option<InvestorOffer>,
}

impl Default for InvestorState {
    fn default() -> Self {
        InvestorState { year_drawn: 0, year_paid: 0, year_judged: 0, club: -1, complaints: 0, offer: None, shown: None }
    }
}

/// `Torcida`'s state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FansState {
    /// `elenco`: the squad last seen, player key and strength, in the order of the Java
    /// `HashMap` it is kept in.
    pub squad: Vec<(String, i32)>,
    /// `base`: the keys of the club's youth players last seen, in the order of the `HashSet`.
    pub academy: Vec<String>,
    /// `motivos`: the last ten reasons the mood moved, oldest first.
    pub reasons: Vec<FanReason>,
}

impl FansState {
    /// The side file had no line (`Clubes.linhas(...).isEmpty()`).
    pub fn is_empty(&self) -> bool {
        self.squad.is_empty() && self.academy.is_empty() && self.reasons.is_empty()
    }
}

/// `Torcida.motivos`: {day, change, why}.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FanReason {
    /// The calendar day's date (`c.a.SR.T()`).
    pub day: i64,
    pub change: i32,
    pub why: FanEvent,
}

/// Why the fans' mood moved; the original keeps the Portuguese sentence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FanEvent {
    /// "Garoto da base {name} sobe ao profissional"
    AcademyPromotion { player: String },
    /// "Chega um craque mundial: {name}"
    WorldStarSigned { player: String },
    /// "Contratação de impacto: {name}"
    ImpactSigning { player: String },
    /// "Venda do ídolo {name} para o {club}"
    IdolSold { player: String, club: String },
    /// "Saída de {name}, um dos melhores do elenco"
    TopPlayerLeft { player: String },
    /// "Novo dono: {investor}" (`Investidor.aceitar`); indices into `Economia.DONO_A`, `DONO_B`.
    NewOwner { investor: (u16, u16) },
    /// "O dono desistiu do clube" (`Investidor.fimDeTemporada`)
    OwnerGaveUp,
    /// "Conforto e hospitalidade da Arena" (`Patrimonio.pagarMes`)
    ArenaComfort,
    /// "Inauguração: {level's name}" (`Patrimonio.evoluir`)
    Inauguration { pillar: Pillar, level: i32 },
    /// "Venda de Naming Rights do estádio ({stadium})" (`Patrimonio.assinarNaming`)
    NamingRightsSold { stadium: String },
    /// The campaign of `Patrimonio.realizarCampanha` (1 to 3).
    Campaign { kind: i32 },
    /// "Estádio rebatizado: {stadium}" (`Estadios.proposta`)
    StadiumRenamed { stadium: String },
    /// "Título do Campeonato de Base Sub-20" (`CampeonatoBase.finalizarTemporada`)
    YouthLeagueTitle,
    /// "Vice-campeão do Campeonato de Base Sub-20"
    YouthLeagueRunnerUp,
}

/// `Base.Garoto`: a kid with his hidden ceiling, followed until 25.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcademyKid {
    /// `Temporada.chave` of the player.
    pub key: String,
    /// `teto`: the hidden ceiling, 30 to 95.
    pub ceiling: i32,
    /// `idadeDev`: the development age, run by the calendar in the academy and by minutes once
    /// professional.
    pub development_age: f64,
    /// `erro`: the scouts' error in standard deviations, fixed per kid.
    pub scout_error: f64,
    /// `profissional`
    pub professional: bool,
    /// `avisado`: the player was told he is ready.
    pub notified: bool,
    /// `anoEntrada`
    pub year_joined: i32,
    /// `ultimoT`: the season time (season index plus the year's fraction) of the last run.
    pub last_time: f64,
}

/// `Comissao.Cargo`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StaffRole {
    /// `ANALISTA`: the performance analyst.
    Analyst,
    /// `AUXILIAR`: the assistant coach.
    Assistant,
    /// `PREPARADOR`: the fitness coach.
    FitnessCoach,
}

impl StaffRole {
    pub const ALL: [StaffRole; 3] = [StaffRole::Analyst, StaffRole::Assistant, StaffRole::FitnessCoach];
}

/// `Comissao.Profissional`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaffMember {
    pub role: StaffRole,
    /// Indices into the mod's first names and surnames (`NOMES`, `SOBRENOMES`; the words are in
    /// the catalog).
    pub name: (u16, u16),
    /// 1 to 5.
    pub level: i32,
    /// Monthly.
    pub salary: i64,
    /// Index into `PERFIS`.
    pub profile: u16,
    /// The year hired; 0 for a candidate.
    pub since: i32,
}

/// `Comissao`'s state: the staff follows the coach, not the club.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StaffState {
    /// `contratados`, one per role, in the order of the roles (`EnumMap`).
    pub hired: Vec<StaffMember>,
    pub candidates: Vec<StaffMember>,
    /// `mesCandidatos`: the month the candidates were drawn for (year * 100 + month), 0 for none.
    pub candidates_month: i32,
}

impl StaffState {
    pub fn hired(&self, role: StaffRole) -> Option<&StaffMember> {
        self.hired.iter().find(|m| m.role == role)
    }
}

/// `Jornal`'s clock: the day and the season it last ran for (`ultimaData`, `ultimaTemporada`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewsState {
    pub last_day: i64,
    pub last_season: i32,
}

/// `Joias.Joia`: a revealed gem of the youth academies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gem {
    pub key: String,
    pub name: String,
    pub country: i32,
    pub position: i32,
    pub club: Option<String>,
    pub year_revealed: i32,
    pub age_revealed: i32,
    /// `forcaInicial`: -1 (`DA_BASE`) for the academy's gems, developed by `Base`.
    pub initial_strength: i32,
    pub target: i32,
    pub debut: i32,
}

/// `bfmod.Janelas`'s state (`sav/<save>.janelas`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowsState {
    /// `atual`: "year|n" of the window being worked on.
    pub current: Option<String>,
    /// `avisos`: "year|n|abriu", "year|n|fechou", "year|n|semana" (a `HashSet`).
    pub notices: Vec<String>,
    /// `plano`: buyer club id, signings left, budget left (a `LinkedHashMap`).
    #[serde(default)]
    pub plan: Vec<(i32, i64, i64)>,
    /// `planejados`, `executados`, `propostas`, `pedidos`
    #[serde(default)]
    pub planned: i32,
    #[serde(default)]
    pub executed: i32,
    #[serde(default)]
    pub offers: i32,
    #[serde(default)]
    pub requests: i32,
    /// `negocios`: the window's signings by the AI (fee, player, from, to: names).
    #[serde(default)]
    pub deals: Vec<(i64, String, String, String)>,
    /// `PRE`: a pre-season window opened late, its year and first and last day of the year
    /// (`preAno`, `preInicio`, `preFim`; 0 when none).
    pub late_preseason: Option<(i32, i32, i32)>,
}

/// How a club's master sponsorship is paid (`Economia.Clube.tipoPatrocinio`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SponsorshipKind {
    /// `""`: no sponsor yet.
    #[default]
    None,
    /// `Fixo`
    Fixed,
    /// `Misto`
    Mixed,
    /// `Arriscado`
    Risky,
    /// `Ligado ao dono`
    OwnerLinked,
}

/// `Economia.Clube`: an AI club's money as the mod sees it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomyClub {
    /// The club's game id.
    pub id: i32,
    pub fortune: i64,
    pub has_owner: bool,
    /// The owner's yearly money.
    pub owner_funding: i64,
    pub owner_since: i32,
    /// The sponsor's brand: indices into the two word lists (`MARCA_A`, `MARCA_B`).
    pub brand: Option<(u16, u16)>,
    pub sponsorship_kind: SponsorshipKind,
    pub sponsorship: i64,
    pub sponsorship_until: i32,
    /// Last year of a new owner's spending spree (0 when none).
    pub project_until: i32,
    /// The owner's name: indices into `DONO_A` and `DONO_B`.
    pub owner_name: Option<(u16, u16)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomyState {
    /// In the order the clubs joined (`LinkedHashMap`).
    pub clubs: Vec<EconomyClub>,
    /// Transfers already charged, by signature (a `HashSet`; a sequence in the save as before).
    pub processed: std::collections::BTreeSet<String>,
    pub last_season: i32,
    pub cleaned: bool,
    /// `eventos`: owners arriving and leaving.
    #[serde(default)]
    pub events: Vec<OwnerEvent>,
}

/// `Economia.eventos`: {year, club name, "Novo dono" or "Dono saiu"}.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnerEvent {
    pub year: i32,
    pub club: String,
    /// True for a new owner, false for one who left.
    pub arrived: bool,
}

impl EconomyState {
    pub fn club(&self, id: i32) -> Option<&EconomyClub> {
        self.clubs.iter().find(|c| c.id == id)
    }

    pub fn club_mut(&mut self, id: i32) -> Option<&mut EconomyClub> {
        self.clubs.iter_mut().find(|c| c.id == id)
    }
}

/// The season-end awards' history: the side files `.premios` ([`Award`]), `.carreiras`
/// ([`Career`]), `.recordes` ([`RecordMark`]), `.clubes` ([`ClubSeason`]) and `.idolos`
/// ([`Idol`]), each list in the order of its file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AwardsState {
    /// By year, then kind.
    pub awards: Vec<Award>,
    /// Notable careers, also after the player retires (`LinkedHashMap` by key).
    pub careers: Vec<Career>,
    /// The top ten of each record, grouped by [`RecordKind`] in its order.
    pub records: Vec<RecordMark>,
    /// One row per league club and season, by year then club id.
    pub club_seasons: Vec<ClubSeason>,
    /// Each club's ten top scorers and ten most capped players, grouped by club.
    pub idols: Vec<Idol>,
}

/// `Premios.Tipo`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AwardKind {
    /// `BOLA`
    BallonDor,
    /// `CHUTEIRA`
    GoldenShoe,
    /// `LUVA`
    GoldenGlove,
    /// `GOLDEN_BOY`
    GoldenBoy,
    /// `SELECAO`
    TeamOfTheYear,
    /// `TECNICO`
    CoachOfTheYear,
}

impl AwardKind {
    pub const ALL: [AwardKind; 6] =
        [AwardKind::BallonDor, AwardKind::GoldenShoe, AwardKind::GoldenGlove, AwardKind::GoldenBoy, AwardKind::TeamOfTheYear, AwardKind::CoachOfTheYear];
}

/// `Premios.Premio`: one award of one year. A coach's award has only the name, the club and
/// its country (the other figures stay at zero).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Award {
    pub year: i32,
    pub kind: AwardKind,
    /// The player key (`Temporada.chave`), or `tec|` and the coach's name.
    pub key: String,
    pub name: String,
    /// The club's name then; None for a player without one (`-`).
    pub club: Option<String>,
    pub country: i32,
    pub position: i32,
    pub age: i32,
    pub rating: f64,
    pub goals: i32,
    pub assists: i32,
    pub extra: AwardExtra,
}

/// `Premio.extra`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AwardExtra {
    #[default]
    None,
    /// The Golden Shoe's goals times the league weight.
    WeightedGoals(f64),
    /// The place in the team of the year.
    Slot(TeamSlot),
    /// The coach of the year's season.
    Coach(CoachSummary),
}

/// The places of the team of the year (`Temporada.selecao`), a 4-3-3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamSlot {
    Goalkeeper,
    RightBack,
    CentreBack,
    LeftBack,
    Midfielder,
    Forward,
}

/// `Tecnicos.Candidato.resumo()`: the league campaign and the titles.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CoachSummary {
    pub campaign: Option<LeagueCampaign>,
    pub titles: Vec<SeasonTitle>,
}

/// The coach's club in its national league (`Tecnicos.Campanha` as `resumo()` shows it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeagueCampaign {
    pub league: CompetitionLabel,
    pub position: i32,
    /// The club's rank in the league by squad strength.
    pub expected: i32,
}

/// A title of a season (`Temporada.titulos`): won, or the runner-up place of a big one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeasonTitle {
    pub runner_up: bool,
    /// The competition's name then. The mod keeps the label it shows, its runs of spaces made
    /// one (`replaceAll("\\s+", " ")`), so that is done on the text.
    pub competition: CompetitionLabel,
}

/// `HallDaFama.Carreira`: a player's whole career, kept after he retires.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Career {
    pub key: String,
    pub name: String,
    pub country: i32,
    pub position: i32,
    pub games: i32,
    pub goals: i32,
    pub assists: i32,
    pub rating_sum: f64,
    pub ratings: i32,
    /// The club's name; None without one (`-`).
    pub club: Option<String>,
    pub age: i32,
    pub last_year: i32,
    /// The last year played before retiring; 0 while active.
    pub retired: i32,
}

impl Career {
    /// `Carreira.nota()`
    pub fn rating(&self) -> f64 {
        if self.ratings > 0 { self.rating_sum / self.ratings as f64 } else { 0.0 }
    }
}

/// `HallDaFama.Recorde`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RecordKind {
    /// `GOLS`: goals in a season.
    Goals,
    /// `ASSISTENCIAS`
    Assists,
    /// `PARTICIPACOES`: goals and assists.
    Contributions,
    /// `NOTA`: the average rating, with 20 games and 10 ratings at least.
    Rating,
}

impl RecordKind {
    pub const ALL: [RecordKind; 4] = [RecordKind::Goals, RecordKind::Assists, RecordKind::Contributions, RecordKind::Rating];
}

/// `HallDaFama.Marca`: a season's figure in a record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordMark {
    pub kind: RecordKind,
    pub value: f64,
    pub key: String,
    pub name: String,
    /// The club he played most for that season; None for none (`-`).
    pub club: Option<String>,
    pub country: i32,
    pub year: i32,
}

/// `Clubes.Linha`: a league club's season.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClubSeason {
    pub year: i32,
    /// The club's game id.
    pub club: i32,
    pub name: String,
    pub country: i32,
    /// The national league; None for a past season.
    pub league: Option<CompetitionLabel>,
    /// 1 for the first division; 0 without a league.
    pub division: i32,
    pub position: i32,
    /// The league's size.
    pub clubs: i32,
    pub titles: Vec<SeasonTitle>,
    /// The club's top scorer of the season and his goals.
    pub top_scorer: Option<(String, i32)>,
    pub coach: Option<String>,
}

/// `Clubes.Idolo`: a player's games and goals for one club.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Idol {
    /// The club's game id.
    pub club: i32,
    pub key: String,
    pub name: String,
    pub country: i32,
    pub position: i32,
    pub games: i32,
    pub goals: i32,
}

/// `Caixa.Mes`: the season's running totals at the last tick of a month (`yyyymm`), in the order
/// of the income lines then the expense lines.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CashMonth {
    pub month: i32,
    pub season: i32,
    /// The club's game id.
    pub club: i32,
    pub cash: i64,
    pub values: Vec<i64>,
}

/// `Caixa`'s state: the months (`meses`) and the cash of each day shown (`dias`, `yyyymmdd`, at
/// most 400).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CashBook {
    pub months: std::collections::BTreeMap<i32, CashMonth>,
    pub days: std::collections::BTreeMap<i32, i64>,
}
