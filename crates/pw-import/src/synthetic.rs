//! Synthetic worlds for tests and benchmarks only — never game content.
//! Real worlds come from the FM export through `load_dir`.

use pw_core::rng::{Rng, stream};
use pw_core::{Date, TeamId};
use pw_data::DataPack;
use pw_world::contract::ContractKind;
use pw_world::nation::Confed;
use pw_world::{CompKind, Contract, Format, TeamKind, World};

use crate::builder::{self, ClubSpec};

#[derive(Clone, Copy, Debug)]
pub struct Scale {
    pub nations: usize,
    pub divisions: usize,
    pub clubs_per_division: usize,
    pub youth_teams: bool,
}

impl Scale {
    pub const TINY: Scale = Scale { nations: 1, divisions: 1, clubs_per_division: 8, youth_teams: false };
    pub const SMALL: Scale = Scale { nations: 2, divisions: 2, clubs_per_division: 16, youth_teams: true };
    /// Roughly 300k players: 40 nations × 4 divisions × 22 clubs × (26 + 20 + 20).
    pub const HUGE: Scale = Scale { nations: 40, divisions: 4, clubs_per_division: 22, youth_teams: true };
}

const SYL: [&str; 24] = ["ka", "lo", "mi", "ra", "ten", "vo", "si", "dan", "el", "bu", "ro", "na", "ti", "gor", "sa", "le", "mar", "in", "os", "ve", "za", "ul", "ber", "to"];

fn word(rng: &mut Rng, parts: usize) -> String {
    let mut s: String = (0..parts).map(|_| *rng.pick(&SYL)).collect();
    if let Some(f) = s.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    s
}

pub fn build(pack: DataPack, seed: u64, scale: Scale) -> World {
    let start = Date::from_ymd(2026, 7, 1);
    let mut w = World::new(pack, seed, start);
    let mut rng = Rng::keyed(&[seed, stream::WORLDGEN]);
    let extra: &[TeamKind] = if scale.youth_teams { &[TeamKind::U21, TeamKind::U18] } else { &[] };

    for n in 0..scale.nations {
        let rep = (9000 - n * 180).max(1500) as u16;
        let code = format!("N{n:02}");
        let nation = builder::add_nation(&mut w, &code, &format!("Nation {n}"), if n % 5 == 4 { Confed::Conmebol } else { Confed::Uefa }, rep, "autumn_spring", 1.0 - n as f32 * 0.015, 12);
        for d in 0..scale.divisions {
            let tier = (d + 1) as u8;
            let lrep = rep / u16::from(tier);
            let league = builder::add_comp(
                &mut w,
                &format!("{code} Division {tier}"),
                &format!("{code} D{tier}"),
                nation,
                None,
                CompKind::League,
                tier,
                TeamKind::First,
                scale.clubs_per_division as u16,
                if d > 0 { 3 } else { 0 },
                if d + 1 < scale.divisions { 3 } else { 0 },
                lrep,
                Format::League { rounds: 2 },
                i64::from(lrep) * 8_000,
            );
            for k in 0..scale.clubs_per_division {
                let crep = (f32::from(lrep) * (1.25 - 0.5 * k as f32 / scale.clubs_per_division as f32)) as u16;
                let name = format!("{} {}", word(&mut rng, 2), ["United", "City", "Athletic", "FC", "Rovers"][k % 5]);
                let club = builder::add_club(
                    &mut w,
                    ClubSpec {
                        name: &name,
                        short: "",
                        nation,
                        city: "",
                        league,
                        reputation: crep,
                        balance: i64::from(crep) * 3_000,
                        stadium: "",
                        capacity: u32::from(crep) * 5 + 2_000,
                        facilities: builder::default_facilities(crep),
                        colors: [rng.next_u32() & 0xffffff, 0xffffff],
                        founded: 1900,
                        extra_teams: extra,
                    },
                );
                let teams: Vec<TeamId> = w.clubs[club].teams.to_vec();
                for t in teams {
                    let (count, ages) = match w.teams[t].kind {
                        TeamKind::First => (26, (19, 34)),
                        TeamKind::U18 => (20, (15, 17)),
                        _ => (20, (17, 20)),
                    };
                    let target = 55.0 + 110.0 * (f32::from(crep) / 10_000.0).powf(0.8);
                    for _ in 0..count {
                        let age = rng.range_i32(ages.0, ages.1);
                        let dob = start.add_days(-(age * 365 + rng.range_i32(0, 364)));
                        let pos = pw_sim::generate::random_position(&mut rng);
                        let youth_scale = pw_sim::generate::ca_share_at(age as f32);
                        let pa = rng.normal_ms(target + 10.0, 14.0).clamp(40.0, 200.0);
                        let ca = (pa * youth_scale * rng.normal_ms(1.0, 0.07)).clamp(20.0, pa);
                        let contract = Contract {
                            club,
                            kind: if age < 17 { ContractKind::Youth } else { ContractKind::Professional },
                            wage: 0,
                            start,
                            end: Date::from_ymd(2027 + rng.range_i32(0, 3), 6, 30),
                            yearly_rise: 3,
                            ..Default::default()
                        };
                        let np = pw_sim::people::NewPlayer { nation, dob, pos, ca, pa: pa as u8, club, team: t, contract };
                        let p = pw_sim::people::spawn_player(&mut w, np, &mut rng);
                        let first = word(&mut rng, 2);
                        let last = word(&mut rng, 3);
                        let (fi, li) = (w.names.intern(&first), w.names.intern(&last));
                        let person = w.players.cold[p].person;
                        w.people[person].first = fi;
                        w.people[person].last = li;
                    }
                }
            }
        }
        builder::add_comp(&mut w, &format!("{code} Cup"), "", nation, None, CompKind::Cup, 2, TeamKind::First, 0, 0, 0, rep / 2, Format::Knockout { legs: 1, final_legs: 1 }, i64::from(rep) * 1_000);
        if scale.youth_teams {
            builder::add_comp(&mut w, &format!("{code} U18 League"), "", nation, None, CompKind::League, 1, TeamKind::U18, 16, 0, 0, rep / 10, Format::League { rounds: 2 }, 0);
        }
    }
    if scale.nations >= 4 {
        builder::add_comp(&mut w, "Champions Cup", "CC", pw_core::NationId::NONE, Some(Confed::Uefa), CompKind::Continental, 1, TeamKind::First, 32, 0, 0, 9500, Format::Groups { groups: 8, size: 4, advance: 2, legs: 2, ko_legs: 2, final_legs: 1 }, 400_000_000);
    }
    builder::finalize(&mut w);
    builder::ensure_staff(&mut w);
    for p in w.players.ids() {
        let club = w.players.hot[p].club;
        if club.is_some() && w.players.cold[p].contract.wage == 0 {
            w.players.cold[p].contract.wage = pw_sim::market::wage_demand(&w, p, club);
        }
    }
    w
}
