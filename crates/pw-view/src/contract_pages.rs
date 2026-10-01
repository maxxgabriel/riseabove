//! The page payloads of the contract: what the pages that are built as JSON send (people, clubs, competitions, matches and the
//! inhabited person's own pages), declared once as types and as TypeScript.
//!
//! Unlike `contract!` types, which the pages build directly, these describe responses the pages assemble field by field. Each type
//! also deserialises and refuses fields it does not declare, so `check_response` reads a live response as its declared type: a field
//! missing, renamed, of another kind or not declared anywhere in the tree is an error. The playthrough and the contract tests run every
//! such response they make through it, in both kinds of world, observing and lived as. An `Opt` field may be left out; an `Option` field
//! is always there and may be null. `unknown` marks a value the pages send in more than one shape.
//!
//! The declarations were first written from the responses of both worlds over a playthrough and a sweep of every page, then checked
//! against the app's own interfaces for where an object may be missing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contract::Ts;

/// A field that may be left out of the response (`name?:` in TypeScript).
#[derive(Clone, Debug, Serialize)]
pub struct Opt<T>(pub Option<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Opt<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(d).map(Opt)
    }
}

impl<T: Ts> Ts for Opt<T> {
    fn ts() -> String {
        T::ts()
    }
    fn absent() -> bool {
        true
    }
}

/// A link as the pages send it (`Named` in TypeScript).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedIn {
    pub k: String,
    pub id: f64,
    pub name: String,
}

impl Ts for NamedIn {
    fn ts() -> String {
        "Named".into()
    }
}

/// A bare reference (`Ref` in TypeScript).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefIn {
    pub k: String,
    pub id: f64,
}

impl Ts for RefIn {
    fn ts() -> String {
        "Ref".into()
    }
}

/// A run of a sentence (`Part` in TypeScript).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartIn {
    pub t: String,
    pub r: Option<RefIn>,
    pub m: Option<f64>,
    pub d: Option<f64>,
}

impl Ts for PartIn {
    fn ts() -> String {
        "Part".into()
    }
}

