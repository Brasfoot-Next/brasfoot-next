//! Constant tables of `best.aq` that the match engine and the line-ups use.

/// `aq.sE`: per line-up slot (1 to 25; 0 unused), the position it takes, the side (0 right,
/// 1 left, -1 any) and the style (-1 any).
pub const SLOT_ROLE: [[i32; 3]; 26] = [
    [0, -1, -1], [0, -1, -1], [1, 1, -1], [2, 1, -1], [2, -1, -1], [2, 0, -1], [2, 1, -1], [2, -1, -1], [2, 0, -1],
    [1, 0, -1], [1, 1, 1], [3, 1, 0], [3, -1, 0], [3, 0, 0], [3, 1, 1], [3, -1, 1], [3, 0, 1], [1, 1, 0], [4, 1, 2],
    [4, 1, -1], [4, -1, -1], [4, 0, -1], [4, 1, -1], [4, -1, -1], [4, 0, -1], [4, 0, 2],
];

/// `aq.sF`: per slot, the pitch sectors (1 to 12) it covers, for the strength of `ah.a(I, ah, int)`.
pub const SLOT_SECTORS: [[i32; 3]; 26] = [
    [-1, -1, -1], [1, -1, -1], [3, 6, -1], [2, 3, -1], [2, -1, -1], [2, 1, -1], [2, 3, -1], [2, -1, -1], [2, 1, -1],
    [1, 4, -1], [6, 9, -1], [5, 6, -1], [5, -1, -1], [5, 4, -1], [8, 9, -1], [8, -1, -1], [8, 7, -1], [4, 7, -1],
    [12, -1, -1], [11, 12, -1], [11, -1, -1], [11, 10, -1], [11, 12, -1], [11, -1, -1], [11, 10, -1], [10, -1, -1],
];

/// `aq.sH`: per position, the positions tried in order when filling a slot of that position.
pub const POSITION_FALLBACK: [[i32; 5]; 5] = [[0, 2, 1, 3, 4], [1, 3, 2, 4, 0], [2, 1, 3, 4, 0], [3, 1, 2, 4, 0], [4, 3, 1, 2, 0]];

/// `aq.sI`: the slots whose kind the AI bench is picked for, in order.
pub const BENCH_SLOTS: [i32; 11] = [1, 1, 2, 4, 4, 12, 15, 15, 20, 20, 23];

/// `aq.sJ`: formations, the 11 slots of each.
pub const FORMATIONS: [[i32; 11]; 13] = [
    [1, 20, 11, 13, 14, 16, 2, 9, 6, 4, 8],
    [1, 20, 11, 13, 14, 16, 2, 9, 6, 4, 8],
    [1, 22, 24, 12, 14, 16, 2, 9, 6, 4, 8],
    [1, 23, 11, 13, 15, 2, 9, 6, 8, 10, 17],
    [1, 22, 24, 11, 13, 14, 16, 2, 9, 3, 5],
    [1, 19, 21, 11, 12, 13, 15, 2, 9, 6, 8],
    [1, 22, 24, 12, 14, 15, 16, 2, 9, 6, 8],
    [1, 22, 23, 24, 12, 14, 16, 2, 9, 6, 8],
    [1, 19, 20, 21, 11, 13, 15, 2, 9, 6, 8],
    [1, 22, 24, 11, 13, 15, 4, 6, 8, 10, 17],
    [1, 18, 25, 23, 11, 13, 4, 6, 8, 10, 17],
    [1, 23, 14, 16, 15, 13, 11, 2, 9, 6, 8],
    [1, 20, 10, 17, 15, 13, 11, 2, 9, 6, 8],
];

/// `aq.sS`: slot ranges of the players an incident falls on (cards, injuries).
pub const INCIDENT_SLOTS: [[i32; 2]; 7] = [[10, 13], [14, 17], [3, 8], [2, 3], [8, 9], [19, 24], [1, 1]];

/// `aq.A(int, int)`: a share in whole percent, rounded like `Math.round(float)`.
pub fn percent(part: i32, total: i32) -> i32 {
    brasfoot_model::rng::java_round_f32(part as f32 / total as f32 * 100.0)
}
