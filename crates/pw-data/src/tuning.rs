use serde::{Deserialize, Serialize};

/// Global tuning constants (17_FORMULAS_APPENDIX). Every field has a default so
/// a mod's `tuning.toml` only needs the values it changes.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Tuning {
    pub development: Development,
    pub health: Health,
    pub matches: MatchTuning,
    pub market: Market,
    pub perception: Perception,
    pub finance: Finance,
    pub squad: Squad,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Development {
    pub growth: f32,
    pub decline: f32,
    pub weekly_cap: f32,
    pub weekly_cap_youth: f32,
    pub noise: f32,
    pub pa_reroll_max: f32,
}

impl Default for Development {
    fn default() -> Self {
        Self { growth: 0.085, decline: 0.06, weekly_cap: 0.12, weekly_cap_youth: 0.20, noise: 0.02, pa_reroll_max: 15.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Health {
    /// Injury probability per high-intensity match exposure at neutral risk.
    pub match_exposure: f32,
    /// Injury probability per training session at neutral risk.
    pub training_session: f32,
    /// Daily illness probability (scaled up in winter).
    pub illness_daily: f32,
    pub acwr_safe_low: f32,
    pub acwr_safe_high: f32,
    pub condition_recovery: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { match_exposure: 0.00034, training_session: 0.00045, illness_daily: 0.0009, acwr_safe_low: 0.8, acwr_safe_high: 1.3, condition_recovery: 24.0 }
    }
}

/// Which match engine resolves fixtures. Both produce the same `MatchResult`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatchBackend {
    /// OpenFootManager's minute engine (vendored, GPL-3). Calibrated, 0.3 ms/match.
    #[default]
    Ofm,
    /// Pathway's zone/possession-chain engine (Apache-2). Richer events, still calibrating.
    Native,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchTuning {
    pub backend: MatchBackend,
    /// Logistic slope per attribute point of advantage in a contest.
    pub contest_k: f32,
    pub home_advantage: f32,
    /// Seconds of ball-in-play per match target (≈ 57 min).
    pub ball_in_play: f32,
    pub foul_rate: f32,
    pub yellow_rate: f32,
    pub red_rate: f32,
    pub penalty_share: f32,
    pub max_subs: u8,
    pub sub_windows: u8,
    pub bench_size: u8,
    pub rating_scale: f32,
    pub shot_bias: f32,
    pub finish_bias: f32,
    /// Injury probability per contact/sprint exposure at neutral risk.
    pub injury_exposure: f32,
    /// OFM backend: probability an on-target shot beats the keeper.
    pub ofm_conversion: f32,
    /// OFM backend: condition lost per minute (drives fatigue substitutions).
    pub ofm_fatigue: f32,
}

impl Default for MatchTuning {
    fn default() -> Self {
        Self {
            backend: MatchBackend::Ofm,
            contest_k: 0.32,
            home_advantage: 0.35,
            ball_in_play: 3420.0,
            foul_rate: 0.12,
            yellow_rate: 0.16,
            red_rate: 0.006,
            penalty_share: 0.08,
            max_subs: 5,
            sub_windows: 3,
            bench_size: 9,
            rating_scale: 2.2,
            shot_bias: 1.0,
            finish_bias: 1.0,
            injury_exposure: 0.0009,
            ofm_conversion: 0.225,
            ofm_fatigue: 0.45,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Market {
    pub value_base: f32,
    pub value_exp: f32,
    pub max_transfers_per_club_window: u8,
    pub rebid_cooldown_days: u16,
    pub search_interval_days: u16,
    pub renewal_lead_days_key: u16,
    pub renewal_lead_days_regular: u16,
    pub agent_fee_share: f32,
}

impl Default for Market {
    fn default() -> Self {
        Self {
            value_base: 1_000_000.0,
            value_exp: 0.042,
            max_transfers_per_club_window: 4,
            rebid_cooldown_days: 10,
            search_interval_days: 4,
            renewal_lead_days_key: 600,
            renewal_lead_days_regular: 330,
            agent_fee_share: 0.07,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Perception {
    pub sigma0: f32,
    pub decay_per_week: f32,
    pub full_knowledge_minutes: f32,
    /// World reputation above which a player is known everywhere via media.
    pub famous_reputation: u16,
}

impl Default for Perception {
    fn default() -> Self {
        Self { sigma0: 4.0, decay_per_week: 0.05, full_knowledge_minutes: 3000.0, famous_reputation: 7000 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Finance {
    /// Season revenue for a club at reputation 10,000 in a top economy.
    pub revenue_top: f32,
    /// Share of revenue a board allows for wages.
    pub wage_share: f32,
    pub ticket_price_top: f32,
    pub prize_pool_share: f32,
}

impl Default for Finance {
    fn default() -> Self {
        Self { revenue_top: 650_000_000.0, wage_share: 0.62, ticket_price_top: 75.0, prize_pool_share: 0.25 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Squad {
    pub first_team_target: u8,
    pub first_team_max: u8,
    pub youth_intake_min: u8,
    pub youth_intake_max: u8,
    pub retire_age_min: u8,
}

impl Default for Squad {
    fn default() -> Self {
        Self { first_team_target: 25, first_team_max: 30, youth_intake_min: 5, youth_intake_max: 12, retire_age_min: 32 }
    }
}