/// Declare a page payload: a struct that serialises and deserialises (refusing undeclared fields) and its TypeScript interface.
macro_rules! response {
    ($( $(#[$m:meta])* pub struct $name:ident { $( $(#[$fm:meta])* pub $f:ident : $t:ty ),* $(,)? } )*) => {$(
        $(#[$m])*
        #[derive(Clone, Debug, Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $( $(#[$fm])* pub $f: $t ),* }

        impl Ts for $name {
            fn ts() -> String { stringify!($name).into() }
        }

        impl $name {
            pub fn declaration() -> String {
                let mut s = format!("export interface {} {{\n", stringify!($name));
                $(
                    let f = stringify!($f).trim_start_matches("r#");
                    if <$t as Ts>::absent() {
                        s.push_str(&format!("  {f}?: {};\n", <$t as Ts>::ts()));
                    } else {
                        s.push_str(&format!("  {f}: {};\n", <$t as Ts>::ts()));
                    }
                )*
                s.push_str("}\n");
                s
            }
        }
    )*};
}

response! {
    pub struct WorldSavesViewSaveFormat {
        pub note: String,
        pub schema: f64,
        pub state: String,
    }
    pub struct WorldSavesViewSaveInfo {
        pub clubs: f64,
        pub date: f64,
        pub name: String,
        pub perspective: String,
        pub players: f64,
        pub version: String,
    }
    pub struct WorldSavesViewSave {
        pub file: String,
        pub format: Option<WorldSavesViewSaveFormat>,
        pub has_backup: bool,
        pub info: WorldSavesViewSaveInfo,
        pub modified: f64,
        pub size: f64,
    }
    pub struct WorldSavesView {
        pub dir: String,
        pub saves: Vec<WorldSavesViewSave>,
    }
    pub struct RouteOptionsViewStart {
        pub age: f64,
        pub blurb: String,
        pub key: String,
        pub label: String,
    }
    pub struct RouteOptionsViewStateDistrict {
        pub id: f64,
        pub name: String,
        pub population_k: f64,
    }
    pub struct RouteOptionsViewState {
        pub districts: Vec<RouteOptionsViewStateDistrict>,
        pub id: f64,
        pub name: String,
    }
    pub struct RouteOptionsView {
        pub available: bool,
        pub starts: Vec<RouteOptionsViewStart>,
        pub states: Vec<RouteOptionsViewState>,
    }
    pub struct SearchViewGroupItem {
        pub id: f64,
        pub k: String,
        pub sub: String,
        pub title: String,
    }
    pub struct SearchViewGroup {
        pub items: Vec<SearchViewGroupItem>,
        pub label: String,
    }
    pub struct SearchView {
        pub groups: Vec<SearchViewGroup>,
    }
    pub struct OverviewViewCounts {
        pub clubs: f64,
        pub competitions: f64,
        pub nations: f64,
        pub players: f64,
    }
    pub struct OverviewViewLeagueLeader {
        pub played: f64,
        pub points: f64,
        pub team: NamedIn,
    }
    pub struct OverviewViewLeague {
        pub comp: NamedIn,
        pub leader: Option<OverviewViewLeagueLeader>,
        pub stage: String,
    }
    pub struct OverviewViewRecent {
        pub date: f64,
        pub kind: String,
        pub parts: Vec<PartIn>,
    }
    pub struct OverviewViewUpcoming {
        pub away: NamedIn,
        pub comp: String,
        pub date: f64,
        pub home: NamedIn,
        pub uid: f64,
    }
    pub struct OverviewView {
        pub counts: Option<OverviewViewCounts>,
        pub date: f64,
        pub followed: Vec<NamedIn>,
        pub leagues: Vec<OverviewViewLeague>,
        pub name: String,
        pub recent: Vec<OverviewViewRecent>,
        pub upcoming: Vec<OverviewViewUpcoming>,
    }
    pub struct WorldPulseViewItem {
        pub date: f64,
        pub id: f64,
        pub label: String,
        pub parts: Vec<PartIn>,
        pub target: PulseTarget,
    }
    pub struct WorldPulseView {
        pub items: Vec<WorldPulseViewItem>,
    }
    pub struct Capability {
        pub area: String,
        pub note: String,
        pub status: String,
    }
    pub struct DiagnosticsViewTimings {
        pub avg_ms: f64,
        pub recent: Vec<Vec<f64>>,
        pub samples: f64,
        pub worst_ms: f64,
    }
    pub struct DiagnosticsViewWorld {
        pub clubs: f64,
        pub competitions: f64,
        pub date: f64,
        pub days_simulated: f64,
        pub decisions: f64,
        pub events: f64,
        pub fixtures: f64,
        pub followed: f64,
        pub name: String,
        pub people: f64,
        pub players: f64,
        pub reports: f64,
        pub seed: f64,
        pub staff: f64,
        pub teams: f64,
    }
    pub struct DiagnosticsView {
        pub capabilities: Vec<Capability>,
        pub perspective: String,
        pub revision: f64,
        pub timings: DiagnosticsViewTimings,
        pub version: String,
        pub world: DiagnosticsViewWorld,
    }
    pub struct PersonViewPlayerAvailability {
        pub detail: String,
        pub label: String,
        pub tone: String,
    }
    pub struct PersonViewPlayerCondition {
        pub condition: f64,
        pub confidence: f64,
        pub fatigue: f64,
        pub fitness: f64,
        pub morale: f64,
        pub sharpness: f64,
        pub wellbeing: f64,
    }
    pub struct PersonViewPlayerContract {
        pub appearance_bonus: f64,
        pub assist_bonus: f64,
        pub cap_bonus: f64,
        pub clean_sheet_bonus: f64,
        pub club: NamedIn,
        pub continental_bonus: f64,
        pub days_left: f64,
        pub end: f64,
        pub goal_bonus: f64,
        pub kind: String,
        pub loyalty_bonus: f64,
        pub options: Vec<Value>,
        pub promised_status: Value,
        pub promotion_bonus: f64,
        pub release_clause: f64,
        pub relegation_cut: f64,
        pub relegation_release: f64,
        pub start: f64,
        pub title_bonus: f64,
        pub wage: f64,
        pub yearly_rise: f64,
    }
    pub struct PersonViewPlayerInternalPlan {
        pub focus: String,
        pub intensity: String,
    }
    pub struct PersonViewPlayerInternalReputation {
        pub current: f64,
        pub home: f64,
        pub world: f64,
    }
    pub struct PersonViewPlayerInternal {
        pub bio_offset: f64,
        pub ca: f64,
        pub pa: f64,
        pub personality: String,
        pub plan: Option<PersonViewPlayerInternalPlan>,
        pub reputation: PersonViewPlayerInternalReputation,
    }
    pub struct PersonViewPlayerLoan {
        pub club: NamedIn,
        pub end: f64,
        pub parent: NamedIn,
    }
    pub struct PersonViewPlayerPosition {
        pub code: String,
        pub fam: f64,
        pub level: String,
    }
    pub struct PersonViewPlayer {
        pub availability: PersonViewPlayerAvailability,
        pub best_pos: String,
        pub caps: f64,
        pub career_coverage: String,
        pub condition: Option<PersonViewPlayerCondition>,
        pub contract: Option<PersonViewPlayerContract>,
        pub foot: String,
        pub form: Vec<f64>,
        pub height: f64,
        pub internal: Option<PersonViewPlayerInternal>,
        pub intl_goals: f64,
        pub joined: Option<f64>,
        pub loan: Option<PersonViewPlayerLoan>,
        pub player_id: f64,
        pub positions: Vec<PersonViewPlayerPosition>,
        pub senior_apps: f64,
        pub senior_goals: f64,
        pub shirt: f64,
        pub squad_status: Option<String>,
        pub team: Option<String>,
        pub traits: Vec<String>,
        pub value: Option<f64>,
        pub weight: f64,
        pub youth_club: Option<NamedIn>,
    }
    pub struct PersonViewRole {
        pub label: String,
        pub org: Option<NamedIn>,
    }
    pub struct PersonViewStaffAttr {
        pub label: String,
        pub v: f64,
    }
    pub struct PersonViewStaffContract {
        pub end: f64,
        pub wage: f64,
    }
    pub struct PersonViewStaffRecord {
        pub draws: f64,
        pub games: f64,
        pub losses: f64,
        pub sackings: f64,
        pub trophies: f64,
        pub wins: f64,
    }
    pub struct PersonViewStaffStyle {
        pub archetype: String,
        pub directness: f64,
        pub formations: Vec<String>,
        pub mentality: f64,
        pub press: f64,
        pub tempo: f64,
    }
    pub struct PersonViewStaff {
        pub attrs: Option<Vec<PersonViewStaffAttr>>,
        pub club: Option<NamedIn>,
        pub contract: Option<PersonViewStaffContract>,
        pub joined: f64,
        pub record: PersonViewStaffRecord,
        pub reputation: f64,
        pub role: String,
        pub role_rating: Option<f64>,
        pub staff_id: f64,
        pub style: Option<PersonViewStaffStyle>,
    }
    pub struct PersonView {
        pub age: f64,
        pub can_inhabit: bool,
        pub dob: f64,
        pub id: f64,
        pub initials: String,
        pub is_me: bool,
        pub name: String,
        pub nations: Vec<NamedIn>,
        pub perspective: String,
        pub player: Option<PersonViewPlayer>,
        pub provenance: Value,
        pub roles: Vec<PersonViewRole>,
        pub short: String,
        pub staff: Option<PersonViewStaff>,
        pub status: String,
    }
    pub struct CrestColorsView {
        pub colors: std::collections::BTreeMap<String, Vec<String>>,
    }
    pub struct CompOverviewViewHeld {
        pub results: f64,
        pub withheld: Vec<Value>,
    }
    pub struct CompOverviewViewLeftRowTeam {
        pub colors: Vec<String>,
        pub full: String,
        pub id: f64,
        pub k: String,
        pub me: bool,
        pub name: String,
    }
    pub struct CompOverviewViewLeftRow {
        pub gd: f64,
        pub played: f64,
        pub points: f64,
        pub pos: f64,
        pub team: Option<CompOverviewViewLeftRowTeam>,
        pub zone: Option<String>,
    }
    pub struct CompOverviewViewLeftTy {
        pub a: CompOverviewViewLeftRowTeam,
        pub b: CompOverviewViewLeftRowTeam,
        pub goals_a: Option<f64>,
        pub goals_b: Option<f64>,
        pub hidden: bool,
        pub legs: f64,
        pub played: f64,
        pub winner: Option<String>,
    }
    pub struct CompOverviewViewLeft {
        pub date: Opt<Option<f64>>,
        pub kind: String,
        pub round: Opt<String>,
        pub rows: Opt<Vec<CompOverviewViewLeftRow>>,
        pub shown: Opt<f64>,
        pub ties: Opt<Vec<CompOverviewViewLeftTy>>,
        pub title: String,
        pub total: f64,
    }
    pub struct CompOverviewViewMeta {
        pub label: String,
        pub r#ref: Opt<NamedIn>,
        pub sub: Opt<String>,
        pub value: Value,
    }
    pub struct CompOverviewViewNew {
        pub away: Option<CompOverviewViewLeftRowTeam>,
        pub date: f64,
        pub days_ago: f64,
        pub headline: String,
        pub home: CompOverviewViewLeftRowTeam,
        pub r#match: RefIn,
        pub outlet: String,
        pub score: Vec<f64>,
        pub spoils: bool,
    }
    pub struct CompOverviewViewPlayerRow {
        pub p: NamedIn,
        pub pill: bool,
        pub team: Option<CompOverviewViewLeftRowTeam>,
        pub value: String,
    }
    pub struct CompOverviewViewPlayer {
        pub page: f64,
        pub rows: Vec<CompOverviewViewPlayerRow>,
        pub title: String,
    }
    pub struct CompOverviewViewTeamsStatRow {
        pub team: Option<CompOverviewViewLeftRowTeam>,
        pub value: String,
    }
    pub struct CompOverviewViewTeamsStat {
        pub page: f64,
        pub rows: Vec<CompOverviewViewTeamsStatRow>,
        pub title: String,
    }
    pub struct CompOverviewViewTicker {
        pub r#as: Option<f64>,
        pub away: Option<CompOverviewViewLeftRowTeam>,
        pub date: f64,
        pub home: CompOverviewViewLeftRowTeam,
        pub hs: Option<f64>,
        pub pens: Option<Vec<f64>>,
        pub round: f64,
        pub status: String,
        pub uid: f64,
    }
    pub struct CompOverviewView {
        pub held: CompOverviewViewHeld,
        pub id: f64,
        pub kind: String,
        pub kind_key: String,
        pub left: CompOverviewViewLeft,
        pub meta: Vec<CompOverviewViewMeta>,
        pub name: String,
        pub news: Vec<CompOverviewViewNew>,
        pub next: Option<NamedIn>,
        pub players: Vec<CompOverviewViewPlayer>,
        pub prev: Option<NamedIn>,
        pub season: String,
        pub short: String,
        pub stage: String,
        pub teams: f64,
        pub teams_stats: Vec<CompOverviewViewTeamsStat>,
        pub ticker: Vec<CompOverviewViewTicker>,
        pub tier: f64,
    }
    pub struct ClubViewAcademy {
        pub age_groups: Vec<String>,
        pub kind: String,
        pub name: String,
        pub origin: String,
        pub residential: Option<bool>,
    }
    pub struct ClubViewAlsoKnown {
        pub kind: String,
        pub origin: String,
        pub text: String,
    }
    pub struct ClubViewBoard {
        pub patience: f64,
        pub satisfaction: f64,
        pub target_position: f64,
        pub warnings: f64,
    }
    pub struct ClubViewChannel {
        pub name: String,
        pub real: bool,
    }
    pub struct ClubViewFacilities {
        pub academy: f64,
        pub medical: f64,
        pub training: f64,
        pub youth: f64,
    }
    pub struct ClubViewFinance {
        pub balance: f64,
        pub debt: f64,
        pub season_income: f64,
        pub season_spend: f64,
        pub transfer_budget: f64,
        pub wage_bill: f64,
        pub wage_budget: f64,
    }
    pub struct ClubViewLeague {
        pub comp: NamedIn,
        pub played: Option<f64>,
        pub points: Option<f64>,
        pub position: Option<f64>,
        pub teams: f64,
    }
    pub struct ClubViewManagerRecord {
        pub draws: f64,
        pub games: f64,
        pub losses: f64,
        pub wins: f64,
    }
    pub struct ClubViewManager {
        pub person: NamedIn,
        pub record: ClubViewManagerRecord,
        pub since: f64,
    }
    pub struct ClubViewNeed {
        pub max_age: f64,
        pub min_ability: f64,
        pub pos: String,
        pub urgency: f64,
    }
    pub struct ClubViewPartner {
        pub active: bool,
        pub origin: String,
        pub purpose: String,
        pub what: String,
        pub with: String,
    }
    pub struct ClubViewStaffCount {
        pub count: f64,
        pub role: String,
    }
    pub struct ClubViewTeam {
        pub captain: Option<NamedIn>,
        pub comp: Option<NamedIn>,
        pub kind: String,
        pub kind_key: String,
        pub squad: f64,
        pub team: f64,
    }
    pub struct ClubView {
        pub academy: Option<ClubViewAcademy>,
        pub also_known: Vec<ClubViewAlsoKnown>,
        pub board: Option<ClubViewBoard>,
        pub capacity: f64,
        pub channels: Vec<ClubViewChannel>,
        pub city: String,
        pub colors: Vec<String>,
        pub facilities: Option<ClubViewFacilities>,
        pub fan_mood: f64,
        pub finance: Option<ClubViewFinance>,
        pub followed: bool,
        pub founded: f64,
        pub id: f64,
        pub league: Option<ClubViewLeague>,
        pub manager: Option<ClubViewManager>,
        pub name: String,
        pub nation: NamedIn,
        pub needs: Option<Vec<ClubViewNeed>>,
        pub ownership: String,
        pub partners: Vec<ClubViewPartner>,
        pub relation: String,
        pub reputation: f64,
        pub short: String,
        pub stadium: String,
        pub staff_counts: Vec<ClubViewStaffCount>,
        pub teams: Vec<ClubViewTeam>,
    }
    pub struct ClubSystemsViewBoardConcern {
        pub label: String,
        pub value: f64,
    }
    pub struct ClubSystemsViewBoardOwnerTraits {
        pub ambition: f64,
        pub fan_sensitivity: f64,
        pub frugality: f64,
        pub meddling: f64,
        pub patience: f64,
        pub wealth: f64,
    }
    pub struct ClubSystemsViewBoardPolicy {
        pub debt_tolerance: f64,
        pub max_signing_age: f64,
        pub sell_to_rivals: bool,
        pub selling_stance: f64,
        pub style_mandate: f64,
        pub transfer_style: String,
        pub wage_cap_mult: f64,
        pub youth_investment: f64,
        pub youth_minutes_target: f64,
    }
    pub struct ClubSystemsViewBoardProject {
        pub completes: f64,
        pub cost: Option<f64>,
        pub kind: String,
        pub started: f64,
        pub target: f64,
    }
    pub struct ClubSystemsViewBoard {
        pub administration: Option<f64>,
        pub chairman: NamedIn,
        pub concerns: Option<Vec<ClubSystemsViewBoardConcern>>,
        pub kind: String,
        pub owner: NamedIn,
        pub owner_traits: Option<ClubSystemsViewBoardOwnerTraits>,
        pub policy: Option<ClubSystemsViewBoardPolicy>,
        pub projects: Vec<ClubSystemsViewBoardProject>,
        pub red_months: Option<f64>,
        pub revenue_history: Option<Vec<f64>>,
        pub since: f64,
    }
    pub struct ClubSystemsViewCultureIdentity {
        pub flair: f64,
        pub glamour: f64,
        pub grit: f64,
        pub local: f64,
        pub underdog: f64,
        pub youth: f64,
    }
    pub struct ClubSystemsViewCulture {
        pub discipline: f64,
        pub drought: f64,
        pub expectations: f64,
        pub graduates: f64,
        pub identity: ClubSystemsViewCultureIdentity,
        pub patience: f64,
        pub tribalism: f64,
    }
    pub struct ClubSystemsViewPlanGroup {
        pub ageing_starters: f64,
        pub avg_age: f64,
        pub depth: f64,
        pub expiring: f64,
        pub group: String,
        pub injured: f64,
        pub prospects: f64,
        pub quality: f64,
        pub target_depth: f64,
        pub target_quality: f64,
    }
    pub struct ClubSystemsViewPlanNeed {
        pub fee_band: f64,
        pub group: String,
        pub homegrown: bool,
        pub max_age: f64,
        pub min_ability: f64,
        pub role: String,
        pub urgency: f64,
        pub wage_band: f64,
    }
    pub struct ClubSystemsViewPlan {
        pub built: f64,
        pub groups: Vec<ClubSystemsViewPlanGroup>,
        pub homegrown_gap: f64,
        pub needs: Vec<ClubSystemsViewPlanNeed>,
        pub promote: Vec<NamedIn>,
        pub sell: Vec<NamedIn>,
    }
    pub struct ClubSystemsViewRivalry {
        pub intensity: f64,
        pub last_met: Option<f64>,
        pub record: Vec<f64>,
        pub since: f64,
        pub why: Vec<String>,
        pub with: NamedIn,
    }
    pub struct ClubSystemsViewRoomGroup {
        pub bond: String,
        pub cohesion: f64,
        pub leader: NamedIn,
        pub size: f64,
        pub stance: f64,
    }
    pub struct ClubSystemsViewRoomInfluential {
        pub influence: f64,
        pub standing: String,
        pub who: NamedIn,
    }
    pub struct ClubSystemsViewRoom {
        pub backing: f64,
        pub groups: Vec<ClubSystemsViewRoomGroup>,
        pub harmony: f64,
        pub influential: Vec<ClubSystemsViewRoomInfluential>,
        pub updated: f64,
    }
    pub struct ClubSystemsViewScoutingScout {
        pub based: String,
        pub briefs: Vec<String>,
        pub capacity: f64,
        pub who: NamedIn,
    }
    pub struct ClubSystemsViewScouting {
        pub reports: f64,
        pub scouts: Vec<ClubSystemsViewScoutingScout>,
    }
    pub struct ClubSystemsViewSponsor {
        pub brand: String,
        pub fee: Option<f64>,
        pub slot: String,
        pub until: f64,
    }
    pub struct ClubSystemsViewSupporter {
        pub board: f64,
        pub kind: String,
        pub last_acted: f64,
        pub manager: f64,
        pub size: f64,
        pub team: f64,
        pub voice: f64,
    }
    pub struct ClubSystemsView {
        pub board: Option<ClubSystemsViewBoard>,
        pub culture: Option<ClubSystemsViewCulture>,
        pub internal: bool,
        pub plan: Option<ClubSystemsViewPlan>,
        pub rivalries: Vec<ClubSystemsViewRivalry>,
        pub room: Option<ClubSystemsViewRoom>,
        pub scouting: Option<ClubSystemsViewScouting>,
        pub sponsors: Vec<ClubSystemsViewSponsor>,
        pub supporters: Vec<ClubSystemsViewSupporter>,
    }
    pub struct CompViewRules {
        pub away_goals: bool,
        pub bench: f64,
        pub extra_time: bool,
        pub foreigner_limit: f64,
        pub subs: f64,
        pub yellow_limit: f64,
    }
    pub struct CompViewState {
        pub end: Option<f64>,
        pub knockout: bool,
        pub round: f64,
        pub runner_up: Option<NamedIn>,
        pub season: String,
        pub season_year: f64,
        pub stage: String,
        pub start: Option<f64>,
        pub teams: f64,
        pub winner: Option<NamedIn>,
    }
    pub struct CompViewTy {
        pub a: NamedIn,
        pub b: NamedIn,
        pub goals_a: f64,
        pub goals_b: f64,
        pub hidden: bool,
        pub index: f64,
        pub legs: f64,
        pub played: f64,
        pub round: f64,
        pub winner: Option<NamedIn>,
    }
    pub struct CompView {
        pub above: Option<NamedIn>,
        pub below: Option<NamedIn>,
        pub continental_places: Vec<Value>,
        pub format: String,
        pub groups: f64,
        pub id: f64,
        pub is_league: bool,
        pub kind: String,
        pub kind_key: String,
        pub name: String,
        pub nation: NamedIn,
        pub past_editions: f64,
        pub prize_pool: Option<f64>,
        pub promote: f64,
        pub relegate: f64,
        pub reputation: f64,
        pub rules: CompViewRules,
        pub short: String,
        pub size: f64,
        pub state: Option<CompViewState>,
        pub team_kind: String,
        pub tier: f64,
        pub ties: Vec<CompViewTy>,
    }
    pub struct NationViewLeague {
        pub comp: NamedIn,
        pub teams: f64,
        pub tier: f64,
    }
    pub struct NationViewSeason {
        pub end: Option<f64>,
        pub label: String,
        pub start: Option<f64>,
        pub windows: Vec<Vec<f64>>,
        pub winter_break: Value,
    }
    pub struct NationViewSideRecord {
        pub drawn: f64,
        pub lost: f64,
        pub won: f64,
    }
    pub struct NationViewSide {
        pub captain: Option<NamedIn>,
        pub level: String,
        pub manager: NamedIn,
        pub record: NationViewSideRecord,
        pub selected: f64,
        pub since: f64,
        pub squad: f64,
    }
    pub struct NationViewWorldEconomy {
        pub broadcast_pool: Option<f64>,
        pub coefficient: f64,
        pub deal_until: Option<f64>,
        pub growth: Option<f64>,
        pub league_strength: f64,
        pub rank: f64,
        pub wage_index: Option<f64>,
    }
    pub struct NationView {
        pub clubs: f64,
        pub code: String,
        pub confed: String,
        pub cups: Vec<NamedIn>,
        pub economy: Option<f64>,
        pub id: f64,
        pub leagues: Vec<NationViewLeague>,
        pub name: String,
        pub players: f64,
        pub reputation: f64,
        pub season: NationViewSeason,
        pub sides: Vec<NationViewSide>,
        pub window_open: bool,
        pub world_economy: Option<NationViewWorldEconomy>,
        pub youth_rating: Option<f64>,
    }
    pub struct MatchViewAway {
        pub club: f64,
        pub colors: Vec<String>,
        pub mine: bool,
        pub short: String,
        pub team: NamedIn,
    }
    pub struct MatchViewDetailEvent {
        pub kind: String,
        pub label: String,
        pub minute: f64,
        pub other: Option<NamedIn>,
        pub player: Option<NamedIn>,
        pub side: f64,
        pub t: f64,
        pub tier: String,
        pub x: f64,
        pub xg: f64,
        pub y: f64,
    }
    pub struct MatchViewDetailLine {
        pub assists: f64,
        pub clearances: f64,
        pub conceded: f64,
        pub dribbles: f64,
        pub dribbles_won: f64,
        pub fouls: f64,
        pub goals: f64,
        pub injured: bool,
        pub interceptions: f64,
        pub keeper: bool,
        pub key_passes: f64,
        pub minutes: f64,
        pub off_at: f64,
        pub on_at: f64,
        pub on_target: f64,
        pub passes: f64,
        pub passes_completed: f64,
        pub player: NamedIn,
        pub pos: Option<String>,
        pub rating: f64,
        pub reds: f64,
        pub saves: f64,
        pub shots: f64,
        pub side: f64,
        pub started: bool,
        pub tackles: f64,
        pub tackles_won: f64,
        pub x: f64,
        pub xa: f64,
        pub xg: f64,
        pub y: f64,
        pub yellows: f64,
    }
    pub struct MatchViewDetailShapeAway {
        pub formation: String,
    }
    pub struct MatchViewDetailStat {
        pub big_chances: f64,
        pub corners: f64,
        pub fouls: f64,
        pub offsides: f64,
        pub on_target: f64,
        pub passes: f64,
        pub passes_completed: f64,
        pub possession: f64,
        pub reds: f64,
        pub saves: f64,
        pub shots: f64,
        pub tackles: f64,
        pub xg: f64,
        pub yellows: f64,
    }
    pub struct MatchViewDetail {
        pub events: Vec<MatchViewDetailEvent>,
        pub lines: Vec<MatchViewDetailLine>,
        pub man_of_the_match: NamedIn,
        pub shape_away: Option<MatchViewDetailShapeAway>,
        pub shape_home: Option<MatchViewDetailShapeAway>,
        pub stats: Vec<MatchViewDetailStat>,
    }
    pub struct MatchViewPreAwayAbsence {
        pub player: NamedIn,
        pub why: String,
    }
    pub struct MatchViewPreAway {
        pub absences: Vec<MatchViewPreAwayAbsence>,
        pub position: Option<f64>,
    }
    pub struct MatchViewPre {
        pub away: Option<MatchViewPreAway>,
        pub home: MatchViewPreAway,
    }
    pub struct MatchViewPreviou {
        pub away: String,
        pub comp: String,
        pub date: f64,
        pub home: String,
        pub score: String,
        pub uid: f64,
    }
    pub struct MatchViewRules {
        pub bench: f64,
        pub extra_time: bool,
        pub subs: f64,
    }
    pub struct MatchViewScore {
        pub away: f64,
        pub extra_time: bool,
        pub home: f64,
        pub ht_away: f64,
        pub ht_home: f64,
        pub pens: Option<Vec<f64>>,
        pub text: String,
    }
    pub struct MatchView {
        pub away: Option<MatchViewAway>,
        pub can_follow: bool,
        pub capacity: Option<f64>,
        pub comp: NamedIn,
        pub concealed: bool,
        pub date: f64,
        pub decisive: bool,
        pub detail: Option<MatchViewDetail>,
        pub detail_kept: bool,
        pub home: MatchViewAway,
        pub neutral: bool,
        pub pre: Option<MatchViewPre>,
        pub previous: Vec<MatchViewPreviou>,
        pub round: String,
        pub rules: MatchViewRules,
        pub score: Option<MatchViewScore>,
        pub status: String,
        pub uid: f64,
        pub venue: String,
        pub watching: bool,
    }
    pub struct MeTodayViewAvailability {
        pub ban: f64,
        pub days: f64,
        pub injured: bool,
        pub injury: Option<String>,
    }
    pub struct MeTodayViewChange {
        pub date: f64,
        pub id: String,
        pub important: bool,
        pub kind: String,
        pub parts: Vec<PartIn>,
    }
    pub struct MeTodayViewCommitment {
        pub kind: String,
        pub r#ref: Opt<RefIn>,
        pub text: String,
    }
    pub struct MeTodayViewConditionCondition {
        pub label: String,
        pub step: f64,
        pub steps: f64,
    }
    pub struct MeTodayViewCondition {
        pub condition: Option<MeTodayViewConditionCondition>,
        pub confidence: MeTodayViewConditionCondition,
        pub fatigue: MeTodayViewConditionCondition,
        pub morale: MeTodayViewConditionCondition,
        pub sharpness: MeTodayViewConditionCondition,
        pub wellbeing: MeTodayViewConditionCondition,
    }
    pub struct MeTodayViewContract {
        pub club: NamedIn,
        pub days_left: f64,
        pub end: f64,
        pub status: String,
        pub wage: f64,
    }
    pub struct MeTodayViewDay {
        pub kind: String,
        pub label: String,
    }
    pub struct MeTodayViewLeague {
        pub comp: NamedIn,
        pub points: f64,
        pub position: f64,
        pub teams: f64,
    }
    pub struct MeTodayViewMe {
        pub age: f64,
        pub club: Option<NamedIn>,
        pub name: String,
        pub person: f64,
        pub position: String,
        pub shirt: f64,
        pub squad_status: Option<String>,
        pub status: String,
        pub team: Option<String>,
    }
    pub struct MeTodayViewMind {
        pub pull: String,
        pub text: String,
    }
    pub struct MeTodayViewNextMatch {
        pub comp: NamedIn,
        pub date: f64,
        pub days: f64,
        pub home: bool,
        pub opponent: NamedIn,
        pub round: String,
        pub uid: f64,
        pub venue: Value,
    }
    pub struct MeTodayViewPlanFocus {
        pub kind: String,
        pub value: Option<String>,
    }
    pub struct MeTodayViewPlan {
        pub extra: f64,
        pub focus: Option<MeTodayViewPlanFocus>,
        pub intensity: String,
        pub recovery: f64,
    }
    pub struct MeTodayViewPromises {
        pub next_due: Option<f64>,
        pub open: f64,
    }
    pub struct MeTodayViewRecent {
        pub comp: NamedIn,
        pub concealed: bool,
        pub date: f64,
        pub days: f64,
        pub home: bool,
        pub opponent: NamedIn,
        pub outcome: Opt<String>,
        pub round: String,
        pub score: Opt<String>,
        pub uid: f64,
        pub venue: Value,
    }
    pub struct MeTodayViewWaitingOn {
        pub date: Opt<f64>,
        pub kind: String,
        pub r#ref: Opt<RefIn>,
        pub since: f64,
        pub text: String,
    }
    pub struct MeTodayView {
        pub availability: MeTodayViewAvailability,
        pub changes: Vec<MeTodayViewChange>,
        pub commitments: Vec<MeTodayViewCommitment>,
        pub conceal_mine: bool,
        pub condition: Option<MeTodayViewCondition>,
        pub contract: Option<MeTodayViewContract>,
        pub date: f64,
        pub day: MeTodayViewDay,
        pub decisions: Vec<Value>,
        pub form: Vec<f64>,
        pub last_viewed: f64,
        pub league: Option<MeTodayViewLeague>,
        pub lifestyle: String,
        pub me: Option<MeTodayViewMe>,
        pub mind: Vec<MeTodayViewMind>,
        pub minutes_4w: f64,
        pub next_match: Option<MeTodayViewNextMatch>,
        pub plan: Option<MeTodayViewPlan>,
        pub plan_pending: Option<MeTodayViewPlan>,
        pub promises: MeTodayViewPromises,
        pub queued: Vec<String>,
        pub recent: Vec<MeTodayViewRecent>,
        pub routine_hours: f64,
        pub unrevealed: Vec<Value>,
        pub waiting_on: Vec<MeTodayViewWaitingOn>,
        pub weekday: f64,
    }
    /// One line of the message list: a decision (`dkind`, `preview`) or an event (`parts`, `unread`).
    pub struct MeMessagesViewMessage {
        pub date: f64,
        pub deadline: Option<f64>,
        pub folder: String,
        pub from: Value,
        pub id: String,
        pub important: bool,
        pub kind: String,
        pub needs_action: bool,
        pub dkind: Opt<String>,
        pub preview: Opt<String>,
        pub parts: Opt<Vec<PartIn>>,
        pub state: String,
        pub subject: String,
        pub unread: Opt<bool>,
    }
    pub struct MeMessagesView {
        pub awaiting: f64,
        pub messages: Vec<MeMessagesViewMessage>,
        pub unread: f64,
    }
    pub struct MeInboxViewThread {
        pub count: f64,
        pub deadline: Value,
        pub id: f64,
        pub kind: String,
        pub last: f64,
        pub last_kind: String,
        pub needs_action: bool,
        pub opened: f64,
        pub preview: String,
        pub title: String,
        pub unread: f64,
        pub with: Option<NamedIn>,
    }
    pub struct MeInboxView {
        pub awaiting: f64,
        pub threads: Vec<MeInboxViewThread>,
        pub unread: f64,
    }
    pub struct MeThreadViewMessageMeetingLine {
        pub name: String,
        pub r#ref: RefIn,
        pub side: String,
        pub text: String,
        pub tone: String,
    }
    pub struct MeThreadViewMessageMeeting {
        pub date: f64,
        pub lines: Vec<MeThreadViewMessageMeetingLine>,
        pub outcomes: Vec<String>,
        pub state: String,
        pub topic: String,
        pub why: Vec<String>,
        pub with: NamedIn,
    }
    pub struct MeThreadViewMessagePostAuthor {
        pub display: String,
        pub followers: f64,
        pub handle: String,
        pub kind: String,
        pub person: Value,
        pub you: bool,
    }
    pub struct MeThreadViewMessagePostParent {
        pub about: Option<NamedIn>,
        pub author: MeThreadViewMessagePostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Value,
        pub quote_of: Value,
        pub quoted: Value,
        pub replies: f64,
        pub reply_to: Value,
        pub reposts: f64,
        pub text: String,
    }
    pub struct MeThreadViewMessagePost {
        pub about: Option<NamedIn>,
        pub author: MeThreadViewMessagePostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Option<MeThreadViewMessagePostParent>,
        pub quote_of: Option<f64>,
        pub quoted: Option<MeThreadViewMessagePostParent>,
        pub replies: f64,
        pub reply_to: Option<f64>,
        pub reposts: f64,
        pub text: String,
    }
    pub struct MeThreadViewMessageReplied {
        pub date: f64,
        pub label: String,
    }
    pub struct MeThreadViewMessageReply {
        pub effect: String,
        pub key: String,
        pub label: String,
        pub quiet: bool,
    }
    pub struct MeThreadViewMessageStory {
        pub body: String,
        pub date: f64,
        pub headline: String,
        pub id: f64,
        pub outlet: String,
    }
    pub struct MeThreadViewMessage {
        pub date: f64,
        pub from: Option<NamedIn>,
        pub id: f64,
        pub kind: String,
        pub label: Opt<String>,
        pub meeting: Opt<Option<MeThreadViewMessageMeeting>>,
        pub parts: Opt<Vec<PartIn>>,
        pub post: Opt<Option<MeThreadViewMessagePost>>,
        pub read: bool,
        pub replied: Option<MeThreadViewMessageReplied>,
        pub replies: Vec<MeThreadViewMessageReply>,
        pub story: Opt<Option<MeThreadViewMessageStory>>,
        pub text: String,
        pub decision: Opt<ThreadDecision>,
        /// How sure the person who told you was, in words.
        pub sureness: Opt<String>,
    }
    pub struct MeThreadView {
        pub id: f64,
        pub kind: String,
        pub last: f64,
        pub messages: Vec<MeThreadViewMessage>,
        pub opened: f64,
        pub title: String,
        pub with: Option<NamedIn>,
    }
    pub struct MeMessageViewStory {
        pub body: String,
        pub date: f64,
        pub headline: String,
        pub outlet: String,
    }
    /// `me.message` for an event.
    pub struct MeMessageView {
        pub date: f64,
        pub id: String,
        pub kind: String,
        pub meeting: Option<MeThreadViewMessageMeeting>,
        pub parts: Vec<PartIn>,
        pub primary: Option<RefIn>,
        pub story: Option<MeMessageViewStory>,
        pub title: String,
        pub why: Vec<String>,
    }
    pub struct MeOptionsViewAgent {
        pub base: String,
        pub clients: f64,
        pub id: f64,
        pub person: NamedIn,
        pub reputation: f64,
    }
    pub struct MeOptionsViewCareer {
        pub key: String,
        pub label: String,
    }
    pub struct MeOptionsViewCours {
        pub cost: f64,
        pub done: bool,
        pub effort: f64,
        pub key: String,
        pub label: String,
        pub requires: Option<String>,
        pub studying: bool,
    }
    pub struct MeOptionsViewHelperHired {
        pub cost: f64,
        pub quality: f64,
    }
    pub struct MeOptionsViewHelper {
        pub base_cost: f64,
        pub hired: Option<MeOptionsViewHelperHired>,
        pub key: String,
        pub label: String,
    }
    pub struct MeOptionsViewMeetWith {
        pub role: String,
        pub who: NamedIn,
    }
    pub struct MeOptionsViewNation {
        pub id: f64,
        pub name: String,
    }
    pub struct MeOptionsView {
        pub agents: Vec<MeOptionsViewAgent>,
        pub careers: Vec<MeOptionsViewCareer>,
        pub courses: Vec<MeOptionsViewCours>,
        pub helpers: Vec<MeOptionsViewHelper>,
        pub lifestyles: Vec<MeOptionsViewCareer>,
        pub meet_with: Vec<MeOptionsViewMeetWith>,
        pub nations: Vec<MeOptionsViewNation>,
        pub posts: Vec<MeOptionsViewCareer>,
        pub roles: Vec<MeOptionsViewCareer>,
        pub routine_budget: f64,
        pub stances: Vec<MeOptionsViewCareer>,
        pub tones: Vec<MeOptionsViewCareer>,
        pub topics: Vec<MeOptionsViewCareer>,
    }
    pub struct MeSelfViewCareer {
        pub apps: f64,
        pub caps: f64,
        pub clubs: f64,
        pub goals: f64,
        pub position: String,
        pub status: String,
    }
    pub struct MeSelfViewMood {
        pub factor: String,
        pub pull: String,
        pub text: String,
    }
    pub struct MeSelfViewTold {
        pub date: f64,
        pub sureness: String,
        pub text: String,
    }
    pub struct MeSelfView {
        pub age: f64,
        pub career: MeSelfViewCareer,
        pub fulfilment: MeTodayViewConditionCondition,
        pub hint: String,
        pub mood: Vec<MeSelfViewMood>,
        pub name: String,
        pub nation: NamedIn,
        pub personality: String,
        pub sleep: MeTodayViewConditionCondition,
        pub stress: MeTodayViewConditionCondition,
        pub told: Vec<MeSelfViewTold>,
        pub wellbeing: Vec<MeSelfViewMood>,
    }
    pub struct MeLifeViewGiving {
        pub community: f64,
        pub foundation: bool,
        pub pct: f64,
    }
    pub struct MeLifeViewHelper {
        pub cost: f64,
        pub label: String,
        pub quality: f64,
    }
    pub struct MeLifeViewHome {
        pub kind: Option<String>,
        pub nation: NamedIn,
        pub quality: Option<f64>,
        pub since: f64,
    }
    pub struct MeLifeViewLanguage {
        pub level: MeTodayViewConditionCondition,
        pub nation: NamedIn,
    }
    pub struct MeLifeViewMoney {
        pub debt: f64,
        pub family_support: f64,
        pub income: f64,
        pub invested: f64,
        pub lifestyle: String,
        pub savings: f64,
        pub spending: f64,
    }
    pub struct MeLifeViewParents {
        pub alive: f64,
        pub closeness: String,
        pub health: String,
        pub nation: String,
    }
    pub struct MeLifeViewPartner {
        pub bond: MeTodayViewConditionCondition,
        pub lives: Option<String>,
        pub occupation: String,
        pub since: f64,
        pub status: String,
        pub who: NamedIn,
    }
    pub struct MeLifeViewRoutineRow {
        pub hours: f64,
        pub key: String,
        pub label: String,
    }
    pub struct MeLifeViewRoutine {
        pub budget: f64,
        pub rows: Vec<MeLifeViewRoutineRow>,
        pub total: f64,
    }
    pub struct MeLifeViewStudying {
        pub course: String,
        pub done: f64,
        pub effort: f64,
    }
    pub struct MeLifeView {
        pub children: f64,
        pub education: f64,
        pub giving: Option<MeLifeViewGiving>,
        pub helpers: Vec<MeLifeViewHelper>,
        pub home: MeLifeViewHome,
        pub languages: Vec<MeLifeViewLanguage>,
        pub money: Option<MeLifeViewMoney>,
        pub morale: Vec<MeSelfViewMood>,
        pub occupation: String,
        pub open_to_dating: Option<bool>,
        pub parents: MeLifeViewParents,
        pub partner: Option<MeLifeViewPartner>,
        pub qualifications: Vec<MeThreadViewMessageReplied>,
        pub routine: MeLifeViewRoutine,
        pub siblings: f64,
        pub studying: Option<MeLifeViewStudying>,
        pub wellbeing: Vec<MeSelfViewMood>,
        pub work: Value,
    }
    pub struct MePressViewFan {
        pub club: NamedIn,
        pub label: String,
        pub reasons: Vec<String>,
        pub score: f64,
    }
    pub struct MePressViewQuote {
        pub about: Value,
        pub at_conference: bool,
        pub date: f64,
        pub id: f64,
        pub stance: String,
    }
    pub struct MePressViewReaction {
        pub date: f64,
        pub sentiment: f64,
        pub text: String,
    }
    pub struct MePressViewStory {
        pub about_you: bool,
        pub date: f64,
        pub headline: String,
        pub id: f64,
        pub outlet: String,
    }
    pub struct MePressView {
        pub fans: Vec<MePressViewFan>,
        pub image: String,
        pub quotes: Vec<MePressViewQuote>,
        pub reactions: Vec<MePressViewReaction>,
        pub stories: Vec<MePressViewStory>,
    }
    pub struct MeFeedViewAccount {
        pub followers: f64,
        pub handle: String,
    }
    pub struct MeFeedViewPostAuthor {
        pub display: String,
        pub followers: f64,
        pub handle: String,
        pub kind: String,
        pub person: Option<NamedIn>,
        pub you: bool,
    }
    pub struct MeFeedViewPostParent {
        pub about: Option<NamedIn>,
        pub author: MeThreadViewMessagePostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Value,
        pub quote_of: Value,
        pub quoted: Value,
        pub replies: f64,
        pub reply_to: Option<f64>,
        pub reposts: f64,
        pub text: String,
    }
    pub struct MeFeedViewPost {
        pub about: Option<NamedIn>,
        pub author: MeFeedViewPostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Option<MeFeedViewPostParent>,
        pub quote_of: Option<f64>,
        pub quoted: Option<MeThreadViewMessagePostParent>,
        pub replies: f64,
        pub reply_to: Option<f64>,
        pub reposts: f64,
        pub text: String,
    }
    pub struct MeFeedView {
        pub account: Option<MeFeedViewAccount>,
        pub posts: Vec<MeFeedViewPost>,
    }
    pub struct SocialThreadViewPost {
        pub about: Option<NamedIn>,
        pub author: MeFeedViewPostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Option<MeFeedViewPostParent>,
        pub quote_of: Value,
        pub quoted: Value,
        pub replies: f64,
        pub reply_to: Option<f64>,
        pub reposts: f64,
        pub text: String,
    }
    pub struct SocialThreadViewReply {
        pub about: Option<NamedIn>,
        pub author: MeThreadViewMessagePostAuthor,
        pub date: f64,
        pub id: f64,
        pub likes: f64,
        pub parent: Value,
        pub quote_of: Value,
        pub quoted: Value,
        pub replies: f64,
        pub reply_to: f64,
        pub reposts: f64,
        pub text: String,
    }
    pub struct SocialThreadView {
        pub post: Option<SocialThreadViewPost>,
        pub replies: Vec<SocialThreadViewReply>,
    }
    pub struct MeCalendarViewDayEntry {
        pub kind: String,
        pub label: String,
        pub r#ref: Opt<RefIn>,
        pub required: Opt<bool>,
        pub result: Opt<String>,
        pub source: String,
        pub state: String,
        pub sub: Opt<String>,
    }
    pub struct MeCalendarViewDay {
        pub date: f64,
        pub entries: Vec<MeCalendarViewDayEntry>,
    }
    pub struct MeCalendarView {
        pub days: Vec<MeCalendarViewDay>,
        pub from: f64,
        pub to: f64,
        pub today: f64,
    }
    pub struct MeFootballViewOptionsAttribute {
        pub group: String,
        pub key: String,
        pub label: String,
    }
    pub struct MeFootballViewOptionsPosition {
        pub code: String,
    }
    pub struct MeFootballViewOptions {
        pub attributes: Vec<MeFootballViewOptionsAttribute>,
        pub positions: Vec<MeFootballViewOptionsPosition>,
    }
    pub struct MeFootballViewRival {
        pub age: f64,
        pub available: bool,
        pub minutes_4w: f64,
        pub player: NamedIn,
        pub pos: String,
    }
    pub struct MeFootballViewSeason {
        pub apps: f64,
        pub minutes: f64,
        pub starts: f64,
    }
    pub struct MeFootballViewUsagePlayed {
        pub assists: f64,
        pub goals: f64,
        pub minutes: f64,
        pub rating: f64,
        pub started: bool,
    }
    pub struct MeFootballViewUsage {
        pub comp: NamedIn,
        pub concealed: bool,
        pub date: f64,
        pub days: f64,
        pub home: bool,
        pub opponent: NamedIn,
        pub outcome: Opt<String>,
        pub played: Opt<Option<MeFootballViewUsagePlayed>>,
        pub round: String,
        pub score: Opt<String>,
        pub uid: f64,
        pub venue: Value,
    }
    pub struct MeFootballView {
        pub minutes_4w: f64,
        pub options: Option<MeFootballViewOptions>,
        pub plan: Option<MeTodayViewPlan>,
        pub plan_pending: Option<MeTodayViewPlan>,
        pub promised: Value,
        pub rivals: Vec<MeFootballViewRival>,
        pub season: MeFootballViewSeason,
        pub squad_status: String,
        pub team: Option<String>,
        pub usage: Vec<MeFootballViewUsage>,
    }
    pub struct MeContractViewAgent {
        pub fee_pct: f64,
        pub satisfaction: String,
        pub until: f64,
        pub who: NamedIn,
    }
    pub struct MeContractViewSummary {
        pub days_left: f64,
        pub end: f64,
        pub kind: String,
        pub status: String,
        pub wage: f64,
    }
    pub struct MeContractViewTerm {
        pub date: Opt<f64>,
        pub label: String,
        pub money: Opt<Option<f64>>,
        pub text: Opt<String>,
    }
    pub struct MeContractView {
        pub agent: Opt<Option<MeContractViewAgent>>,
        pub club: Opt<NamedIn>,
        pub guaranteed_note: Opt<String>,
        pub has_contract: bool,
        pub history: Opt<Vec<Value>>,
        pub listed: Opt<bool>,
        pub loan: Opt<Value>,
        pub offers: Opt<Vec<Value>>,
        pub status: Opt<String>,
        pub summary: Opt<Option<MeContractViewSummary>>,
        pub talks: Opt<Value>,
        pub terms: Opt<Vec<MeContractViewTerm>>,
        pub transfer_request: Opt<Option<f64>>,
    }
}

response! {
    /// One answer to a decision, as the inbox offers it: the engine's label and what choosing it does where the engine words the
    /// decision (`consequence`).
    pub struct DecisionOptionView {
        pub i: f64,
        pub label: String,
        pub kind: String,
        pub positive: bool,
        pub default: bool,
        pub consequence: Opt<String>,
        pub effect: Opt<String>,
        pub tone: Opt<String>,
        pub counter: Opt<Value>,
    }
    /// The decision a thread message carries.
    pub struct ThreadDecision {
        pub id: String,
        pub state: String,
        pub title: String,
        pub deadline: f64,
        pub options: Vec<DecisionOptionView>,
        pub kind: String,
    }
    /// The chosen answer that applies if none is given.
    pub struct DecisionDefault {
        pub i: f64,
        pub label: String,
    }
    /// `me.message` for a decision: what it is, what each answer does, and how it ended. The blocks for talks, meetings, incidents
    /// and press questions are present only for those kinds of decision.
    pub struct DecisionDetailView {
        pub id: String,
        pub kind: String,
        pub dkind: String,
        pub title: String,
        pub from: Value,
        pub created: f64,
        pub deadline: f64,
        pub state: String,
        pub paragraphs: Vec<String>,
        pub options: Vec<DecisionOptionView>,
        pub answer: Option<f64>,
        pub default: DecisionDefault,
        pub without_response: Option<String>,
        pub consequences: Vec<String>,
        pub terms: Value,
        pub current_terms: Value,
        pub talk: Value,
        pub meeting: Value,
        pub incident: Value,
        pub press: Value,
        pub outcome: Option<String>,
    }
    /// Where a pulse item leads: a person, club, competition or match, or a story (`k` "news").
    pub struct PulseTarget {
        pub k: String,
        pub id: f64,
    }
    /// `world.datasets`: the consolidated databases found on this machine; `database` is the dataset's own manifest.
    pub struct DatasetsView {
        pub datasets: Vec<DatasetRow>,
    }
    pub struct DatasetRow {
        pub path: String,
        pub database: Value,
    }
    /// `world.inspect_import`: what a folder holds before it is imported. When it cannot be read as an import, `ok` is false and
    /// `error` says why; otherwise the counts, warnings and findings are there.
    pub struct InspectImportView {
        pub ok: bool,
        pub files: Vec<ImportFile>,
        pub counts: Opt<ImportCounts>,
        pub warnings: Opt<Vec<String>>,
        pub findings: Opt<Vec<ImportFinding>>,
        pub start: Opt<Option<f64>>,
        pub database: Opt<Value>,
        pub error: Opt<String>,
    }
    pub struct ImportFile {
        pub name: String,
        pub size: f64,
    }
    pub struct ImportCounts {
        pub nations: f64,
        pub competitions: f64,
        pub clubs: f64,
        pub players: f64,
        pub staff: f64,
        pub unresolved: f64,
    }
    pub struct ImportFinding {
        pub code: String,
        pub count: f64,
    }
    /// `database.sources`: the source folders connected for browsing, and why any could not be.
    pub struct DatabaseSourcesView {
        pub sources: Vec<DatabaseSource>,
        pub errors: Vec<String>,
    }
    pub struct DatabaseSource {
        pub id: f64,
        pub path: String,
        pub tables: Vec<DatabaseTable>,
    }
    pub struct DatabaseTable {
        pub name: String,
        pub bytes: f64,
        pub source: String,
        pub status: String,
    }
    /// `database.attach`: the folder connected, and every connected source.
    pub struct DatabaseAttached {
        pub id: f64,
        pub sources: Vec<DatabaseSource>,
    }
    /// `database.query`: one page of a source table. `refs` (Transfermarkt tables) links a row's ids to the world's people, clubs and
    /// competitions where the import kept them.
    pub struct DatabaseQueryView {
        pub table: String,
        pub columns: Vec<String>,
        pub indexed_columns: Vec<String>,
        pub rows: Vec<DatabaseRecord>,
        pub total: f64,
        pub matched: f64,
        pub malformed: f64,
        pub offset: f64,
        pub name_search: bool,
        pub source: f64,
    }
    pub struct DatabaseRecord {
        pub row: f64,
        pub fields: std::collections::BTreeMap<String, Option<String>>,
        pub refs: Opt<std::collections::BTreeMap<String, RefIn>>,
    }
}

/// Every page payload declaration, in order.
pub fn declarations() -> Vec<String> {
    vec![
        DecisionOptionView::declaration(),
        ThreadDecision::declaration(),
        DecisionDefault::declaration(),
        DecisionDetailView::declaration(),
        "/** `me.message`: an event, or a decision in full. */\nexport type MeMessage = MeMessageView | DecisionDetailView;\n".to_string(),
        PulseTarget::declaration(),
        DatasetsView::declaration(),
        DatasetRow::declaration(),
        InspectImportView::declaration(),
        ImportFile::declaration(),
        ImportCounts::declaration(),
        ImportFinding::declaration(),
        DatabaseSourcesView::declaration(),
        DatabaseSource::declaration(),
        DatabaseTable::declaration(),
        DatabaseAttached::declaration(),
        DatabaseQueryView::declaration(),
        DatabaseRecord::declaration(),
        WorldSavesViewSaveFormat::declaration(),
        WorldSavesViewSaveInfo::declaration(),
        WorldSavesViewSave::declaration(),
        WorldSavesView::declaration(),
        RouteOptionsViewStart::declaration(),
        RouteOptionsViewStateDistrict::declaration(),
        RouteOptionsViewState::declaration(),
        RouteOptionsView::declaration(),
        SearchViewGroupItem::declaration(),
        SearchViewGroup::declaration(),
        SearchView::declaration(),
        OverviewViewCounts::declaration(),
        OverviewViewLeagueLeader::declaration(),
        OverviewViewLeague::declaration(),
        OverviewViewRecent::declaration(),
        OverviewViewUpcoming::declaration(),
        OverviewView::declaration(),
        WorldPulseViewItem::declaration(),
        WorldPulseView::declaration(),
        Capability::declaration(),
        DiagnosticsViewTimings::declaration(),
        DiagnosticsViewWorld::declaration(),
        DiagnosticsView::declaration(),
        PersonViewPlayerAvailability::declaration(),
        PersonViewPlayerCondition::declaration(),
        PersonViewPlayerContract::declaration(),
        PersonViewPlayerInternalPlan::declaration(),
        PersonViewPlayerInternalReputation::declaration(),
        PersonViewPlayerInternal::declaration(),
        PersonViewPlayerLoan::declaration(),
        PersonViewPlayerPosition::declaration(),
        PersonViewPlayer::declaration(),
        PersonViewRole::declaration(),
        PersonViewStaffAttr::declaration(),
        PersonViewStaffContract::declaration(),
        PersonViewStaffRecord::declaration(),
        PersonViewStaffStyle::declaration(),
        PersonViewStaff::declaration(),
        PersonView::declaration(),
        CrestColorsView::declaration(),
        CompOverviewViewHeld::declaration(),
        CompOverviewViewLeftRowTeam::declaration(),
        CompOverviewViewLeftRow::declaration(),
        CompOverviewViewLeftTy::declaration(),
        CompOverviewViewLeft::declaration(),
        CompOverviewViewMeta::declaration(),
        CompOverviewViewNew::declaration(),
        CompOverviewViewPlayerRow::declaration(),
        CompOverviewViewPlayer::declaration(),
        CompOverviewViewTeamsStatRow::declaration(),
        CompOverviewViewTeamsStat::declaration(),
        CompOverviewViewTicker::declaration(),
        CompOverviewView::declaration(),
        ClubViewAcademy::declaration(),
        ClubViewAlsoKnown::declaration(),
        ClubViewBoard::declaration(),
        ClubViewChannel::declaration(),
        ClubViewFacilities::declaration(),
        ClubViewFinance::declaration(),
        ClubViewLeague::declaration(),
        ClubViewManagerRecord::declaration(),
        ClubViewManager::declaration(),
        ClubViewNeed::declaration(),
        ClubViewPartner::declaration(),
        ClubViewStaffCount::declaration(),
        ClubViewTeam::declaration(),
        ClubView::declaration(),
        ClubSystemsViewBoardConcern::declaration(),
        ClubSystemsViewBoardOwnerTraits::declaration(),
        ClubSystemsViewBoardPolicy::declaration(),
        ClubSystemsViewBoardProject::declaration(),
        ClubSystemsViewBoard::declaration(),
        ClubSystemsViewCultureIdentity::declaration(),
        ClubSystemsViewCulture::declaration(),
        ClubSystemsViewPlanGroup::declaration(),
        ClubSystemsViewPlanNeed::declaration(),
        ClubSystemsViewPlan::declaration(),
        ClubSystemsViewRivalry::declaration(),
        ClubSystemsViewRoomGroup::declaration(),
        ClubSystemsViewRoomInfluential::declaration(),
        ClubSystemsViewRoom::declaration(),
        ClubSystemsViewScoutingScout::declaration(),
        ClubSystemsViewScouting::declaration(),
        ClubSystemsViewSponsor::declaration(),
        ClubSystemsViewSupporter::declaration(),
        ClubSystemsView::declaration(),
        CompViewRules::declaration(),
        CompViewState::declaration(),
        CompViewTy::declaration(),
        CompView::declaration(),
        NationViewLeague::declaration(),
        NationViewSeason::declaration(),
        NationViewSideRecord::declaration(),
        NationViewSide::declaration(),
        NationViewWorldEconomy::declaration(),
        NationView::declaration(),
        MatchViewAway::declaration(),
        MatchViewDetailEvent::declaration(),
        MatchViewDetailLine::declaration(),
        MatchViewDetailShapeAway::declaration(),
        MatchViewDetailStat::declaration(),
        MatchViewDetail::declaration(),
        MatchViewPreAwayAbsence::declaration(),
        MatchViewPreAway::declaration(),
        MatchViewPre::declaration(),
        MatchViewPreviou::declaration(),
        MatchViewRules::declaration(),
        MatchViewScore::declaration(),
        MatchView::declaration(),
        MeTodayViewAvailability::declaration(),
        MeTodayViewChange::declaration(),
        MeTodayViewCommitment::declaration(),
        MeTodayViewConditionCondition::declaration(),
        MeTodayViewCondition::declaration(),
        MeTodayViewContract::declaration(),
        MeTodayViewDay::declaration(),
        MeTodayViewLeague::declaration(),
        MeTodayViewMe::declaration(),
        MeTodayViewMind::declaration(),
        MeTodayViewNextMatch::declaration(),
        MeTodayViewPlanFocus::declaration(),
        MeTodayViewPlan::declaration(),
        MeTodayViewPromises::declaration(),
        MeTodayViewRecent::declaration(),
        MeTodayViewWaitingOn::declaration(),
        MeTodayView::declaration(),
        MeMessagesViewMessage::declaration(),
        MeMessagesView::declaration(),
        MeInboxViewThread::declaration(),
        MeInboxView::declaration(),
        MeThreadViewMessageMeetingLine::declaration(),
        MeThreadViewMessageMeeting::declaration(),
        MeThreadViewMessagePostAuthor::declaration(),
        MeThreadViewMessagePostParent::declaration(),
        MeThreadViewMessagePost::declaration(),
        MeThreadViewMessageReplied::declaration(),
        MeThreadViewMessageReply::declaration(),
        MeThreadViewMessageStory::declaration(),
        MeThreadViewMessage::declaration(),
        MeThreadView::declaration(),
        MeMessageViewStory::declaration(),
        MeMessageView::declaration(),
        MeOptionsViewAgent::declaration(),
        MeOptionsViewCareer::declaration(),
        MeOptionsViewCours::declaration(),
        MeOptionsViewHelperHired::declaration(),
        MeOptionsViewHelper::declaration(),
        MeOptionsViewMeetWith::declaration(),
        MeOptionsViewNation::declaration(),
        MeOptionsView::declaration(),
        MeSelfViewCareer::declaration(),
        MeSelfViewMood::declaration(),
        MeSelfViewTold::declaration(),
        MeSelfView::declaration(),
        MeLifeViewGiving::declaration(),
        MeLifeViewHelper::declaration(),
        MeLifeViewHome::declaration(),
        MeLifeViewLanguage::declaration(),
        MeLifeViewMoney::declaration(),
        MeLifeViewParents::declaration(),
        MeLifeViewPartner::declaration(),
        MeLifeViewRoutineRow::declaration(),
        MeLifeViewRoutine::declaration(),
        MeLifeViewStudying::declaration(),
        MeLifeView::declaration(),
        MePressViewFan::declaration(),
        MePressViewQuote::declaration(),
        MePressViewReaction::declaration(),
        MePressViewStory::declaration(),
        MePressView::declaration(),
        MeFeedViewAccount::declaration(),
        MeFeedViewPostAuthor::declaration(),
        MeFeedViewPostParent::declaration(),
        MeFeedViewPost::declaration(),
        MeFeedView::declaration(),
        SocialThreadViewPost::declaration(),
        SocialThreadViewReply::declaration(),
        SocialThreadView::declaration(),
        MeCalendarViewDayEntry::declaration(),
        MeCalendarViewDay::declaration(),
        MeCalendarView::declaration(),
        MeFootballViewOptionsAttribute::declaration(),
        MeFootballViewOptionsPosition::declaration(),
        MeFootballViewOptions::declaration(),
        MeFootballViewRival::declaration(),
        MeFootballViewSeason::declaration(),
        MeFootballViewUsagePlayed::declaration(),
        MeFootballViewUsage::declaration(),
        MeFootballView::declaration(),
        MeContractViewAgent::declaration(),
        MeContractViewSummary::declaration(),
        MeContractViewTerm::declaration(),
        MeContractView::declaration(),
    ]
}

/// The declared response type of a page method, as TypeScript.
pub fn response_type(method: &str) -> Option<String> {
    Some(match method {
        "capabilities" => <Vec<Capability> as Ts>::ts(),
        "club" => <ClubView as Ts>::ts(),
        "club.systems" => <ClubSystemsView as Ts>::ts(),
        "comp" => <CompView as Ts>::ts(),
        "comp.overview" => <CompOverviewView as Ts>::ts(),
        "crest.colors" => <CrestColorsView as Ts>::ts(),
        "diagnostics" => <DiagnosticsView as Ts>::ts(),
        "match" => <MatchView as Ts>::ts(),
        "match.watch" => <MatchView as Ts>::ts(),
        "me.calendar" => <MeCalendarView as Ts>::ts(),
        "me.contract" => <MeContractView as Ts>::ts(),
        "me.feed" => <MeFeedView as Ts>::ts(),
        "me.football" => <MeFootballView as Ts>::ts(),
        "me.inbox" => <MeInboxView as Ts>::ts(),
        "me.life" => <MeLifeView as Ts>::ts(),
        "me.message" => "MeMessage".to_string(),
        "me.messages" => <MeMessagesView as Ts>::ts(),
        "me.options" => <MeOptionsView as Ts>::ts(),
        "me.press" => <MePressView as Ts>::ts(),
        "me.self" => <MeSelfView as Ts>::ts(),
        "me.thread" => <MeThreadView as Ts>::ts(),
        "me.today" => <MeTodayView as Ts>::ts(),
        "nation" => <NationView as Ts>::ts(),
        "overview" => <OverviewView as Ts>::ts(),
        "person" => <PersonView as Ts>::ts(),
        "person.life" => <MeLifeView as Ts>::ts(),
        "route.options" => <RouteOptionsView as Ts>::ts(),
        "search" => <SearchView as Ts>::ts(),
        "social.thread" => <SocialThreadView as Ts>::ts(),
        "world.pulse" => <WorldPulseView as Ts>::ts(),
        "world.saves" => <WorldSavesView as Ts>::ts(),
        "world.datasets" => <DatasetsView as Ts>::ts(),
        "world.inspect_import" => <InspectImportView as Ts>::ts(),
        "database.sources" => <DatabaseSourcesView as Ts>::ts(),
        "database.attach" => <DatabaseAttached as Ts>::ts(),
        "database.query" => <DatabaseQueryView as Ts>::ts(),
        _ => return None,
    })
}

// ------------------------------------------------------------------ checking a response against the TypeScript

/// The declarations of the contract's TypeScript, parsed once: interfaces (fields, and whether each may be left out) and type aliases.
struct Decls {
    interfaces: std::collections::HashMap<String, Vec<(String, bool, String)>>,
    aliases: std::collections::HashMap<String, String>,
}

fn decls() -> &'static Decls {
    static D: std::sync::OnceLock<Decls> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let ts = crate::contract::typescript();
        let mut interfaces = std::collections::HashMap::new();
        let mut aliases = std::collections::HashMap::new();
        let mut lines = ts.lines().peekable();
        while let Some(line) = lines.next() {
            if let Some(rest) = line.strip_prefix("export interface ") {
                let head = rest.trim_end_matches('{').trim();
                let (name, parent) = match head.split_once(" extends ") {
                    Some((n, p)) => (n.trim().to_string(), Some(p.trim().to_string())),
                    None => (head.to_string(), None),
                };
                let mut fields: Vec<(String, bool, String)> = parent.and_then(|p| interfaces.get(&p).cloned()).unwrap_or_default();
                for l in lines.by_ref() {
                    let l = l.trim();
                    if l == "}" {
                        break;
                    }
                    if l.starts_with("/*") || l.starts_with('*') || l.starts_with("//") {
                        continue;
                    }
                    let Some((n, t)) = l.trim_end_matches(';').split_once(':') else { continue };
                    let (n, opt) = match n.strip_suffix('?') {
                        Some(n) => (n, true),
                        None => (n, false),
                    };
                    fields.push((n.trim().to_string(), opt, t.trim().to_string()));
                }
                interfaces.insert(name, fields);
            } else if let Some(rest) = line.strip_prefix("export type ") {
                let Some((name, first)) = rest.split_once('=') else { continue };
                // A generic alias (`Knowledge<T>`) is kept under its bare name with its parameters in front: `T|...`.
                let (name, params) = match name.trim().split_once('<') {
                    Some((n, p)) => (n.to_string(), format!("{}|", p.trim_end_matches('>').trim())),
                    None => (name.trim().to_string(), "|".to_string()),
                };
                let mut text = first.trim().to_string();
                while !text.ends_with(';') {
                    match lines.next() {
                        Some(l) => {
                            text.push(' ');
                            text.push_str(l.trim());
                        }
                        None => break,
                    }
                }
                aliases.insert(name, format!("{params}{}", text.trim_end_matches(';').trim().trim_start_matches('|').trim()));
            }
        }
        Decls { interfaces, aliases }
    })
}

