//! Engine data pack: the rules and tuning tables the simulation runs on.
//! World content (nations, clubs, people) is imported separately; this crate
//! holds only what defines *how* football works in Pathway (P7).

mod pack;
mod tuning;

pub use pack::*;
pub use tuning::*;

#[cfg(test)]
mod tests {
    use pw_core::pos::Pos;

    use super::*;

    #[test]
    fn builtin_pack_loads_and_weights_normalise() {
        let pack = DataPack::builtin();
        assert!(pack.formations.len() >= 8);
        for p in Pos::ALL {
            let s: f32 = pack.weights.row(p).iter().sum();
            assert!((s - 1.0).abs() < 1e-3, "{p:?} sums to {s}");
        }
        assert!(pack.curves.factor(pw_core::attr::CurveGroup::Speed, 17.0) > 0.5);
        assert!(pack.curves.factor(pw_core::attr::CurveGroup::Speed, 33.0) < 0.0);
        assert!(pack.calendar("autumn_spring").crosses_year());
    }
}
