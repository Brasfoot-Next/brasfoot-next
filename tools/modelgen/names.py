"""The obfuscated-to-readable dictionary of the classes the original game saves.

The game's obfuscator renamed fields through one global table, so an obfuscated field name means
the same thing in every class (`bW` is always a club id, `ae` always a season). GLOBAL holds those
names; CLASS overrides them for one class. Unknown meanings are marked with UNKNOWN in NOTES and
get a name that says what is known (where it lives, what it holds).
"""

# Java class -> (Rust type name, storage). "arena" classes are shared and referenced by id.
CLASSES = {
    "best.f": ("Game", "root"),
    "best.ay": ("CoachJobMarket", "embedded"),
    "est.InfoArquivoSalvoType": ("SaveSummary", "embedded"),
    "best.F": ("Player", "arena"),
    "best.H": ("CompetitionStats", "embedded"),
    "best.k": ("SeasonStats", "embedded"),
    "best.J": ("MatchRating", "embedded"),
    "best.ah": ("Club", "arena"),
    "best.C": ("ClubFinances", "embedded"),
    "best.v": ("Stadium", "arena"),
    "best.ak": ("StageRecord", "embedded"),
    "best.aj": ("CompetitionRecord", "embedded"),
    "best.ao": ("Trophy", "embedded"),
    "components.ag": ("SavedLineup", "embedded"),
    "best.al": ("Coach", "arena"),
    "best.j": ("CoachSeason", "embedded"),
    "components.as": ("Message", "embedded"),
    "best.I": ("Match", "arena"),
    "best.A": ("MatchEvent", "embedded"),
    "best.a": ("Day", "embedded"),
    "best.at": ("Competition", "arena"),
    "best.i": ("TitleRecord", "embedded"),
    "best.c": ("ScorerRecord", "embedded"),
    "best.e": ("RatingRecord", "embedded"),
    "best.am": ("SeasonSelection", "embedded"),
    "components.am": ("CountrySlots", "embedded"),
    "best.L": ("Stage", "arena"),
    "best.G": ("Group", "embedded"),
    "f.G": ("KnockoutRound", "embedded"),
    "best.Z": ("Country", "arena"),
    "best.w": ("StateLeague", "arena"),
    "best.u": ("CoachChange", "embedded"),
    "best.ap": ("Transfer", "embedded"),
    "components.t": ("Loan", "embedded"),
    "best.T": ("HeadToHead", "embedded"),
    "best.b": ("RetiredIdol", "embedded"),
    "best.B": ("StadiumWork", "embedded"),
}

# Concrete classes folded into a base class's struct, with the kind they become.
SUBCLASSES = {
    "best.at": {
        "f.a": "Friendly", "f.A": "IntercontinentalCup", "f.B": "NationalLeague", "f.c": "AfricaCupOfNations",
        "f.d": "CopaAmerica", "f.D": "SuperCupD", "f.e": "AsianCup", "f.E": "SuperCupE", "f.f": "NationalCup",
        "f.F": "RegionalCup", "f.g": "OceaniaNationsCup", "f.h": "GoldCup", "f.H": "CopaSudamericana",
        "f.i": "AfricanQualifiers", "f.I": "NationalSuperCup", "f.j": "AsianQualifiers", "f.J": "ConferenceLeague",
        "f.k": "ConcacafQualifiers", "f.K": "Finalissima", "f.l": "EuroQualifiers", "f.L": "FriendlyTournament",
        "f.m": "OceaniaQualifiers", "f.M": "NationsLeagueM", "f.n": "SouthAmericanQualifiers", "f.N": "NationsLeagueN",
        "f.o": "StateChampionship", "f.p": "EuropeanChampionship", "f.q": "WorldCup", "f.r": "Libertadores",
        "f.t": "CafChampionsLeague", "f.u": "AfcChampionsLeague", "f.v": "ChampionsLeague", "f.w": "ConcacafChampionsCup",
        "f.x": "EuropaLeague", "f.y": "OfcChampionsLeague",
    },
    "best.L": {"f.s": "League", "f.z": "Knockout"},
}

