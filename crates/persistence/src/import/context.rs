//! What the generated converters read the Kryo graph through: typed field readers that fall back
//! to the field's default when a field is absent (subclasses folded into one struct), and the
//! maps from graph nodes and game ids to arena ids.

use std::collections::HashMap;

use brasfoot_model::ids::*;

use crate::kryo::{Graph, NodeData, NodeId, Value};

/// Arena kinds, in the order of [`brasfoot_model::world::World`]'s fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arena {
    Player,
    Club,
    Coach,
    Stadium,
    Match,
    Competition,
    Stage,
    Country,
    StateLeague,
}

impl Arena {
    /// The arena of a Java class, if its objects are shared.
    pub fn of_class(class: &str) -> Option<Arena> {
        Some(match class {
            "best.F" => Arena::Player,
            "best.ah" => Arena::Club,
            "best.al" => Arena::Coach,
            "best.v" => Arena::Stadium,
            "best.I" => Arena::Match,
            "best.Z" => Arena::Country,
            "best.w" => Arena::StateLeague,
            "f.s" | "f.z" => Arena::Stage,
            // f.G is a knockout tie, owned by its stage
            "f.G" => return None,
            c if c.starts_with("f.") => Arena::Competition,
            _ => return None,
        })
    }
}

pub struct Context<'g> {
    pub graph: &'g Graph,
    /// Arena index of every node that belongs to an arena.
    arena_index: Vec<u32>,
    /// Nodes of each arena, in arena order.
    pub members: HashMap<Arena, Vec<NodeId>>,
    club_by_game_id: HashMap<i32, ClubId>,
    coach_by_game_id: HashMap<i32, CoachId>,
    competition_by_game_id: HashMap<i32, CompetitionId>,
    /// `g.b.Ai` numbering: players of `c.a.SR.O()` (list 1) and `c.a.SR.Q()` (list 2) by `F.ei`.
    player_by_save_number: [HashMap<i32, PlayerId>; 2],
}

impl<'g> Context<'g> {
    pub fn new(graph: &'g Graph) -> Self {
        let mut arena_index = vec![u32::MAX; graph.nodes.len()];
        let mut members: HashMap<Arena, Vec<NodeId>> = HashMap::new();
        for (i, node) in graph.nodes.iter().enumerate() {
            if let Some(arena) = Arena::of_class(graph.class_name(node.class)) {
                let list = members.entry(arena).or_default();
                arena_index[i] = list.len() as u32;
                list.push(i as NodeId);
            }
        }
        let mut ctx = Context {
            graph,
            arena_index,
            members,
            club_by_game_id: HashMap::new(),
            coach_by_game_id: HashMap::new(),
            competition_by_game_id: HashMap::new(),
            player_by_save_number: [HashMap::new(), HashMap::new()],
        };
        // first object wins, like the game's linear searches (c.a.SR.x, c.a.SR.y)
        let by_game_id = |ctx: &Context, arena: Arena, field: &str| {
            let mut map = HashMap::new();
            for (i, &node) in ctx.nodes(arena).iter().enumerate() {
                map.entry(ctx.int(node, field)).or_insert(i as u32);
            }
            map
        };
        ctx.club_by_game_id = by_game_id(&ctx, Arena::Club, "mU").into_iter().map(|(k, v)| (k, ClubId(v))).collect();
        ctx.coach_by_game_id = by_game_id(&ctx, Arena::Coach, "nU").into_iter().map(|(k, v)| (k, CoachId(v))).collect();
        ctx.competition_by_game_id =
            by_game_id(&ctx, Arena::Competition, "ei").into_iter().map(|(k, v)| (k, CompetitionId(v))).collect();
        if let Some(&Value::Node(root)) = graph.roots.first() {
            for (list, field) in ["ag", "ah"].into_iter().enumerate() {
                let players: Vec<Value> = ctx.items(root, field, |_, v| v);
                for v in players {
                    if let Value::Node(node) = v {
                        let id = PlayerId(ctx.arena_index[node as usize]);
                        // ap.me() takes the first match in the list
                        ctx.player_by_save_number[list].entry(ctx.int(node, "ei")).or_insert(id);
                    }
                }
            }
        }
        ctx
    }

