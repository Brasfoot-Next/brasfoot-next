//! The codes a data pack writes instead of the game's numbers: FIFA trigrams for countries,
//! the two letters of a Brazilian state, and short names for positions, sides and traits.

/// The FIFA trigram of each country of the game, indexed by the game's country id
/// (`best.ac`, see `brasfoot_model::countries::COUNTRIES`). Countries outside FIFA use the code
/// of their confederation (Bonaire `BOE`, Saint Martin `SMN`, Guadeloupe `GLP`...) or, when they
/// have none, ISO 3166 (Monaco `MCO`, Micronesia `FSM`, Marshall Islands `MHL`, Palau `PLW`).
pub const COUNTRY_FIFA: [&str; 224] = [
    "AFG", "RSA", "ALB", "GER", "AND", "ANG", "AIA", "ATG", "CUW", "KSA", // 0
    "ALG", "ARG", "ARM", "ARU", "AUS", "AUT", "AZE", "BAH", "BHR", "BAN", // 10
    "BRB", "BEL", "BLZ", "BEN", "BER", "BLR", "BOL", "BIH", "BOT", "BRA", // 20
    "BRU", "BUL", "BFA", "BDI", "BHU", "CPV", "CMR", "CAM", "CAN", "QAT", // 30
    "KAZ", "CHA", "CHI", "CHN", "CYP", "TLS", "COL", "CGO", "PRK", "KOR", // 40
    "CIV", "CRC", "CRO", "CUB", "DEN", "DJI", "DMA", "EGY", "SLV", "UAE", // 50
    "ECU", "ERI", "SCO", "SVK", "SVN", "ESP", "EST", "ETH", "USA", "FIJ", // 60
    "FIN", "PHI", "FRA", "GAB", "GAM", "GHA", "GEO", "GRN", "GRE", "GUA", // 70
    "GUY", "GUI", "GNB", "EQG", "HAI", "NED", "HON", "HKG", "HUN", "YEM", // 80
    "CAY", "COK", "FRO", "SOL", "VGB", "IND", "IDN", "ENG", "IRN", "IRQ", // 90
    "IRL", "NIR", "ISL", "ISR", "ITA", "MNE", "JAM", "JPN", "JOR", "KEN", // 100
    "KVX", "KUW", "LAO", "LES", "LVA", "LBN", "LBY", "LBR", "LIE", "LTU", // 110
    "LUX", "MAC", "MKD", "MAD", "MAS", "MWI", "MDV", "MLI", "MLT", "MAR", // 120
    "MTN", "MEX", "MYA", "MOZ", "MDA", "MCO", "MNG", "NAM", "NEP", "NCA", // 130
    "NIG", "NGA", "NOR", "NZL", "OMA", "WAL", "PLE", "PAN", "PNG", "PAK", // 140
    "PAR", "PER", "POL", "PUR", "POR", "KGZ", "CTA", "COD", "DOM", "CZE", // 150
    "ROU", "RWA", "RUS", "SAM", "SMR", "LCA", "SKN", "STP", "VIN", "SEN", // 160
    "SLE", "SRB", "SEY", "SGP", "SYR", "SOM", "SRI", "SWZ", "SDN", "SWE", // 170
    "SUI", "SUR", "TJK", "THA", "TAH", "TPE", "TAN", "TOG", "TGA", "TRI", // 180
    "TUN", "TKM", "TUR", "UKR", "UGA", "URU", "UZB", "VAN", "VEN", "VIE", // 190
    "ZAM", "ZIM", "COM", "FSM", "MHL", "MRI", "NRU", "PLW", "KIR", "SSD", // 200
    "TUV", "VIR", "MSR", "TCA", "ASA", "NCL", "GIB", "GLP", "GUM", "MTQ", // 210
    "GUF", "BOE", "SMN", "SMA", // 220
];

/// The game's id of Brazil (`P29`).
pub const BRAZIL: i32 = 29;

/// `aq.rX`: the Brazilian states, indexed by the game's state id.
pub const STATE_CODES: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE", "PI", "PR", "RJ", "RN", "RO",
    "RR", "RS", "SC", "SE", "SP", "TO",
];

