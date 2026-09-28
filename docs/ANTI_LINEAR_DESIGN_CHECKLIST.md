# Anti-Linear Design Checklist

Use this on every design proposal and every PR that touches simulation, career or client code. The
aim is to catch drift toward a scripted click-through career game early. Rule IDs (S1…S27) refer to
[`SYSTEMIC_SIMULATION_RULES.md`](SYSTEMIC_SIMULATION_RULES.md).

---

## 1. The four questions (answer in writing for every feature)

| # | Question | If the answer is wrong |
|---|---|---|
| Q1 | Would this system still work with **no human player** in the world? | If not, it is a protagonist feature disguised as a system. Redesign (S1). |
| Q2 | Would this happen to an **AI-controlled person** in the same circumstances? | If not, the human is getting special treatment (S2, S5). |
| Q3 | Is it caused by **persistent world state**, or invented to make something interesting happen? | If invented, redesign so that state causes it (S11, S17). |
| Q4 | Could the situation end in **several plausible ways** depending on the people and circumstances involved? | If not, it is scripted (S6, S7). |

## 2. Automated checks (should run in CI)

| Check | How | Rule |
|---|---|---|
| Mind reads are allowlisted | `grep -rn "MindKind::External\|\.mind\b\|is_external"` in `pw-world`, `pw-sim`, `pw-match` must match only the allowlisted sites (decision routing, recording level of detail) | S2 |
| No career dependency in world crates | `cargo tree` for `pw-world`, `pw-sim` and `pw-match` must not contain `pw-career` or client crates | S25 |
| Headless autonomy | 20 or more seasons with zero External minds, several seeds; world invariants hold (checklist §3) | S1 |
| Swap test | Take control of player P, auto-answer every request with P's own AI default. The resulting world hash must equal a run where P stayed AI. | S2, S4, S20 |
| Takeover does not change the world | Hash the world before and after `take_control`; only `mind` differs | S4, S20 |
| Every rendered text has a source | Each inbox item, article, post and conversation line carries an event or state reference; a test renders a season's feed and asserts none are missing | S17 |
| Every event has causes | Events of consequential kinds (transfers, dropped from squad, fines, sackings, conflicts) have a non-empty `causes` list | S11 |
| Option parity | For a sample of decision points, the option set given to an External mind equals the one the AI evaluated for that person | S5, S6 |
| No stage or age triggers | Review grep for `age ==`, `age >=`, `stage`, `chapter`, `milestone` in simulation code; each hit must be a data rule or a consideration input | S8, S9 |
| Playthrough variance | Same world, two playthrough seeds, same scripted human policy: careers diverge. Same seed twice: identical. | S22 |
| No game-over | No code path in engine or client ends the session because a person retired or died | S10 |

## 3. World health invariants (autonomy benchmark)

Checked every season of a no-human run:
- Every fixture is played with legal squads.
- The free-agent pool, age distribution and ability distribution per tier stay stable (no power creep
  or decay above 5% over 20 years).
- Managers are hired and sacked, staff pools refill, and retired players enter staff jobs.
- Transfers, loans, renewals and releases fall within the calibration bands (`plan/14_QA_BALANCE.md`).
- Title diversity and club rise and fall stay within bands.
- Relationships, rumours, conversations and family events occur among AI people at plausible rates.
  If a life or social system only produces output when a human is present, it fails.
- Memory and save size stay bounded (50-season soak).

## 4. Red flags (any one triggers an architecture review)

- A `CareerStage` or `Chapter` enum, or any stage value that gates content, screens or events.
- A scripted sequence of career events, or a "career events" catalogue keyed by milestones.
- An opportunity generator that serves only the controlled person (trials, offers, call-ups, scout
  interest).
- Scout or club interest caused by the human's performance without a scout or club actually observing
  it.
- Guaranteed outcomes: a trial that always happens, a first contract at a fixed age, a transfer after
  N good matches.
- A random popup that does not come from world state ("5% chance per month: partner unhappy").
- News, inbox or social text without a source event.
- Conversation choices with hard-coded consequences (`Argue → trust −10`).
- Family or partner events that exist only as text, with no entity behind them.
- A single reputation number used for every audience.
- A single relationship number with no history of why.
- A hard-coded retirement ending, legacy "the end" screen or "thanks for playing".
- `is_protagonist` (or `mind == External`) influencing an evaluation, probability or outcome.
- Protagonist-only data types that hold world facts (family, money, agent, contract).
- The client writing world state, or reading truth that the person could not know.
- A detailed model for the human and a separate statistical model for AI people (S20).
- Messages on a fixed calendar that are not produced by an entity's behaviour ("every Monday: coach
  report"), unless a real club process produces them for every player.

## 5. PR review template

Paste this into PR descriptions that touch simulation, career or client code:

```
Rise Above systemic review
- Q1 works with no human:            yes / no — why
- Q2 same for AI in same situation:  yes / no — why
- Q3 caused by world state:          which events/state cause it
- Q4 multiple plausible outcomes:    list 2–3
- New considerations added:          ...
- Events emitted (with causes):      ...
- Knowledge/visibility:              who can learn about this, through which channel
- Text sources:                      which events/state each new text renders
- Red flags checked (§4):            none / list with justification
- Headless metric added/updated:     ...
```

## 6. Example review verdicts

| Proposal | Verdict |
|---|---|
| "Send the player an agent report on the 1st of every month." | **Reject as stated.** Rework it: agents are people with workload and diligence. Each agent decides when to contact each client, based on news, interest and their relationship. Clients of AI agents get the same contacts, which feed those players' decisions. |
| "When the human scores a hat-trick, a big club shows interest." | **Reject.** Interest requires a club with a need whose scout or analyst actually observed the player, or a public signal every club receives equally (reputation from media coverage). |
| "At 17, show the pro-contract negotiation screen." | **Reject.** Clubs offer first professional deals through squad planning and national rules. The screen appears when a real offer exists. |
| "Manager calls player in when training drops for 3 weeks, Discipline ≥ 14, trust low." | **Accept as a consideration-driven decision**, provided the same check runs for every squad member, the manager can also choose not to act, and the outcome depends on both people. |
| "Retired: show legacy summary and 'Start new career'." | **Accept only as a UI option.** The person persists, the world continues, and the user can keep inhabiting them (coach, pundit, private life) or switch. |
