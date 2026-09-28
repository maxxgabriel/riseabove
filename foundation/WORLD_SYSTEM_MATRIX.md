# World System Matrix

Each cell: Classification (A/B/C/D/E) + one-line evidence from source.

A = usable almost directly | B = portable with adaptation | C = reference only | D = incomplete stub | E = unsuitable

---

| System | OpenFootManager (GPL-3) | open-football (Apache-2) |
|--------|------------------------|--------------------------|
| **Clubs** | B — team.rs has finance, formation, training_groups, player_roles, satisfaction; no facilities | B — club/board/core/, estate.rs, ownership/benefactor — deeper but different entity model |
| **Competitions** | B — league.rs has fixtures, standings, group stage, knockout, seasons; moderate depth | A — continent/competitions/, national/, global/, cup structures all real |
| **Player aging** | A — aging.rs: seeded pace-decay 30+, technical growth <32, retirement 33-37+ with modifiers | B — development.rs: CoachingEffect + age curves; different API |
| **Retirement** | A — apply_seasonal_aging() with chance table 33-37; skip controlled player | B — player/calculators.rs: FreeAgentReleaseReason includes retirement |
| **Player development** | B — training.rs: coaching bonus, specialization, physio, focus, fatigue guard; real | A — development.rs: FM-style CA/PA model, coaching effect, 1-20 skills |
| **Injuries** | D — Injury has name + days_remaining only; no type, severity, recovery mechanics | B — injury/processing.rs: MedicalStaffQuality, InjurySeverity, InjuryType |
| **Manager careers** | B — ai_hiring.rs + firing.rs: satisfaction→warning→fire→replacement cycle; real | B — board/manager/: full search, candidate scoring, market, seat; deeper |
| **Manager AI selection** | A — team_builder.rs: positional_fit + form + fitness + trust + assessment + load management | C — club/team/selection?: not confirmed at code level; need more investigation |
| **Transfers (permanent)** | B — transfers/market.rs + bids.rs + execution.rs: full multi-round; rebid cooldown | B — transfers/pipeline.rs + strategy.rs: RecruitmentPolicy + NegotiationPolicy; deeper policies |
| **Transfers (loans)** | B — transfers/loans.rs: loan offers, wage contribution pct, buy option; active loan tracking | B — implied by TransferClause; loan preference in NegotiationPolicy |
| **Contract renewals** | B — contracts/renewals.rs + delegated_renewals.rs: assistant-managed for AI; player conversation rounds | B — player/mailbox.rs: PlayerContractAsk; mind-driven |
| **Free agents** | B — contracts/free_agent.rs: AI clubs pick up out-of-contract players | B — player/calculators.rs: FreeAgentReleaseReason + market_state |
| **Scouting** | B — scouting.rs: assignments, youth scouting regions, report generation | B — club/staff/perception.rs: AbilityEstimator, PotentialEstimator, CoachEye |
| **Youth pipeline** | B — scouting.rs youth assignments; squad_role: Youth exists; limited generation | A — academy/: intake.rs + training.rs + graduation.rs + tuning.rs; full system |
| **Finances** | B — finances/mod.rs: wages, transfers, sponsorship, marketing, board support; real | B — club/finance/balance.rs + ledger.rs: complex multi-source |
| **Reputation** | B — reputation.rs: player/club reputation tracked; market_value derived | A — PlayerAttributes has current/home/world reputation, international_apps |
| **Standings/tables** | A — league.rs standings via fixtures | A — competitions/: standings computed per fixture |
| **Promotion/relegation** | A — end_of_season/berths.rs | A — continent/rankings/ + competition transitions |
| **National teams** | B — national_team.rs: squad selection + matches | A — continent/national/: full NationalSquadBuilder + international calendar |
| **Match output (per-player)** | A — PlayerMatchStats: minutes, goals, assists, shots, passes, tackles, fouls, cards, rating | B — match stats exist; PlayerCareer tracking; need to verify per-player read API |
| **Match output (events)** | A — EventType: 15+ event types per minute | B — match engine embedded; events fire but extraction API unconfirmed |
| **Multi-season headless** | C — ofm-cli exists but not a standalone world-sim binary | A — dev/simulate: verified headless daily tick; accepts days parameter |
| **Player career mode** | A — is_player_career(), controlled_player(), player_career module; explicit | D — no player career concept; purely autonomous world sim |
| **Selection forecast** | A — player_selection_outlook(): predicts if protagonist selected, identifies dominant factor | E — no concept of human-controlled player |
| **Protagonist protection** | A — transfer market gates on controlled_player_is_known_to_buyer(); retirement exempted | E — not applicable; all players are equal autonomous agents |
| **History persistence** | A — career.rs: CareerEntry with attribute snapshots; movement_history; awards_won | B — statistics.rs + PlayerStatisticsHistory; persisted per-club |
| **News/media system** | B — news.rs + inbox.rs: articles with categories, scheduled delivery | B — club/news/ + country/media/: real but no human-facing narrative |

---

## Summary Scores

| Dimension | OFM | open-football |
|-----------|-----|---------------|
| World autonomy depth | 7/10 | 9/10 |
| Player career support | 9/10 | 1/10 |
| Match per-player data | 9/10 | 6/10 |
| Transfer realism | 7/10 | 8/10 |
| Development realism | 7/10 | 9/10 |
| Multi-season headless | 5/10 | 9/10 |
| Attribute depth | 5/10 (19 attrs) | 8/10 (~40 attrs) |
| License risk | HIGH (GPL-3) | LOW (Apache-2) |
| Protagonist as ordinary player | 9/10 | N/A |
