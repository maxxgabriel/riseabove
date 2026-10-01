//! Academy pathways (Slice 3): what each academy's players became, and how a
//! child and their family choose between academies that want them.

use pw_core::{ClubId, PersonId, PlayerId, PosGroup};
use pw_world::ruling::{Outcome, Ruling, RulingKind, Stance, StanceRole};
use pw_world::{PlayerStatus, World};
use smallvec::SmallVec;

use crate::consider;

fn line(g: PosGroup) -> usize {
    match g {
        PosGroup::Gk => 0,
        PosGroup::Def => 1,
        PosGroup::Mid => 2,
        PosGroup::Att => 3,
    }
}

/// Yearly: every player turning 22 who came through a club counts once toward its record;
/// older years fade, so a club's name follows what it has done lately.
pub fn yearly(w: &mut World) {
    for rec in w.ext.academy.pathway.values_mut() {
        rec.total *= 0.85;
        rec.made *= 0.85;
        for x in &mut rec.by_line {
            *x *= 0.85;
        }
    }
    let ids: Vec<PlayerId> = w.players.ids().collect();
    for p in ids {
        let c = &w.players.cold[p];
        if c.youth_club.is_none() || w.players.hot[p].status == PlayerStatus::Retired || w.age(p) != 22 {
            continue;
        }
        let made = c.senior_apps >= 15;
        let l = line(c.best_pos.group());
        let rec = w.ext.academy.pathway.entry(c.youth_club).or_default();
        rec.total += 1.0;
        if made {
            rec.made += 1.0;
            rec.by_line[l] += 1.0;
        }
    }
    review_spending(w);
}

/// Clubs learn whether the academy pays: a board that sees graduates reaching the first team spends more,
/// one that sees few spends less. It reads the record, never a target.
pub fn review_spending(w: &mut World) {
    let mut clubs: Vec<ClubId> = w.ext.academy.pathway.keys().copied().collect();
    clubs.sort();
    for club in clubs {
        let rec = w.ext.academy.pathway[&club];
        if rec.total < 6.0 {
            continue;
        }
        let Some(g) = w.governance.get_mut(&club) else { continue };
        let rate = rec.made / rec.total;
        if rate > 0.30 {
            g.policy.youth_investment = (g.policy.youth_investment + 3).min(80);
        } else if rate < 0.12 {
            g.policy.youth_investment = g.policy.youth_investment.saturating_sub(4).max(5);
        }
    }
}

/// What a family makes of one academy, for one child: standing, record of producing first-team players,
/// nearness, and how they got on with the head of youth.
fn appeal(w: &World, club: ClubId, p: PlayerId, head: Option<PersonId>) -> f32 {
    let who = w.players.cold[p].person;
    let c = &w.clubs[club];
    let rep = f32::from(c.reputation) / 10_000.0;
    let record = w.ext.academy.pathway.get(&club).copied().unwrap_or_default().rate();
    let home = w.lives.get(who).map_or(w.people[who].nation, |l| if l.home.is_some() { l.home } else { w.people[who].nation });
    let near = if c.nation == home { 0.15 } else { -0.15 - 0.2 * consider::household_move_cost(w, who, c.nation).min(1.0) };
    let rapport = head.map_or(0.0, |h| (consider::trust(w, who, h) - 0.5) * 0.3);
    // Ambitious families weigh the badge, careful ones the road to the first team.
    let ambition = consider::hid(w, who, pw_core::Hidden::Ambition) / 20.0;
    rep * (0.2 + 0.4 * ambition) + record * (0.9 - 0.4 * ambition) + near + rapport
}

/// Several academies want the same child: the family picks. Returns the chosen club and, when more
/// than one wanted them, records the choice as a ruling (why Club B and not Club A).
pub fn choose(w: &mut World, p: PlayerId, clubs: &[ClubId]) -> ClubId {
    if clubs.len() == 1 {
        return clubs[0];
    }
    let today = w.date;
    let who = w.players.cold[p].person;
    let heads: Vec<Option<PersonId>> = clubs.iter().map(|&c| w.clubs[c].staff.iter().copied().find(|&s| w.staff[s].role == pw_world::StaffRole::HeadOfYouth).map(|s| w.staff[s].person)).collect();
    let scores: Vec<f32> = clubs
        .iter()
        .zip(&heads)
        .map(|(&c, &h)| appeal(w, c, p, h) + (w.roll(pw_core::rng::stream::FAMILY, &[u64::from(p.0), u64::from(c.0), (today.0 / 7) as u64, 0xacad]) - 0.5) * 0.1)
        .collect();
    let best = (0..clubs.len()).max_by(|&a, &b| scores[a].total_cmp(&scores[b]).then(clubs[b].cmp(&clubs[a]))).unwrap_or(0);
    let mut stances: SmallVec<[Stance; 4]> = SmallVec::new();
    for (i, &h) in heads.iter().enumerate().take(4) {
        if let Some(h) = h {
            stances.push(Stance { who: h, role: StanceRole::Recruiter, believed_pct: 255, backing: (scores[i] * 100.0).clamp(-100.0, 100.0) as i8, authority: i == best });
        }
    }
    let id = w.ext.decisions.add(Ruling {
        id: 0,
        kind: RulingKind::AcademyChoice,
        date: today,
        club: clubs[best],
        subject: p,
        about: PersonId::NONE,
        liability: 0,
        decider: who,
        stances,
        true_pct: 0,
        want: 0,
        outcome: Outcome::Enacted,
        resolved: Some(today),
        event: pw_core::EventId::NONE,
    });
    let ev = w.events.push(today, pw_world::event::Visibility::Person(who), pw_world::event::EventKind::Ruling { ruling: id });
    if let Some(r) = w.ext.decisions.get_mut(id) {
        r.event = ev;
    }
    clubs[best]
}