/// Split a type at its top-level `|`.
fn alternatives(t: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' | b'<' | b'{' | b'[' => depth += 1,
            b')' | b'>' | b'}' | b']' => depth -= 1,
            b'|' if depth == 0 => {
                out.push(t[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(t[start..].trim());
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Split a type at its top-level `&`.
fn parts(t: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in t.char_indices() {
        match c {
            '(' | '<' | '{' | '[' => depth += 1,
            ')' | '>' | '}' | ']' => depth -= 1,
            '&' if depth == 0 => {
                out.push(t[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(t[start..].trim());
    out
}

/// The text of an alias used as `t` (`Knowledge<number>` is `Knowledge`'s text with `T` read as `number`).
fn alias(t: &str) -> Option<String> {
    let (name, args) = match t.split_once('<') {
        Some((n, a)) if !t.starts_with("Record<") => (n.trim(), Some(a.trim_end_matches('>').trim())),
        _ => (t, None),
    };
    let text = decls().aliases.get(name)?;
    let (params, body) = text.split_once('|').unwrap_or(("", text));
    let mut body = body.to_string();
    if let (Some(args), false) = (args, params.is_empty()) {
        for (p, a) in params.split(',').zip(args.split(',')) {
            body = replace_word(&body, p.trim(), a.trim());
        }
    }
    Some(body)
}

fn replace_word(s: &str, word: &str, with: &str) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else {
            out.push_str(if cur == word { with } else { &cur });
            cur.clear();
            out.push(c);
        }
    }
    out.push_str(if cur == word { with } else { &cur });
    out
}

type Field = (String, bool, String);

/// The field sets an object-like type allows: one for an interface or an inline object, one per alternative for a union of them, the
/// combinations for an intersection. `None` for a type that is not an object.
fn field_sets(t: &str) -> Option<Vec<Vec<Field>>> {
    let t = t.trim();
    let ps = parts(t);
    if ps.len() > 1 {
        let mut acc: Vec<Vec<Field>> = vec![Vec::new()];
        for p in ps {
            let sets = field_sets(p)?;
            acc = acc.iter().flat_map(|a| sets.iter().map(move |s| a.iter().cloned().chain(s.iter().cloned()).collect())).collect();
        }
        return Some(acc);
    }
    let alts = alternatives(t);
    if alts.len() > 1 {
        let mut out = Vec::new();
        for a in alts {
            out.extend(field_sets(a)?);
        }
        return Some(out);
    }
    if t.starts_with('{') {
        let body = t.trim_start_matches('{').trim_end_matches('}');
        let fields = body
            .split(';')
            .filter_map(|f| {
                let (n, ft) = f.split_once(':')?;
                let (n, opt) = match n.trim().strip_suffix('?') {
                    Some(n) => (n.to_string(), true),
                    None => (n.trim().to_string(), false),
                };
                Some((n, opt, ft.trim().to_string()))
            })
            .collect();
        return Some(vec![fields]);
    }
    if let Some(f) = decls().interfaces.get(t) {
        return Some(vec![f.clone()]);
    }
    alias(t).and_then(|a| field_sets(&a))
}

/// Does the object `o` have exactly one of these field sets? The error is the closest miss.
fn object_fits(sets: &[Vec<Field>], o: &serde_json::Map<String, Value>, name: &str, path: &str) -> Result<(), String> {
    let mut best: Option<(usize, String)> = None;
    for fields in sets {
        let mut err = None;
        let mut fit = 0;
        for (n, opt, ft) in fields {
            match o.get(n) {
                Some(x) => match conforms(ft, x, &format!("{path}.{n}")) {
                    Ok(()) => fit += 1,
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                },
                None if *opt => {}
                None => {
                    err = Some(format!("{path}.{n}: missing ({name})"));
                    break;
                }
            }
        }
        if err.is_none() {
            if let Some(k) = o.keys().find(|k| !fields.iter().any(|f| &f.0 == *k)) {
                err = Some(format!("{path}.{k}: not declared in {name}"));
            }
        }
        match err {
            None => return Ok(()),
            Some(e) => {
                if best.as_ref().is_none_or(|(b, _)| fit > *b) {
                    best = Some((fit, e));
                }
            }
        }
    }
    Err(best.map_or_else(|| format!("{path}: no shape of {name}"), |b| b.1))
}

/// Does `v` have the TypeScript type `t`? The error names the path to the first thing that does not fit.
pub fn conforms(t: &str, v: &Value, path: &str) -> Result<(), String> {
    let t = t.trim();
    if parts(t).len() > 1 {
        let Some(o) = v.as_object() else { return Err(format!("{path}: expected an object, found {}", short(v))) };
        let sets = field_sets(t).ok_or_else(|| format!("{path}: cannot read the type {t}"))?;
        return object_fits(&sets, o, t, path);
    }
    let alts = alternatives(t);
    if alts.len() > 1 {
        let mut first = None;
        for a in &alts {
            match conforms(a, v, path) {
                Ok(()) => return Ok(()),
                Err(e) => first = first.or(Some(e)),
            }
        }
        return Err(first.unwrap_or_default());
    }
    let bad = |want: &str| Err(format!("{path}: expected {want}, found {}", short(v)));
    match t {
        "unknown" => Ok(()),
        "null" => if v.is_null() { Ok(()) } else { bad("null") },
        "number" => if v.is_number() { Ok(()) } else { bad("a number") },
        "string" => if v.is_string() { Ok(()) } else { bad("a string") },
        "boolean" => if v.is_boolean() { Ok(()) } else { bad("true or false") },
        _ if t.starts_with('"') => if v.as_str() == Some(t.trim_matches('"')) { Ok(()) } else { bad(t) },
        _ if t.ends_with("[]") => {
            let inner = t[..t.len() - 2].trim();
            let inner = inner.strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(inner);
            let Some(a) = v.as_array() else { return bad("a list") };
            a.iter().enumerate().try_for_each(|(i, x)| conforms(inner, x, &format!("{path}[{i}]")))
        }
        _ if t.starts_with("Record<string,") => {
            let inner = t.trim_start_matches("Record<string,").trim_end_matches('>');
            let Some(o) = v.as_object() else { return bad("an object") };
            o.iter().try_for_each(|(k, x)| conforms(inner, x, &format!("{path}.{k}")))
        }
        _ if t.starts_with('{') => {
            let Some(o) = v.as_object() else { return bad("an object") };
            object_fits(&field_sets(t).unwrap_or_default(), o, "the inline type", path)
        }
        name => {
            let d = decls();
            if let Some(fields) = d.interfaces.get(name) {
                let Some(o) = v.as_object() else { return bad(name) };
                return object_fits(std::slice::from_ref(fields), o, name, path);
            }
            match alias(name) {
                Some(a) => conforms(&a, v, path),
                None => Err(format!("{path}: no declaration of {name}")),
            }
        }
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 80 { format!("{}...", &s[..80]) } else { s }
}

/// Read a method's live response as its declared type, all the way down. `None` for a method whose response is not declared.
pub fn check_response(method: &str, v: &Value) -> Option<Result<(), String>> {
    let res = crate::contract::manifest().into_iter().find(|m| m.name == method)?.response?;
    Some(conforms(res, v, method))
}

#[cfg(test)]
mod tests {
    use super::conforms;
    use serde_json::json;

    #[test]
    fn a_response_fits_its_type_all_the_way_down_and_a_miss_names_its_path() {
        // An interface with a nested link, a list and a nullable object.
        let club = json!({"academy": null, "also_known": [], "board": null, "capacity": 0, "channels": [], "city": "", "colors": ["#fff"], "facilities": null,
            "fan_mood": 50, "finance": null, "followed": false, "founded": 1900, "id": 1, "league": null, "manager": null, "name": "A", "nation": {"k": "nation", "id": 0, "name": "N"},
            "needs": null, "ownership": "", "partners": [], "relation": "", "reputation": 1, "short": "A", "stadium": "", "staff_counts": [], "teams": []});
        assert_eq!(conforms("ClubView", &club, "club"), Ok(()));
        let mut extra = club.clone();
        extra["secret"] = json!(1);
        assert_eq!(conforms("ClubView", &extra, "club"), Err("club.secret: not declared in ClubView".into()));
        let mut bad = club.clone();
        bad["nation"]["k"] = json!("planet");
        assert!(conforms("ClubView", &bad, "club").unwrap_err().starts_with("club.nation.k:"));
        let mut missing = club;
        missing.as_object_mut().unwrap().remove("teams");
        assert_eq!(conforms("ClubView", &missing, "club"), Err("club.teams: missing (ClubView)".into()));
        // A generic alias in an intersection: every alternative of the union, with the extra fields.
        assert_eq!(conforms("AttrRow", &json!({"key": "pace", "label": "Pace", "kind": "range", "lo": 10, "hi": 14, "v": 12}), "a"), Ok(()));
        assert_eq!(conforms("AttrRow", &json!({"key": "pace", "label": "Pace", "kind": "unknown"}), "a"), Ok(()));
        assert!(conforms("AttrRow", &json!({"key": "pace", "label": "Pace", "kind": "range", "v": 12}), "a").is_err());
        assert!(conforms("AttrRow", &json!({"key": "pace", "kind": "unknown"}), "a").is_err());
        // Maps, optional fields and nullable values.
        assert_eq!(conforms("Record<string, number>", &json!({"a": 1, "b": 2}), "m"), Ok(()));
        assert!(conforms("Record<string, number>", &json!({"a": "x"}), "m").is_err());
        assert_eq!(conforms("string | null", &json!(null), "s"), Ok(()));
    }
}