    pub fn nodes(&self, arena: Arena) -> &[NodeId] {
        self.members.get(&arena).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn class_name(&self, node: NodeId) -> &str {
        self.graph.class_name(self.graph.node(node).class)
    }

    fn field(&self, node: NodeId, name: &str) -> Value {
        self.graph.field(node, name).unwrap_or(Value::Null)
    }

    pub fn node_of(&self, value: Value) -> NodeId {
        match value {
            Value::Node(id) => id,
            other => panic!("expected an object, found {other:?}"),
        }
    }

    // ---- primitives ----

    pub fn int(&self, node: NodeId, name: &str) -> i32 {
        self.value_int(self.field(node, name))
    }

    pub fn long(&self, node: NodeId, name: &str) -> i64 {
        match self.field(node, name) {
            Value::Long(n) => n,
            Value::Int(n) => n as i64,
            _ => 0,
        }
    }

    pub fn double(&self, node: NodeId, name: &str) -> f64 {
        match self.field(node, name) {
            Value::Double(d) => d,
            Value::Float(f) => f as f64,
            _ => 0.0,
        }
    }


    pub fn boolean(&self, node: NodeId, name: &str) -> bool {
        matches!(self.field(node, name), Value::Bool(true))
    }

    /// A `java.lang.Boolean`; null reads as false.
    pub fn boxed_boolean(&self, node: NodeId, name: &str) -> bool {
        self.boolean(node, name)
    }

    pub fn boxed_int(&self, node: NodeId, name: &str) -> Option<i32> {
        match self.field(node, name) {
            Value::Int(n) => Some(n),
            _ => None,
        }
    }

    pub fn value_int(&self, value: Value) -> i32 {
        match value {
            Value::Int(n) => n,
            Value::Short(n) => n as i32,
            Value::Byte(n) => n as i32,
            Value::Char(n) => n as i32,
            _ => 0,
        }
    }

    // ---- strings, dates, arrays ----

    /// A string field; null reads as "".
    pub fn string(&self, node: NodeId, name: &str) -> String {
        self.optional_string(node, name).unwrap_or_default()
    }

    pub fn optional_string(&self, node: NodeId, name: &str) -> Option<String> {
        match self.field(node, name) {
            Value::Str(id) => Some(self.graph.string(id).to_string()),
            _ => None,
        }
    }

    pub fn value_string(&self, value: Value) -> String {
        match value {
            Value::Str(id) => self.graph.string(id).to_string(),
            _ => String::new(),
        }
    }

    /// A `java.util.Calendar`, as epoch milliseconds (0 when null).
    pub fn calendar(&self, node: NodeId, name: &str) -> i64 {
        match self.field(node, name) {
            Value::Node(id) => match &self.graph.node(id).data {
                NodeData::Calendar(c) => c.millis,
                _ => 0,
            },
            _ => 0,
        }
    }

    fn array_data(&self, node: NodeId, name: &str) -> Option<&'g NodeData> {
        match self.field(node, name) {
            Value::Node(id) => Some(&self.graph.node(id).data),
            _ => None,
        }
    }

    pub fn int_array(&self, node: NodeId, name: &str) -> Vec<i32> {
        match self.array_data(node, name) {
            Some(NodeData::Ints(v)) => v.clone(),
            _ => Vec::new(),
        }
    }

