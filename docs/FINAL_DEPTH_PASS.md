> **Historical report.** Written for branch `claude/kind-planck-t66j8f` (commit `401b7ae`). It may not describe the current code; see `IMPLEMENTATION_STATUS.md` and `docs/README.md`.

# Final depth pass

Written 2026-09-28, **not compiled yet**. After this pass, features are frozen. The next work is
compiling, testing, fixing and validating (`docs/INTEGRATION_REPORT.md`).

This document maps each item of the final brief to the code that implements it. It also contains the
decision-smartness audit, the review of single "magic" scores, and the data boundary.

| # | Item | Where | State |
|---|---|---|---|
| 1 | Contextual trust and reputation by topic, organisation and audience; belief is not fact | `socialnet::believes` (outlet record per account, credulity, knowledge, desirability, corroboration, claim type); `Grapevine` knowers' confidence and version; `newsroom::source_reliability`; the media's per-club fan standing | written |
| 2 | Replies and threads with memory; decay limits | `socialnet::threads` (reply, disagree, quote, call-outs citing a real post from the last 60 days); depth 3 with halving probability; four posts a day per account; mutes; opinions remember their highs and lows | written |
| 3 | Feed and attention | `socialnet::feed` (follows, club, self, teammates, rival, real people, trends, engagement, recency) | written |
| 4 | Human social interaction entering the same systems | `Intent::Post` → `person_post`; inbox replies → intents (`inbox::reply`); `Intent::Tell`, `Intent::Thank` | written |
| 5 | Football grammar and vocabulary | `pw-narrate/src/grammar.rs` (`Term`, `style`, `style_complaint`, `score`) | written |
| 6 | Registers, slang and banter by age, region, personality and platform, without stereotyping | `lexicon::{Register, AgeBand, Dialect (lexical only), Platform}`; `grammar::banter` depends on humour and hostility | written |
| 7 | Era-evolving language | `lexicon::Era`; era-dated slang in `en()`; era-gated terms (xG, positional play) in `grammar::term` | written |
| 8 | Memes from context | `socialnet::memes` (derby red cards, late winners, viral posts; reuse, variants, fading) | written |
| 9 | Original procedural chants | `socialnet::chants` + `social::chant` (shapes and seeds; no lyrics) | written |
| 10 | Supporter groups and their actions | `SupporterGroup`, `socialnet::weekly/act` (protest, petition, banner, applause) | written |
| 11 | Stadium atmosphere with bounded effects | `officials::pre_match` (at most +3 / −3 confidence, once per match; chants sung) | written |
| 12 | Persistent referees; perceived bias without corruption | `officials` (referees are persons; correctness depends only on accuracy; grievances form perceived bias) | written |
| 13 | Discipline, appeals, investigations | red-card appeals (a human decides), panels, frivolous extensions, misconduct charges; incident investigations from the incidents pass | written |
| 14 | Evolving tactical meta and schools | `evolution` (schools founded by distinctive repeat champions; lineage through players and coaches; national fashion pulled toward champions) | written |
| 15 | Rare, institution-caused rule evolution, versioned by season | `evolution::rules` (four rules, measured pressure, conservatism, a six-year cooldown); `Evolution::value_in` reconstructs any past season | written |
| 16 | Emergent rivalries and club culture feedback | culture pass (rivalries from title races, promotion, finals, transfers and manager moves); backfill title races; supporter groups → fan mood → board; atmosphere from tribalism | written |
| 17 | Procedural historical backfill, generated vs imported | `backfill` + optional `history.csv`; provenance on every season and figure | written |
| 18 | History at every level | minor football, the record book, votes, halls, the chronicle (see `MEDIA_SOCIAL_HISTORY_SYSTEMS.md` §13) | written |
| 19 | Inbox and press threads where replies create real intents | `inbox` | written |
| 20 | Text quality checks and semantic truth guards | `pw-narrate/src/quality.rs`, `pw-sim/src/audit.rs` | written (the tests that run them come with compilation) |
| 21 | Decision-smartness audit | below | done (on paper) |
| 22 | No single magic scores | below | reviewed |
| 23 | Data-pack boundary | `docs/DATA_PACK_BOUNDARY.md` | done |
| 24 | Freeze and commit separately | commits 92fdf17…(this pass) | frozen after this pass |

