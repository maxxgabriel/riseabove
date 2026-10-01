//! Player generation: attributes shaped to a position and ability, positional
//! familiarity, personality. Used for regens (youth intake) and to fill gaps
//! in imported data.

use pw_core::math::interp;
use pw_core::rng::Rng;
use pw_core::{Attr, Attrs, Hidden, HiddenAttrs, N_ATTR, N_HIDDEN, N_POS, Pos, PosGroup};
use pw_data::PositionWeights;

/// Attributes whose CA at `pos` is close to `ca`. Key attributes for the
/// position sit above the player's general level, irrelevant ones below.
pub fn attrs_for(w: &PositionWeights, pos: Pos, ca: f32, rng: &mut Rng) -> Attrs {
    let row = w.row(pos);
    let max_w = row.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let mut noise = [0.0f32; N_ATTR];
    for n in &mut noise {
        *n = rng.normal() * 1.4;
    }
    let keeper = pos == Pos::GK;
    let shape = |level: f32, out: &mut Attrs| {
        for a in Attr::ALL {
            let i = a.idx();
            let v = if a.is_goalkeeping() && !keeper {
                1.0 + (noise[i].abs() * 1.5).min(4.0)
            } else {
                let emphasis = row[i] / max_w;
                1.0 + (level - 1.0) * (0.55 + 0.9 * emphasis) + noise[i]
            };
            out.set(a, v.clamp(1.0, 20.0));
        }
    };
    let ca_of = |a: &Attrs| pw_world::player::raw_ability(a, pos, w);
    let (mut lo, mut hi) = (1.0f32, 22.0f32);
    let mut out = Attrs::default();
    for _ in 0..18 {
        let mid = (lo + hi) * 0.5;
        shape(mid, &mut out);
        if ca_of(&out) < ca {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    shape((lo + hi) * 0.5, &mut out);
    out
}

/// Natural at `pos`, with familiarity elsewhere falling off with dissimilarity.
pub fn familiarity_for(pos: Pos, versatility: f32, rng: &mut Rng) -> [u8; N_POS] {
    let mut f = [1u8; N_POS];
    for p in Pos::ALL {
        let sim = pos.similarity(p);
        let v = if p == pos { 20.0 } else { 1.0 + sim * (10.0 + versatility * 0.4) + rng.normal() * 1.5 };
        f[p.idx()] = v.round().clamp(1.0, 20.0) as u8;
    }
    // A second natural position for a fair share of players.
    if rng.chance(0.3) && pos != Pos::GK {
        let alt = Pos::ALL.into_iter().filter(|&p| p != pos && p != Pos::GK).max_by(|&a, &b| pos.similarity(a).total_cmp(&pos.similarity(b)));
        if let Some(alt) = alt {
            f[alt.idx()] = f[alt.idx()].max(15 + rng.below(4) as u8);
        }
    }
    f
}

pub fn hidden_random(rng: &mut Rng) -> HiddenAttrs {
    let mut h = HiddenAttrs([10; N_HIDDEN]);
    for x in Hidden::ALL {
        let mean = match x {
            Hidden::InjuryProneness | Hidden::Controversy | Hidden::Dirtiness => 8.0,
            Hidden::Sportsmanship | Hidden::Professionalism => 11.0,
            _ => 10.5,
        };
        h.set(x, rng.normal_ms(mean, 3.4).round().clamp(1.0, 20.0) as u8);
    }
    h
}

/// Share of potential typically realised by a given age (youth → peak).
pub fn ca_share_at(age: f32) -> f32 {
    interp(
        &[
            (8.0, 0.12),
            (10.0, 0.18),
            (12.0, 0.24),
            (14.0, 0.3),
            (15.0, 0.35),
            (16.0, 0.42),
            (17.0, 0.5),
            (18.0, 0.58),
            (19.0, 0.66),
            (20.0, 0.73),
            (21.0, 0.8),
            (23.0, 0.89),
            (25.0, 0.95),
            (27.0, 1.0),
        ],
        age,
    )
}

/// The most of his potential a player of this (biological) age can have realised: what a typical player has (`ca_share_at`), plus
/// room for the precocious, room that widens with age until, by the early twenties, nothing but potential limits him. A ten-year-old
/// is a ten-year-old whatever his gift; without this a gifted child grew at a youth rate from eight and was as good as a senior
/// professional at twelve.
pub fn maturity_ceiling(age: f32) -> f32 {
    let headroom = interp(&[(12.0, 0.1), (16.0, 0.2), (19.0, 0.3), (22.0, 1.0)], age);
    (ca_share_at(age) + headroom).min(1.0)
}

/// FM-style potential: positive values are exact; negative encode a range
/// (-1..-10 → 10-point bands 50 wide; -15..-95 → 30-wide bands), sampled once.
pub fn resolve_pa(raw: i32, ca: u8, rng: &mut Rng) -> u8 {
    let pa = match raw {
        v if v > 0 => v as f32,
        v if v >= -10 => {
            let lo = (-v) as f32 * 10.0 + 50.0;
            rng.range_f32(lo, (lo + 50.0).min(200.0))
        }
        v => {
            let lo = (-v) as f32 + 75.0;
            rng.range_f32(lo.min(170.0), (lo + 30.0).min(200.0))
        }
    };
    (pa.round() as i32).clamp(i32::from(ca.max(1)), 200) as u8
}

/// Position distribution for generated squads and intakes.
pub fn random_position(rng: &mut Rng) -> Pos {
    const W: [(Pos, f32); N_POS] = [
        (Pos::GK, 0.09),
        (Pos::DR, 0.08),
        (Pos::DC, 0.15),
        (Pos::DL, 0.07),
        (Pos::WBR, 0.02),
        (Pos::WBL, 0.02),
        (Pos::DM, 0.07),
        (Pos::MR, 0.05),
        (Pos::MC, 0.13),
        (Pos::ML, 0.05),
        (Pos::AMR, 0.05),
        (Pos::AMC, 0.06),
        (Pos::AML, 0.05),
        (Pos::ST, 0.11),
    ];
    let weights: [f32; N_POS] = W.map(|(_, w)| w);
    W[rng.weighted(&weights)].0
}

pub fn height_for(pos: Pos, rng: &mut Rng) -> u8 {
    let mean = match pos.group() {
        PosGroup::Gk => 189.0,
        PosGroup::Def => {
            if pos == Pos::DC {
                187.0
            } else {
                178.0
            }
        }
        PosGroup::Mid => 178.0,
        PosGroup::Att => {
            if pos == Pos::ST {
                183.0
            } else {
                175.0
            }
        }
    };
    rng.normal_ms(mean, 5.5).round().clamp(158.0, 205.0) as u8
}

#[cfg(test)]
mod tests {
    use pw_data::DataPack;

    use super::*;

    #[test]
    fn attributes_hit_target_ability() {
        let pack = DataPack::builtin();
        let mut rng = Rng::new(3);
        for &(pos, ca) in &[(Pos::ST, 150.0), (Pos::DC, 90.0), (Pos::GK, 120.0), (Pos::MC, 60.0)] {
            let a = attrs_for(&pack.weights, pos, ca, &mut rng);
            let got = pw_world::player::raw_ability(&a, pos, &pack.weights);
            assert!((got - ca).abs() < 4.0, "{pos:?}: wanted {ca} got {got}");
        }
    }
}