# Fields that are constant tables re-created by the constructor; not kept in the model.
CONSTANTS = {
    "f.s": ["Zm", "Zn", "Zo", "Zp", "Zq", "Zr", "Zs", "Zt", "Zu", "Zv", "Zw", "Zx", "Zy", "Zz"],
    "f.q": ["Zw"],
    "f.K": ["agj", "agk", "agl", "agm"],
    "best.f": ["bP"],
}

# Fields dropped from the model, with the reason (written to docs/mapa).
DROPPED = {
    ("best.ah", "ei"): "save-time renumbering, replaced by ClubId",
}

GLOBAL = {
    # identity and naming
    "dm": "name", "nome": "name", "mU": "id", "nU": "id", "ei": "legacy_id", "bW": "club_id", "pais": "country",
    "dr": "state", "divisao": "division", "hA": "level", "nc": "reputation", "nu": "reputation_points",
    "mW": "human", "ae": "season", "p": "date", "w": "kind", "dz": "subkind", "Y": "competition", "U": "player",
    "dH": "stadium", "dq": "stadium_condition", "cA": "trophies", "hy": "club",
    # statistics
    "T": "games", "V": "goals", "ca": "yellow_cards", "cb": "red_cards", "cc": "assists", "bX": "wins", "d": "losses",
    "nK": "goals_for", "nL": "goals_against", "nT": "points", "nM": "position", "nN": "round_reached",
    # competitions (best.at) and their stages
    "tR": "kind", "tS": "flag_ts", "tT": "titles", "tU": "scorers", "decisaoTerceiroLugar": "third_place_match",
    "YD": "main_stage", "YE": "countries", "YF": "clubs", "YG": "host", "YH": "edition_counter", "YI": "knockout",
    "YJ": "final_round", "YK": "state_league", "YL": "league", "YM": "list_ym", "YN": "country_yn", "YO": "country_yo",
    "YP": "qualifying_round_1", "YQ": "qualifying_round_2", "YR": "qualifying_round_3", "YS": "clubs_ys",
    "YT": "club_yt", "YU": "club_yu", "YV": "country_slots", "YW": "country_slots_yw", "YX": "flag_yx",
    "ZA": "title_holder", "ZB": "qualifying_round_4", "ZC": "club_zc", "ZD": "counter_zd", "ZE": "clubs_ze",
    "afT": "playoff_round", "KR": "country_ref", "cE": "participants",
    "YY": "competition", "nS": "group_stage", "RT": "country_ref", "YZ": "state_league", "Za": "groups", "Zb": "clubs",
    "Zc": "knockout", "NS": "club_ns", "Zd": "knockout_teams", "duasVoltasMataMata": "two_legged_rounds",
    "as": "numbers", "Ze": "value_ze", "Zf": "round", "var0": "format_code", "Zg": "phase_code",
    "Zi": "qualify_by_overall_table", "pulaDuasDatas": "skip_two_dates", "Zj": "group_count", "nomeLiga": "league_name",
    "nomeDivisao": "division_name", "jogosDentroGrupo": "games_within_group", "doisTurnos": "double_round",
    "nTimes": "team_count", "desempateEstadual": "state_tiebreak", "melhoresTerceiros": "best_third_places",
    "numeroTurnos": "round_count", "Zk": "value_zk",
    "ZF": "rounds", "ZG": "last_round", "ZH": "leg", "ZK": "two_legged_knockout_rounds", "ZL": "rounds_computed",
    "afU": "cup_clubs", "nO": "league_move",
}

