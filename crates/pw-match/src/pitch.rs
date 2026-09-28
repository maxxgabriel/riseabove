//! Pitch geometry: zones, expected threat and base shot quality.
//! All coordinates are in the attacking frame of the side in possession:
//! x = 0 own goal line, x = 1 opponent goal line; y = 0 left touchline.

pub const ZONES_X: usize = 6;
pub const ZONES_Y: usize = 5;
pub const N_ZONES: usize = ZONES_X * ZONES_Y;

#[inline]
pub const fn zone_id(zx: usize, zy: usize) -> usize {
    zx * ZONES_Y + zy
}

#[inline]
pub const fn zone_xy(z: usize) -> (usize, usize) {
    (z / ZONES_Y, z % ZONES_Y)
}

/// The same zone seen from the other team's attacking frame.
#[inline]
pub const fn mirror(z: usize) -> usize {
    let (x, y) = zone_xy(z);
    zone_id(ZONES_X - 1 - x, ZONES_Y - 1 - y)
}

#[inline]
pub fn zone_center(z: usize) -> (f32, f32) {
    let (x, y) = zone_xy(z);
    ((x as f32 + 0.5) / ZONES_X as f32, (y as f32 + 0.5) / ZONES_Y as f32)
}

/// Distance in zone units (length counts slightly more than width).
#[inline]
pub fn zone_dist(a: usize, b: usize) -> f32 {
    let (ax, ay) = zone_xy(a);
    let (bx, by) = zone_xy(b);
    let dx = ax as f32 - bx as f32;
    let dy = ay as f32 - by as f32;
    (dx * dx + 0.8 * dy * dy).sqrt()
}

#[inline]
pub const fn in_box(z: usize) -> bool {
    let (x, y) = zone_xy(z);
    x == ZONES_X - 1 && y >= 1 && y <= 3
}

#[inline]
pub const fn is_wide(z: usize) -> bool {
    let (_, y) = zone_xy(z);
    y == 0 || y == ZONES_Y - 1
}

/// Expected threat: probability that possession in a zone ends in a goal.
const XT: [[f32; ZONES_Y]; ZONES_X] = [
    [0.004, 0.005, 0.006, 0.005, 0.004],
    [0.006, 0.008, 0.009, 0.008, 0.006],
    [0.010, 0.013, 0.015, 0.013, 0.010],
    [0.016, 0.022, 0.028, 0.022, 0.016],
    [0.024, 0.036, 0.048, 0.036, 0.024],
    [0.032, 0.070, 0.140, 0.070, 0.032],
];

#[inline]
pub fn xt(z: usize) -> f32 {
    let (x, y) = zone_xy(z);
    XT[x][y]
}

/// Base xG of an open-play footed shot from a zone (before context).
const XG: [[f32; ZONES_Y]; ZONES_X] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.002, 0.004, 0.006, 0.004, 0.002],
    [0.008, 0.016, 0.022, 0.016, 0.008],
    [0.014, 0.032, 0.045, 0.032, 0.014],
    [0.018, 0.085, 0.170, 0.085, 0.018],
];

#[inline]
pub fn base_xg(z: usize) -> f32 {
    let (x, y) = zone_xy(z);
    XG[x][y]
}

#[inline]
pub fn header_xg(z: usize) -> f32 {
    let (x, y) = zone_xy(z);
    match (x, y) {
        (5, 2) => 0.12,
        (5, 1) | (5, 3) => 0.07,
        (5, _) => 0.03,
        (4, 1..=3) => 0.025,
        _ => 0.01,
    }
}
