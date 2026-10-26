//! Methods of the stages of a competition (`best.L`, `f.s`, `f.z`).

use crate::saved::{Stage, StageKind};

impl Stage {
    /// `L.b()`: the competition kind of the stage; a knockout (`f.z.b()`) keeps it in `tR`.
    pub fn kind_code(&self) -> i32 {
        match self.class {
            StageKind::League => self.kind,
            StageKind::Knockout => self.competition_kind,
        }
    }
}