CLASS = {
    "best.F": {
        "ei": "save_number", "ej": "save_list",
        "ek": "star", "el": "world_class", "em": "age", "en": "position", "bW": "club", "ep": "unused_ep",
        "eq": "strength", "er": "side", "es": "potential", "et": "salary", "eu": "market_value", "ev": "price",
        "status": "status", "ew": "shirt_number", "ex": "style", "ey": "trait1", "ez": "trait2",
        "eH": "attribute_progress", "eI": "injured_until", "eJ": "contract_until", "eK": "fitness",
        "eL": "started_recently", "eM": "strength_progress", "eN": "last_rating", "eO": "flag_eo",
        "eP": "sold_between_humans", "eQ": "flag_eq", "eR": "slot", "eS": "competition_stats", "eT": "season_stats",
        "eU": "ratings", "eV": "value_ev", "eW": "listed_for_loan", "eX": "on_loan", "eY": "listed_for_sale",
        "eZ": "previous_club", "fa": "value_fa", "fb": "youth", "fc": "estimated_potential", "fd": "development",
        "fe": "development_progress", "ff": "signed_season", "fh": "career_goals", "fi": "career_games",
    },
    "best.H": {"ft": "competition_number", "fu": "rating_sum", "fv": "rated_games"},
    "best.k": {"cd": "injuries", "ce": "rated_games", "ab": "rating_sum"},
    "best.J": {"Z": "rating", "fy": "day", "gx": "match_index"},
    "best.ah": {
        "mV": "file_ref", "mW": "human", "mX": "league_club", "mY": "team_file_id", "na": "coach", "nb": "cash",
        "nd": "squad", "ne": "youth_squad", "nf": "last_starters", "ng": "last_bench", "nh": "captain",
        "ni": "penalty_taker", "nj": "free_kick_taker", "nk": "corner_taker", "nl": "base_color", "cor1": "color1",
        "cor2": "color2", "no": "stage_records", "np": "competition_records", "nq": "tactics", "nr": "training_focus",
        "ns": "training_mode", "nt": "auto_training", "nv": "lineup_set", "hr": "tiebreak_number", "nw": "rivals",
        "nx": "finances", "ny": "saved_lineups", "nz": "saved_lineup", "nA": "kit", "nB": "ranking_points",
        "nC": "all_time_record", "nD": "value_nd",
    },
    "best.C": {
        "dK": "value_dk", "dL": "income_type5", "dM": "income_type3", "dN": "income_transfers", "dO": "income_type6",
        "dP": "income_type9", "dQ": "expense_type7", "dR": "expense_transfers", "dS": "expense_salaries",
        "dT": "expense_type4", "dU": "expense_other", "dV": "expense_type8", "dW": "bank_loan", "dX": "bank_interest",
    },
    "best.v": {"dn": "capacity", "do": "ticket_prices", "dp": "automatic_prices", "dq": "condition"},
    "best.B": {"dH": "stadium", "dI": "until", "dJ": "seats"},
    "best.ak": {"nS": "stage"},
    "best.aj": {"w": "competition_kind", "dz": "competition_subkind"},
    "best.ao": {"w": "competition_kind", "dz": "competition_subkind", "Y": "competition", "bW": "club_id"},
    "components.ag": {"vp": "players", "Ri": "slots"},
    "best.al": {
        "nX": "previous_club", "nY": "previous_division", "bf": "previous_country", "nZ": "country", "hy": "national_team", "oa": "start_season",
        "ob": "seasons", "oc": "points", "W": "games", "od": "wins", "oe": "losses", "of": "board_confidence",
        "og": "fan_support", "oh": "messages", "oi": "seasons_at_club", "bZ": "titles",
    },
    "best.j": {"bY": "points", "bZ": "titles"},
    "components.as": {
        "RU": "sender", "RV": "message_id", "RW": "text1", "RX": "text2", "data": "date_text", "RY": "read",
        "RZ": "flag_rz", "Sa": "text3", "Sb": "text4", "oq": "amount", "Sc": "value_sc", "Sd": "value_sd",
        "Se": "flag_se",
    },
    "best.I": {
        "v": "played", "fw": "stage", "fx": "competition", "fy": "day", "fz": "home", "fA": "away", "fB": "home_goals",
        "fC": "away_goals", "fD": "attendance", "fE": "income", "fF": "home_starters", "fG": "away_starters",
        "fH": "home_bench", "fI": "away_bench", "fJ": "home_played", "fK": "away_played", "fL": "home_list_fl",
        "fM": "away_list_fm", "fN": "events", "fO": "home_strength", "fP": "away_strength", "fQ": "values_fq",
        "fR": "substitutions_left", "fS": "value_fs", "fT": "value_ft", "fU": "value_fu", "fV": "list_fv",
        "fW": "possession", "fX": "values_fx", "fY": "shots", "fZ": "shots_on_target", "ga": "values_ga",
        "gb": "tackles", "gc": "values_gc", "gd": "values_gd", "ge": "knockout", "gf": "penalties_if_level", "gg": "value_gg",
        "gh": "value_gh", "gi": "penalty_shootout", "gj": "went_to_penalties", "gk": "winner", "gl": "lineup_slots",
        "gm": "substitution_minutes", "gn": "tactical_change_minutes", "dq": "stadium_condition",
    },
    "best.A": {
        "dy": "club", "w": "kind", "dz": "detail", "dA": "minute", "dB": "half", "dC": "player", "dD": "other_player",
        "dE": "done", "dF": "value_df", "dG": "flag_dg",
    },
    "best.a": {"q": "tasks_before", "r": "tasks_after", "s": "matches", "t": "stages", "u": "competitions", "v": "done", "w": "kind"},
    "best.at": {"ei": "legacy_id"},
    "best.i": {
        "bQ": "champion", "bR": "runner_up", "bS": "top_scorer_name", "bT": "top_scorer_club", "V": "top_scorer_goals",
        "bU": "champion_coach", "bV": "runner_up_coach",
    },
    "best.c": {"W": "games"},
    "best.e": {"Z": "rating_sum", "aa": "rated_games", "ab": "average", "ac": "value_ac"},
    "best.am": {"oj": "value_oj", "ok": "players", "ol": "clubs"},
    "components.am": {"Rw": "country", "Rx": "slots", "Ry": "flag_ry"},
    "best.G": {"fs": "clubs"},
    "f.G": {
        "aae": "knockout", "oj": "round", "w": "kind", "aaf": "club_count", "gj": "two_legged", "aag": "away_goals",
        "as": "days", "aai": "first_leg", "aaj": "second_leg", "aak": "winners", "aam": "prize",
        "aan": "value_aan", "var0": "format_code",
    },
    "best.Z": {
        "ds": "divisions", "dt": "clubs", "dw": "value_dw", "hu": "cup", "hv": "clubs_hv", "hw": "list_hw",
        "hy": "national_team", "hz": "flag_hz", "hr": "value_hr", "hB": "text_hb", "hC": "flag_hc",
    },
    "best.w": {"dr": "state", "ds": "divisions", "dt": "clubs", "dw": "value_dw"},
    "best.u": {"di": "coach_out", "dj": "coach_in", "dk": "value_dk"},
    "best.ap": {"d": "day", "on": "month", "y": "year", "oo": "from_club", "op": "to_club", "oq": "fee", "U": "player",
                "ei": "player_number", "ej": "player_list"},
    "components.t": {"U": "player", "Pm": "until", "Pn": "owner", "Po": "flag_po"},
    "best.T": {
        "gV": "club1", "gW": "club2", "gX": "goals1", "gY": "goals2", "gZ": "wins1", "ha": "wins2", "hb": "best_win1_kind",
        "hc": "best_win1_season", "hd": "best_win1_for", "he": "best_win1_against", "hf": "best_win2_kind",
        "hg": "best_win2_season", "hh": "best_win2_for", "hi": "best_win2_against",
    },
    "best.b": {"n": "name", "g": "goals", "T": "games"},
    "est.InfoArquivoSalvoType": {"n": "club", "tc": "coach", "a": "year", "i": "next_match"},
    "best.ay": {"Jg": "countries", "Jh": "shuffled_countries", "Jj": "list_jj", "Jl": "clubs"},
    "best.f": {
        "ad": "summary", "ae": "season", "rM": "start_year", "af": "day", "ag": "players", "ah": "youth_players",
        "ai": "generated_players", "aj": "clubs", "ak": "human_clubs", "al": "coaches", "am": "human_coaches",
        "an": "list_an", "ao": "league_countries", "ap": "countries", "aq": "state_leagues", "ar": "national_super_cups",
        "as": "days", "at": "coach_changes", "au": "transfers", "av": "loans", "aw": "stadium_works", "ax": "head_to_head",
        "ay": "list_ay", "az": "retired_idols", "aA": "intercontinental_cup", "aB": "champions_league",
        "aC": "libertadores", "aD": "caf_champions_league", "aE": "afc_champions_league",
        "aF": "concacaf_champions_cup", "aG": "ofc_champions_league", "aH": "friendly", "Gy": "friendly_tournament",
        "aI": "super_cup_e", "aJ": "super_cup_d", "aK": "world_cup", "GC": "nations_league_m", "GI": "nations_league_n",
        "IX": "finalissima", "aL": "south_american_qualifiers", "aM": "euro_qualifiers", "aN": "african_qualifiers",
        "aO": "asian_qualifiers", "aP": "oceania_qualifiers", "aQ": "concacaf_qualifiers", "aR": "copa_america",
        "aS": "european_championship", "aT": "africa_cup_of_nations", "aU": "asian_cup", "aV": "gold_cup",
        "aW": "oceania_nations_cup", "aX": "europa_league", "IY": "conference_league", "aY": "copa_sudamericana",
        "aZ": "regional_cups", "ba": "flag_ba", "bb": "flag_bb", "bc": "flag_bc", "bd": "flag_bd", "be": "value_be",
        "bf": "value_bf", "bg": "next_club_id", "bh": "training_cycle", "bi": "use_real_cup_groups", "bj": "save_name",
        "bk": "season_over", "bl": "world_cup_season", "bm": "euro_season", "bn": "copa_america_season", "IZ": "finalissima_season", "bo": "africa_cup_season",
        "bp": "asian_cup_season", "bq": "oceania_cup_season", "br": "gold_cup_season", "Ja": "nations_league_m_season", "Jb": "nations_league_n_season", "bs": "interactive",
        "bt": "last_coach_id", "bu": "list_bu",
        "jogaRegionaisTodos": "regionals_all", "jogaRegionais": "plays_regionals", "jogaEstadual": "plays_state",
        "usaGrupoPadraoEstadual": "default_state_groups", "jogaIntClubes": "plays_international_clubs",
        "salarioMensal": "monthly_salary", "habilidadeIndividual": "individual_skills",
        "jogaIntano1": "international_in_first_year", "gruposIntPadrao": "default_international_groups",
        "jogaSelecoesAll": "plays_all_national_teams", "verJint": "show_international", "autoSalvar": "autosave",
        "velocidade": "speed", "velocidadeNH": "speed_non_human", "verDecisaoPenNaoHumano": "show_ai_penalties",
        "verMudancaTecnicos": "show_coach_changes", "verJanelaSubs": "show_substitutions_window",
        "avisoTerminoContrato": "contract_end_warning", "ignoraEstadual": "ignore_state", "autoRenovaContrato": "auto_renew_contracts",
        "usaCorPlacar": "use_scoreboard_color", "corPlacar": "scoreboard_color", "usaSons": "use_sounds",
        "negritoCasa": "bold_home", "usaCoresLista": "use_list_colors", "verEstaduaisAgrupados": "show_states_grouped",
        "ignoraLigas": "ignore_leagues", "verLeiloes": "show_auctions",
    },
    "f.A": {"ZM": "quarter_finals", "ZN": "semi_finals", "ZO": "europe_champion", "ZP": "south_america_champion",
            "ZQ": "africa_champion", "ZR": "asia_champion", "ZS": "oceania_champion", "ZT": "concacaf_champion"},
    "f.B": {"YL": "league", "ZU": "second_league", "ZV": "playoff", "ZW": "ratings", "ZX": "players_zx",
            "ZY": "selections_zy", "aaa": "selections_aaa", "KR": "country_ref", "agz": "flag_agz",
            "nRebaixados": "relegated", "duasVoltasplayoffReb": "two_legged_relegation_playoff",
            "duasVoltasMataMataSobe": "two_legged_promotion_knockout", "vagasSobemPeloMataMata": "promotion_knockout_places",
            "rebaixadosDireto": "relegated_directly", "agA": "promotion_knockout", "agB": "relegation_playoff", "GU": "season_finished"},
    "f.F": {"aad": "region"},
    "f.H": {"aao": "clubs_aao"},
    "f.i": {"afV": "knockout_afv", "afW": "club_afw"},
    "f.I": {"aac": "final_stage", "pais": "country"},
    "f.D": {"aac": "final_stage"},
    "f.E": {"aac": "final_stage"},
    "f.K": {"aac": "final_stage", "agg": "club_agg", "agh": "club_agh", "agi": "host_city"},
    "f.j": {"afX": "stage_afx", "afY": "knockout_afy", "afZ": "stage_afz", "aga": "club_aga"},
    "f.k": {"afW": "club_afw", "afX": "stage_afx", "agb": "stage_agb", "agc": "clubs_agc", "agd": "knockout_agd"},
    "f.l": {"afV": "knockout_afv", "age": "value_age"},
    "f.L": {"agn": "text_agn"},
    "f.m": {"agb": "stage_agb", "agf": "club_agf"},
    "f.n": {"agf": "club_agf"},
    "f.M": {"agp": "league_a", "agq": "league_b", "agr": "league_c", "ags": "league_d", "agt": "clubs_a",
            "agu": "clubs_b", "agv": "clubs_c", "agw": "clubs_d", "agx": "knockout_agx", "xc": "list_xc", "agy": "value_agy"},
    "f.N": {"agp": "league_a", "agq": "league_b", "agr": "league_c", "agt": "clubs_a", "agu": "clubs_b",
            "agv": "clubs_c", "xc": "list_xc", "agy": "value_agy"},
    "f.o": {"YK": "state_league", "YL": "league", "nRebaixados": "relegated"},
    "f.x": {"ago": "club_ago"},
    "f.z": {"tR": "competition_kind"},
}

