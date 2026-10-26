//! Data packs: clubs, squads, leagues and pictures in a plain, versioned folder format that
//! replaces the original's team files and configurations, stacked as a base plus patches.
//! The game reads them at a new game; `brasfoot-data` converts, validates and packs them.

pub mod codes;
pub mod database;
pub mod format;
pub mod issues;
pub mod legacy;
pub mod pack;
pub mod read;
pub mod tree;
pub mod trophies;
