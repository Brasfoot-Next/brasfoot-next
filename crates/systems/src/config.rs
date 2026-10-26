//! Port of `bfmod.Config`: the mod's switches, read from `bfmod.config` in the game folder
//! (`KEY=VALUE` lines, `#` comments).

use std::collections::BTreeMap;

/// `Config.Opcao`: every switch is on unless the file says `0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Switch {
    News,
    AwardEffects,
    Offers,
    UserWindow,
    AiLoans,
    LoanRequests,
    Youth,
    Sponsorship,
    Investor,
    Fans,
    ModernLook,
    Tactics,
    AutoTactics,
    Staff,
    YouthLeague,
    Assets,
    Stadiums,
    Credit,
    Satellites,
}

impl Switch {
    /// The key in `bfmod.config` (`Opcao.name()`).
    pub fn key(self) -> &'static str {
        match self {
            Switch::News => "JORNAL",
            Switch::AwardEffects => "EFEITOS_PREMIOS",
            Switch::Offers => "PROPOSTAS",
            Switch::UserWindow => "JANELA_USUARIO",
            Switch::AiLoans => "EMPRESTIMOS_IA",
            Switch::LoanRequests => "PEDIDOS_EMPRESTIMO",
            Switch::Youth => "BASE",
            Switch::Sponsorship => "PATROCINIO",
            Switch::Investor => "INVESTIDOR",
            Switch::Fans => "TORCIDA",
            Switch::ModernLook => "VISUAL_MODERNO",
            Switch::Tactics => "TATICAS",
            Switch::AutoTactics => "TATICA_AUTO",
            Switch::Staff => "COMISSAO",
            Switch::YouthLeague => "CAMPEONATO_BASE",
            Switch::Assets => "PATRIMONIO",
            Switch::Stadiums => "ESTADIOS",
            Switch::Credit => "CREDITO",
            Switch::Satellites => "SATELITES",
        }
    }
}

/// `Config.Mercado`: how busy the mod's transfer windows are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarketIntensity {
    Off,
    Contained,
    #[default]
    Realistic,
    Busy,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModConfig {
    values: BTreeMap<String, String>,
}

impl ModConfig {
    /// `Config.carregar()` on the text of `bfmod.config`.
    pub fn parse(text: &str) -> Self {
        let mut values = BTreeMap::new();
        for line in text.lines() {
            if let Some(i) = line.find('=')
                && i > 0
                && !line.starts_with('#')
            {
                values.insert(line[..i].trim().to_owned(), line[i + 1..].trim().to_owned());
            }
        }
        ModConfig { values }
    }

    /// `Config.ligado(Opcao)`
    pub fn on(&self, switch: Switch) -> bool {
        self.values.get(switch.key()).is_none_or(|v| v == "1")
    }

    /// `Config.mercado()`
    pub fn market(&self) -> MarketIntensity {
        match self.values.get("MERCADO").map(String::as_str) {
            Some("DESLIGADO") => MarketIntensity::Off,
            Some("CONTIDO") => MarketIntensity::Contained,
            Some("AGITADO") => MarketIntensity::Busy,
            _ => MarketIntensity::Realistic,
        }
    }
}