# Element types of lists that every save leaves empty, read from the code that fills them.
LIST_ELEMENTS = {
    ("best.f", "aq"): "best.w", ("best.f", "an"): "best.F", ("best.f", "aw"): "best.B", ("best.f", "ay"): "best.F",
    ("best.f", "bu"): "java.lang.Integer", ("best.Z", "hw"): "best.ah", ("best.I", "fV"): "best.F",
    ("best.at", "tU"): "best.c", ("best.ah", "nw"): "best.ah", ("best.ay", "Jj"): "java.lang.Integer",
    ("f.c", "YE"): "best.Z", ("f.c", "YF"): "best.ah", ("f.d", "YE"): "best.Z", ("f.d", "YF"): "best.ah",
    ("f.e", "YE"): "best.Z", ("f.e", "YF"): "best.ah", ("f.p", "YE"): "best.Z", ("f.p", "YF"): "best.ah",
    ("f.q", "YE"): "best.Z", ("f.q", "YF"): "best.ah", ("f.p", "YM"): "best.ah", ("f.q", "YM"): "best.ah",
    ("f.J", "YF"): "best.ah", ("f.J", "YV"): "components.am", ("f.M", "xc"): "best.ah", ("f.N", "xc"): "best.ah",
    ("f.s", "as"): "java.lang.Integer", ("f.G", "aal"): "best.ah", ("best.w", "ds"): "f.o", ("best.w", "dt"): "best.ah",
    ("f.F", "YF"): "best.ah", ("f.G", "as"): "java.lang.Integer",
}

