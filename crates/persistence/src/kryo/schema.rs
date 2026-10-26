//! The Kryo wire schema of the game's classes, generated from the game jar by
//! `tools/exporter` (`export.sh schema`, `exporter.KryoSchema`) into `data/kryo-schema.json`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Schema {
    pub format: String,
    pub classes: HashMap<String, ClassSchema>,
}

#[derive(Debug, Deserialize)]
pub struct ClassSchema {
    /// `Kryo.isFinal`: for arrays, whether the innermost element class is final.
    #[serde(rename = "final", default)]
    pub is_final: bool,
    /// Which Kryo serializer a default `new Kryo()` uses (`field`, `collection`, `int-array`...).
    pub serializer: String,
    pub component: Option<String>,
    #[serde(default)]
    pub constants: Vec<String>,
    #[serde(default)]
    pub fields: Vec<FieldSchema>,
    /// Transient fields: not in the stream, left at whatever the no-arg constructor sets.
    #[serde(default)]
    pub transient: Vec<FieldSchema>,
}

#[derive(Debug, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub declaring: String,
    /// A primitive type name, or `object`.
    #[serde(rename = "type")]
    pub kind: String,
    pub declared: Option<String>,
    /// Set when the declared type is final: the value is written without its class.
    #[serde(rename = "valueClass")]
    pub value_class: Option<String>,
    #[serde(rename = "canBeNull", default)]
    pub can_be_null: bool,
}

const SCHEMA_JSON: &str = include_str!("../../data/kryo-schema.json");

pub fn game_schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_JSON).expect("data/kryo-schema.json is valid"))
}
