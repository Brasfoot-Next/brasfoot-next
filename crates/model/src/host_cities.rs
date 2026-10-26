//! The cities the game names as hosts of its national teams' finals, hard-coded in its classes:
//! proper names kept in the save, so data like the clubs' names, not catalog text. Each table is
//! the original's array, in its order, with the countries it pairs them with.

/// `f.K.agj`, `f.K.agk`: the Finalíssima's cities in South America and their countries.
pub const FINALISSIMA_SOUTH_AMERICA: [(&str, i32); 9] = [
    ("Buenos Aires", 11),
    ("Rio de Janeiro", 29),
    ("São Paulo", 29),
    ("Santiago", 42),
    ("Lima", 151),
    ("Belo Horizonte", 29),
    ("Motevidéu", 195),
    ("Quito", 60),
    ("Bogotá", 46),
];

/// `f.K.agl`, `f.K.agm`: the Finalíssima's cities in Europe and their countries.
pub const FINALISSIMA_EUROPE: [(&str, i32); 14] = [
    ("Roma", 104),
    ("Madrid", 65),
    ("Kiev", 193),
    ("Varsóvia", 152),
    ("Paris", 72),
    ("Lisboa", 154),
    ("Berlim", 3),
    ("Milão", 104),
    ("Munique", 3),
    ("Londres", 97),
    ("Liverpool", 97),
    ("Viena", 15),
    ("Budapeste", 88),
    ("Istambul", 192),
];

/// `SuperMundial.SEDES`: the Super Mundial's real bidders, country and city, in its order.
pub const SUPER_MUNDIAL_BIDDERS: [(i32, &str); 7] =
    [(29, "Rio de Janeiro"), (65, "Madri"), (154, "Lisboa"), (97, "Londres"), (68, "Nova York"), (9, "Riade"), (129, "Casablanca")];

/// `cidades.getOrDefault(ano, "Nova York")`: the city of an edition without a host.
pub const SUPER_MUNDIAL_DEFAULT_CITY: &str = "Nova York";

/// `best.M.getString("mundialclubes")` in the game's Portuguese data: the name the game gives
/// its yearly world club cup (`f.A`), which a save keeps.
pub const GAME_CLUB_WORLD_CUP_NAME: &str = "Mundial Clubes";

/// `SuperMundial.NOME`: the name the mod gives the game's friendly tournament for an edition,
/// kept in the save.
pub const SUPER_MUNDIAL_NAME: &str = "Super Mundial de Clubes";