# String fields that are null in a fresh object and mean something when null.
NULLABLE_STRINGS = {("best.f", "bj"), ("best.i", "bS"), ("est.InfoArquivoSalvoType", "n"),
                    ("est.InfoArquivoSalvoType", "tc"), ("est.InfoArquivoSalvoType", "i"), ("f.K", "agi")}

# Ids stored as ints that the model turns into typed ids.
INT_REFS = {
    ("best.F", "bW"): "ClubId", ("best.al", "bW"): "ClubId", ("best.al", "nX"): "ClubId", ("best.ah", "na"): "CoachId",
}

# Fields read by a hand-written reader of the importer context: (class, field) -> (Rust name,
# Rust type, reader).
CUSTOM = {}

# Transient fields of a saved class, rebuilt on load by the original and by the importer: class ->
# [(Rust name, Rust type, doc, reader)]. They are part of the model and of the canonical walk.
TRANSIENT = {
    # g.b.Ai numbers the players of c.a.SR.O() then c.a.SR.Q() before saving; ap.me() relinks by
    # that number in the list ej names
    "best.ap": [("player", "Option<PlayerId>", "`U` (transient), relinked by `ap.me()` on load", "ctx.transfer_player(node)")],
    # H.ft is the number g.b.Ag gave the competition when saving; g.b.Af links the records of the
    # players of c.a.SR.O() to the calendar's competition of that number on load
    "best.H": [("competition_id", "Option<CompetitionId>", "`Y` (transient), linked by `g.b.Af` on load (`World::link_competition_stats`)", "None")],
}