    pub fn int_matrix(&self, node: NodeId, name: &str) -> Vec<Vec<i32>> {
        match self.array_data(node, name) {
            Some(NodeData::Items(rows)) => rows
                .iter()
                .map(|row| match row {
                    Value::Node(id) => match &self.graph.node(*id).data {
                        NodeData::Ints(v) => v.clone(),
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn boolean_array(&self, node: NodeId, name: &str) -> Vec<bool> {
        match self.array_data(node, name) {
            Some(NodeData::Booleans(v)) => v.clone(),
            _ => Vec::new(),
        }
    }

    pub fn double_array(&self, node: NodeId, name: &str) -> Vec<f64> {
        match self.array_data(node, name) {
            Some(NodeData::Doubles(v)) => v.clone(),
            _ => Vec::new(),
        }
    }


    // ---- objects ----

    /// The elements of a list or object-array field; nothing when the field is null.
    pub fn items<T>(&self, node: NodeId, name: &str, mut convert: impl FnMut(&Self, Value) -> T) -> Vec<T> {
        match self.array_data(node, name) {
            Some(NodeData::Items(items)) => items.iter().map(|&v| convert(self, v)).collect(),
            _ => Vec::new(),
        }
    }

    /// `convert` applied to a value unless it is null.
    pub fn nullable<T>(&self, value: Value, convert: impl FnOnce(&Self, Value) -> T) -> Option<T> {
        if value == Value::Null { None } else { Some(convert(self, value)) }
    }

    /// An object field: `convert` of its value, or None when null.
    pub fn reference<T>(&self, node: NodeId, name: &str, convert: impl FnOnce(&Self, Value) -> T) -> Option<T> {
        let value = self.field(node, name);
        self.nullable(value, convert)
    }

    /// An embedded object field.
    pub fn embedded<T>(&self, node: NodeId, name: &str, convert: impl FnOnce(&Self, Value) -> T) -> Option<T> {
        self.reference(node, name, convert)
    }

    fn arena_id(&self, value: Value, arena: Arena) -> u32 {
        let node = self.node_of(value);
        let class = self.class_name(node);
        assert_eq!(Arena::of_class(class), Some(arena), "{class} is not in the {arena:?} arena");
        self.arena_index[node as usize]
    }

    pub fn id_of_player_id(&self, v: Value) -> PlayerId {
        PlayerId(self.arena_id(v, Arena::Player))
    }

    pub fn id_of_club_id(&self, v: Value) -> ClubId {
        ClubId(self.arena_id(v, Arena::Club))
    }

    pub fn id_of_coach_id(&self, v: Value) -> CoachId {
        CoachId(self.arena_id(v, Arena::Coach))
    }

    pub fn id_of_stadium_id(&self, v: Value) -> StadiumId {
        StadiumId(self.arena_id(v, Arena::Stadium))
    }

    pub fn id_of_match_id(&self, v: Value) -> MatchId {
        MatchId(self.arena_id(v, Arena::Match))
    }

    pub fn id_of_competition_id(&self, v: Value) -> CompetitionId {
        CompetitionId(self.arena_id(v, Arena::Competition))
    }

    pub fn id_of_stage_id(&self, v: Value) -> StageId {
        StageId(self.arena_id(v, Arena::Stage))
    }

    pub fn id_of_country_id(&self, v: Value) -> CountryId {
        CountryId(self.arena_id(v, Arena::Country))
    }

    pub fn id_of_state_league_id(&self, v: Value) -> StateLeagueId {
        StateLeagueId(self.arena_id(v, Arena::StateLeague))
    }

    // ---- ids stored as ints ----

    /// `F.bW`: the club with that game id (`c.a.SR.x`), None for -1 or an unknown id.
    pub fn int_ref_club_id(&self, node: NodeId, name: &str) -> Option<ClubId> {
        let id = self.int(node, name);
        if id < 0 { None } else { self.club_by_game_id.get(&id).copied() }
    }

    /// `ah.na`: the coach with that game id (`c.a.SR.y`).
    pub fn int_ref_coach_id(&self, node: NodeId, name: &str) -> Option<CoachId> {
        let id = self.int(node, name);
        if id < 0 { None } else { self.coach_by_game_id.get(&id).copied() }
    }

    /// `ap.ei`/`ap.ej`: the transferred player, as `ap.me()` finds him after a load.
    pub fn transfer_player(&self, node: NodeId) -> Option<PlayerId> {
        let number = self.int(node, "ei");
        let list = match self.int(node, "ej") {
            1 => 0,
            2 => 1,
            _ => return None,
        };
        if number < 0 { None } else { self.player_by_save_number[list].get(&number).copied() }
    }
}