---

## Decision-smartness audit (21)

For each autonomous decision this pass added, the table below lists what it reads, whether it can see
something it should not, and how it could look foolish.

| Decision | Inputs | Knowledge boundary | Weak spots, and the mitigation or follow-up |
|---|---|---|---|
| Tell or keep a secret (`grapevine::inclination`) | role, kind of information, sensitivity, closeness, trust, professionalism, controversy, loyalty to the subject, being asked to keep quiet | only items the teller knows, in their own version | Everyone in a role is treated alike. Personality comes in only through hidden attributes. |
| Respond to an incident (`responses::decide`) | authority, discipline tradition, temperament, trust in those involved, severity, exposure | only incidents they have learned of | Precedent (how similar cases were handled) is not consulted. A candidate for a later pass. |
| Answer a press question (`pressroom::ai_stance`) | media style, temperament, trust in the subject, the truth of the story, whether the player wants to leave | the manager knows the truth about their own squad | A leaking manager who denies what they leaked is not modelled. |
| Appeal a red card (`officials::consider_appeal`) | what the manager saw (the truth with ±0.2 noise), the player's importance | no hidden state beyond a noisy view | Does not consider the fixture list: a ban before a derby is weighed like any other. |
| Criticise the referee (`officials::referee_comments`) | media style, temperament, grievance | public facts only | Ignores the fine they will pay. Charges happen, but the manager does not learn from them. |
| Journalist's desk (`newsroom::run`) | newsworthiness components, verification from real sources, outlet style, risk | only items the journalist learned | Editors do not weigh libel risk against a subject's power. |
| Supporter posting (`socialnet::concept`) | frame valence, prior opinion, lowest opinion, memories, persona | only public frames and the stories they read | Posts react to one frame at a time. A season-long narrative ("third derby loss this year") needs memory queries. Follow-up. |
| Vote (`awards::run`) | per-voter lens over components, familiarity, eligibility rules, personal noise | public performances and honours | Journalists weight fame the same across outlets. Outlet style could shape the lens. |
| Hire a manager's philosophy (`evolution::on_appointed`) | formative school, prestige | career history | Boards do not yet ask for a school explicitly. They get one through fashion. |
| Change a rule (`evolution::rules`) | measured pressures, conservatism, cooldown | aggregate statistics | Only four rules. The pressures are simple ratios. |

Across all of these, no AI decision reads a human's future choices or anything the decider could not
know. Humans receive the same decisions, with the AI's choice as the default.

## Single scores (22)

These are the single numbers that still exist, and why each is acceptable or how it is constrained:

- **`honours::career_score`** makes a player eligible for the world hall. Election is a committee
  vote over components (trophies, peak, fame, goals, longevity, international career), so no single
  number decides induction.
- **`newsroom::newsworthiness`** is a single gate on whether a story runs. Its three components
  (importance, relevance, controversy) are recorded in the story's cause (`Fact::Newsworthy`), so the
  reasons remain visible.
- **`socialnet::believes`** is a probability of belief, not a quality score, and is built from six
  named factors.
- **The incident hazard** is a rate made from named pressures, each recorded as a cause
  (`Fact::Pressure`).
- **Minor-football team strength** is a result model, not a judgement of people.
- **Vote results** are totals of ballots, and each ballot keeps its reason for every pick.

Nothing in this pass chooses a person by a single opaque score. The remaining single thresholds are
tuning constants that the validation phase will measure.

## Data boundary (23)

See `docs/DATA_PACK_BOUNDARY.md`. No lyrics, no proprietary game data, no club artwork, and no
unlicensed history ship in this repository. Generated history never names real people.

## Freeze (24)

Feature work stops here. The compile, test and validation phase may change code only to make it
correct, fast and faithful to these designs. It may not disable a system, and it may not replace logic
with placeholders.
