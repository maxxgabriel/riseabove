//! Rule profiles (02 §3, P7). Pure data: which squad, labour, loan,
//! discipline and eligibility rules a nation's football runs under.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RuleProfile {
    pub key: String,
    /// Free-movement bloc this profile's nations belong to ("" = none).
    pub bloc: String,
    pub squad_max: u8,
    pub homegrown_min: u8,
    pub homegrown_age_from: u8,
    pub homegrown_age_to: u8,
    pub homegrown_years: u8,
    /// Players at or under this age don't count toward the squad limit.
    pub u21_exempt_age: u8,
    pub foreign_squad_max: u8,
    pub foreign_on_pitch_max: u8,
    pub min_pro_age: u8,
    pub max_contract_years: u8,
    pub max_contract_years_minor: u8,
    pub minor_age: u8,
    pub minors_abroad_min_age: u8,
    pub bloc_minors_min_age: u8,
    pub work_permit: bool,
    pub work_permit_min_points: u8,
    pub max_loans_in: u8,
    pub max_loans_out: u8,
    pub loans_domestic_only: bool,
    pub red_ban_straight: u8,
    pub red_ban_second_yellow: u8,
    pub cup_tied: bool,
    pub registrations_per_season: u8,
    pub play_for_clubs_per_season: u8,
    pub max_foreign_age_youth: u8,
}

impl Default for RuleProfile {
    fn default() -> Self {
        Self {
            key: "default".into(),
            bloc: String::new(),
            squad_max: 25,
            homegrown_min: 0,
            homegrown_age_from: 15,
            homegrown_age_to: 21,
            homegrown_years: 3,
            u21_exempt_age: 21,
            foreign_squad_max: 0,
            foreign_on_pitch_max: 0,
            min_pro_age: 16,
            max_contract_years: 5,
            max_contract_years_minor: 3,
            minor_age: 18,
            minors_abroad_min_age: 18,
            bloc_minors_min_age: 16,
            work_permit: false,
            work_permit_min_points: 15,
            max_loans_in: 6,
            max_loans_out: 0,
            loans_domestic_only: false,
            red_ban_straight: 3,
            red_ban_second_yellow: 1,
            cup_tied: true,
            registrations_per_season: 3,
            play_for_clubs_per_season: 2,
            max_foreign_age_youth: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Assign {
    #[serde(default)]
    pub confed: String,
    #[serde(default)]
    pub nation: String,
    pub profile: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Bloc {
    pub key: String,
    pub nations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RulesFile {
    #[serde(default)]
    pub profile: Vec<RuleProfile>,
    #[serde(default)]
    pub assign: Vec<Assign>,
    #[serde(default)]
    pub bloc: Vec<Bloc>,
}

impl RulesFile {
    /// Profile index for a nation: explicit nation assignment, then confederation, then the first profile.
    pub fn profile_for(&self, nation_code: &str, confed_code: &str) -> usize {
        let find = |key: &str| self.profile.iter().position(|p| p.key == key);
        self.assign
            .iter()
            .find(|a| !a.nation.is_empty() && a.nation.eq_ignore_ascii_case(nation_code))
            .or_else(|| self.assign.iter().find(|a| !a.confed.is_empty() && a.confed.eq_ignore_ascii_case(confed_code)))
            .and_then(|a| find(&a.profile))
            .unwrap_or(0)
    }

    pub fn bloc_of(&self, nation_code: &str) -> Option<&str> {
        self.bloc.iter().find(|b| b.nations.iter().any(|n| n.eq_ignore_ascii_case(nation_code))).map(|b| b.key.as_str())
    }

    pub fn get(&self, i: usize) -> RuleProfile {
        self.profile.get(i).cloned().unwrap_or_default()
    }
}
