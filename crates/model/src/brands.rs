//! The names the mod makes up for brands, investors' funds and staff: proper names, so data
//! like the clubs' and players' names, not catalog text. Each table is the original's array, in
//! its order (the saves keep indices into them).

/// `Economia.MARCA_A`: the first word of a sponsor's brand.
pub const BRAND_FIRST: [&str; 30] = [
    "Vortex", "Nexo", "Atlas", "Kairo", "Solaris", "Prisma", "Onda", "Rota", "Pixel", "Fortuna", "Aurora",
    "Titan", "Zenith", "Delta", "Orion", "Vega", "Lumen", "Astra", "Boreal", "Cosmo", "Fênix", "Halo",
    "Ícaro", "Júpiter", "Krona", "Lótus", "Meridia", "Nimbus", "Polar", "Quasar",
];

/// `Economia.MARCA_B`: the second word of a sponsor's brand.
pub const BRAND_SECOND: [&str; 16] = [
    "Bank", "Energia", "Telecom", "Seguros", "Motors", "Saúde", "Log", "Bebidas", "Tech", "Crédito",
    "Airways", "Bet", "Foods", "Mobile", "Pay", "Petro",
];

/// `Economia.DONO_A`: the first part of an owner's fund.
pub const OWNER_FIRST: [&str; 16] = [
    "Crescent Oasis", "Northwind", "Bluewater", "Iron Peak", "Desert Falcon", "Golden Dune", "Silverline",
    "Starlight", "Granite Bay", "Sapphire Coast", "Redwood Arc", "Polaris Bay", "Highcliff", "Emerald Gate",
    "Coral Tide", "Black Summit",
];

/// `Economia.DONO_B`: the second part of an owner's fund.
pub const OWNER_SECOND: [&str; 6] = [
    "Capital", "Sports Group", "Holdings", "Partners", "Ventures", "Investimentos",
];

/// `Patrocinio.MARCAS`: the master sponsors.
pub const MASTER_BRANDS: [&str; 12] = [
    "Vortex Bank", "Nexo Energia", "Fly Brasil", "Mercado Total", "Pixel Telecom", "Rota Seguros",
    "Solaris Motors", "Prisma Saúde", "Atlas Log", "Onda Bebidas", "Kairo Tech", "Fortuna Crédito",
];

/// `Cotas.RAMOS`: the line of business that ends a slot sponsor's brand.
pub const SLOT_LINES: [&str; 16] = [
    "Bank", "Energia", "Telecom", "Seguros", "Motors", "Saúde", "Log", "Bebidas", "Tech", "Crédito",
    "Airways", "Foods", "Mobile", "Pay", "Sports", "Arena",
];

/// `Comissao.NOMES`
pub const STAFF_FIRST_NAMES: [&str; 30] = [
    "Paulo", "Marcos", "Rafael", "Diego", "Thiago", "André", "Lucas", "Bruno", "Fernando", "Ricardo",
    "Javier", "Pablo", "Luis", "Mathieu", "Julien", "Stéphane", "Marco", "Luca", "Stefan", "Jonas", "Tomás",
    "Nuno", "Rui", "Sérgio", "Daniel", "Gabriel", "Hugo", "Pierre", "Andrea", "Carlos",
];

/// `Comissao.SOBRENOMES`
pub const STAFF_SURNAMES: [&str; 30] = [
    "Almeida", "Rocha", "Fonseca", "Carvalho", "Teixeira", "Moreira", "Pires", "Duarte", "Navarro", "Ortega",
    "Ibáñez", "Lambert", "Moreau", "Girard", "Rossi", "Bianchi", "Ferrara", "Keller", "Brandt", "Hofmann",
    "Sousa", "Matos", "Vidal", "Peralta", "Dubois", "Fontaine", "Costa", "Barros", "Lemos", "Guerra",
];

/// `Patrimonio.propostasNaming`'s brands for a stadium's naming rights.
pub const NAMING_BRANDS: [&str; 12] = [
    "Vortex Bank", "Solaris Motors", "Kairo Tech", "Nexo Energia", "Atlas Airways", "Prisma Seguros",
    "Titan Telecom", "Aurora Invest", "Zenith Pay", "Onda Global", "Lumen Digital", "Boreal Air",
];

/// `Economia.novaMarca()`, `Economia.novoNomeDeDono()`: two words.
pub fn two_words(first: &str, second: &str) -> String {
    format!("{first} {second}")
}

/// `MARCA_A[a] + " " + MARCA_B[b]`: a sponsor's brand.
pub fn brand((a, b): (u16, u16)) -> String {
    two_words(BRAND_FIRST[a as usize], BRAND_SECOND[b as usize])
}

/// `DONO_A[a] + " " + DONO_B[b]`: an owner's fund.
pub fn owner((a, b): (u16, u16)) -> String {
    two_words(OWNER_FIRST[a as usize], OWNER_SECOND[b as usize])
}

/// `NOMES[a] + " " + SOBRENOMES[b]`: a professional of the coaching staff.
pub fn staff((a, b): (u16, u16)) -> String {
    two_words(STAFF_FIRST_NAMES[a as usize], STAFF_SURNAMES[b as usize])
}

/// `MARCA_A[a] + " " + RAMOS[r]`: a slot sponsor's brand (`Cotas.resultado`).
pub fn slot_brand((a, r): (u16, u16)) -> String {
    two_words(BRAND_FIRST[a as usize], SLOT_LINES[r as usize])
}

/// The game's generic stadium name, which the mod's renames look for (`startsWith`).
pub const MUNICIPAL_STADIUM: &str = "Estádio Municipal";

/// `Patrimonio.nomeBaseEstadio`'s fallback when a club has no stadium name.
pub fn base_stadium_name() -> String {
    MUNICIPAL_STADIUM.to_owned()
}

/// `Estadios.nome(Lenda)`: a stadium named after a legend.
pub fn legend_stadium(legend: &str) -> String {
    format!("Estádio {legend}")
}

/// `Patrimonio.gerarNomeArenaClube(ah)`: a club's new arena, named the way of its country.
pub fn club_arena(country: i32, club_id: i32, club: &str) -> String {
    match country {
        // France
        30 => format!("{}{club}", if club_id % 2 == 0 { "Stade " } else { "Parc " }),
        // England, USA
        27 | 18 => format!("{club}{}", if club_id % 2 == 0 { " Stadium" } else { " Park" }),
        // Germany
        10 => format!("{club}-Arena"),
        // Italy
        35 => format!("Stadio {club}"),
        _ => format!("Arena {club}"),
    }
}

/// `Patrimonio.comporNomePatrocinado(String, String)`: the stadium's name with its naming-rights
/// brand (its first word).
pub fn sponsored_stadium(base: Option<&str>, brand: &str) -> Option<String> {
    if brand.is_empty() {
        return base.map(str::to_owned);
    }
    let short = brand.split(' ').next().unwrap_or("");
    match base {
        None => Some(format!("{short} Arena")),
        Some(b) if b.starts_with(MUNICIPAL_STADIUM) => Some(format!("{short} Arena")),
        Some(b) if b.to_lowercase().contains(&short.to_lowercase()) => Some(b.to_owned()),
        Some(b) => Some(format!("{short} {b}")),
    }
}