/// The positions, indexed by the game's number (`aq.rI`: goleiro, lateral, zagueiro, meia,
/// atacante).
pub const POSITIONS: [&str; 5] = ["GK", "FB", "CB", "MF", "FW"];

/// The sides, indexed by the game's number (`aq.rK`: direito, esquerdo).
pub const SIDES: [&str; 2] = ["R", "L"];

/// The traits, indexed by the game's number (`aq.qN`: colocação, defesa de pênalti, reflexo,
/// saída do gol, armação, cabeceio, cruzamento, desarme, drible, finalização, marcação, passe,
/// resistência, velocidade).
pub const TRAITS: [&str; 14] = [
    "positioning",
    "penalty-saving",
    "reflexes",
    "rushing-out",
    "playmaking",
    "heading",
    "crossing",
    "tackling",
    "dribbling",
    "finishing",
    "marking",
    "passing",
    "stamina",
    "pace",
];

/// The state league formats, indexed by the game's `formula` (`aq.sL`, named after `aq.sN`).
/// Format 7 is São Paulo's 2021 format: in SP's first division it can use the default groups.
pub const STATE_FORMATS: [&str; 11] = [
    "6-teams",
    "8-teams-4-qualify",
    "10-teams-4-qualify",
    "11-teams-4-qualify",
    "12-teams-4-qualify",
    "12-teams-8-qualify",
    "14-teams-8-qualify",
    "16-teams-4-groups-sp-2021",
    "16-teams-4-qualify",
    "16-teams-8-qualify",
    "20-teams-4-groups",
];

pub fn state_format_code(formula: i32) -> Option<&'static str> {
    lookup(&STATE_FORMATS, formula)
}

pub fn state_format_id(code: &str) -> Option<i32> {
    index(&STATE_FORMATS, code)
}

pub fn country_code(id: i32) -> Option<&'static str> {
    usize::try_from(id).ok().and_then(|i| COUNTRY_FIFA.get(i)).copied()
}

pub fn country_id(code: &str) -> Option<i32> {
    COUNTRY_FIFA.iter().position(|c| *c == code).map(|i| i as i32)
}

pub fn state_code(id: i32) -> Option<&'static str> {
    usize::try_from(id).ok().and_then(|i| STATE_CODES.get(i)).copied()
}

pub fn state_id(code: &str) -> Option<i32> {
    STATE_CODES.iter().position(|c| *c == code).map(|i| i as i32)
}

fn lookup(table: &[&'static str], index: i32) -> Option<&'static str> {
    usize::try_from(index).ok().and_then(|i| table.get(i)).copied()
}

fn index(table: &[&str], code: &str) -> Option<i32> {
    table.iter().position(|c| *c == code).map(|i| i as i32)
}

pub fn position_code(position: i32) -> Option<&'static str> {
    lookup(&POSITIONS, position)
}

pub fn position_id(code: &str) -> Option<i32> {
    index(&POSITIONS, code)
}

pub fn side_code(side: i32) -> Option<&'static str> {
    lookup(&SIDES, side)
}

pub fn side_id(code: &str) -> Option<i32> {
    index(&SIDES, code)
}

pub fn trait_code(t: i32) -> Option<&'static str> {
    lookup(&TRAITS, t)
}

pub fn trait_id(code: &str) -> Option<i32> {
    index(&TRAITS, code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brasfoot_model::countries::COUNTRIES;

    #[test]
    fn country_codes_are_unique_and_cover_every_game_country() {
        assert_eq!(COUNTRY_FIFA.len(), COUNTRIES.len());
        for (i, code) in COUNTRY_FIFA.iter().enumerate() {
            assert_eq!(code.len(), 3, "{code}");
            assert_eq!(country_id(code), Some(i as i32), "{code} appears twice");
        }
    }

    #[test]
    fn codes_round_trip() {
        assert_eq!(country_code(BRAZIL), Some("BRA"));
        assert_eq!(country_id("GER"), Some(3));
        assert_eq!(state_id("SP"), Some(25));
        assert_eq!(position_id("FW"), Some(4));
        assert_eq!(trait_code(13), Some("pace"));
        assert_eq!(country_code(224), None);
        assert_eq!(state_code(-1), None);
    }
}