# Fields the new game adds to a saved class: (Rust name, Rust type, doc). They are absent from
# imported saves and default there.
EXTRA = {
    "best.ah": [
        ("classic_rivals", "Vec<String>", "The club ids of its classic derbies, from a data pack (`rivals` in its `club.toml`); the original's are a fixed table of team files (`I.ic()`)."),
    ],
    "components.as": [
        ("day", "Option<EpochMillis>", "The day of a message the new game sent; imported ones keep `date_text`."),
        ("stage", "Option<StageId>", "The stage a message the new game sent names, where the original writes its name in the text."),
        ("competition", "Option<CompetitionId>", "The competition a message the new game sent names, where the original writes its name in the text."),
        ("money", "Option<i64>", "The amount a message the new game sent names, where the original writes it out in the text."),
        ("confederation", "Option<i32>", "The confederation (`ah.gg()`) of a continental title the new game congratulates, whose cup the original names in the text."),
    ],
}

# Groups of fields stored together as one fixed-size array in the model.
COMBINE = {
    "best.F": [("attributes", ["eA", "eB", "eC", "eD", "eE", "eF", "eG"])],
}

# Fields the canonical comparison leaves out: names the original composes in Portuguese when it
# creates a competition ("Supercopa - França"). The port leaves them empty and the screens
# compose them from the catalog.
# The competitions' `ei` is the number g.b.Ag gives them when saving, in the order the calendar
# days list them; the port keeps direct references instead.
# The stages' league name is "Campeonato <adjective of the country>" (ad.br(int)) unless the
# league's configuration names it: the port leaves the composed one empty as well.
CANONICAL_SKIP = {("best.at", "name"), ("best.L", "name"), ("best.at", "legacy_id"), ("best.L", "league_name"),
                  # the country's name (Z.hB) is catalog text, country-<code>
                  ("best.Z", "text_hb")}
