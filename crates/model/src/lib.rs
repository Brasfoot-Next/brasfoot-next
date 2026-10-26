//! Game data types: a readable mirror of the classes the original game saves, plus the
//! randomness replica every crate shares.

pub mod fdlibm;
pub mod brands;
pub mod host_cities;
pub mod canonical;
pub mod clock;
mod canonical_generated;
pub mod countries;
pub mod ids;
pub mod messages;
pub mod mods;
pub mod options;
pub mod names;
pub mod player;
pub mod runtime;
pub mod rng;
pub mod saved;
pub mod templates;
pub mod java_hash;
pub mod labels;
pub mod java_sort;
mod save_numbers;
pub mod stage;
pub mod world;
