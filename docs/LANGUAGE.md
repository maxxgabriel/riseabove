# Language system (pw-lang)

One semantic event feeds every text channel. The pipeline is: **event** (typed facts) → **speaker knowledge** (which facts, how sure, from which source) → **certainty** → **channel grammar** (slots and article structure) → **frames and lexicon** (chosen for voice, damped by recent use) → **checked text**. Nothing is written that the caller did not supply, and nothing is said more firmly than the speaker holds it.

## Calling it

```rust
let eng = pw_lang::Engine::builtin();               // loads data/lang/en/**/*.toml embedded at build time
let ev = Event::new("transfer.bid_rejected", date).ent("buyer", club_ref).ent("seller", ..).ent("player", ..).money("fee", rupees);
let mut sp = eng.witness("s1", "journalist", "sports_desk", &ev);   // knows every non-hidden fact first hand
sp.knows.insert("fee".into(), Know::of(Certainty::SourceClaim, "unnamed"));   // ...but the fee is only claimed
let r = eng.render(&Request::new(&ev, &sp, "news", today, seed), &mut tracker);   // r.parts, r.options, r.notes
```

* `Ref` carries canonical names and only **true** descriptors from world state (`age_role`, `role`, `title`, ...).
* `Speaker::knows` is the whole leak boundary: a fact not listed cannot be mentioned; `Unknown` lets a text say that something is not known, without showing it.
* Money takes the form `Request::currency` gives it: `Currency::Rupee` (the default: `₹2.5 crore`, `Rs 75 lakh`) or `Currency::Short(sym)` (`£2.5m`, `€450k`); dates are resolved against `Request::now`.
* Options (inbox) carry an `effect` id. **The simulation must implement each effect**; the consequence line only describes it. With a decision, an option is shown only when its effect is one of the decision's own choices (`pw_narrate::lang::effect_choice`): `decision.accept/reject`, `academy.accept/decline`, `university.accept/decline` and `callup.accept` map to accept and reject; the answer is then applied by the decision exactly as before. Other ids (`transfer.open_bid`, `transfer.drop_target`, `medical.request_report`, `inbox.dismiss`) are not offered with a decision yet.

## Data (`data/lang/en/`)

`events.toml` (facts, `hidden` ones), `channels.toml` (slots, style, limits, forbidden marks), `articles.toml` (slot:topic order per event), `certainty.toml` (wrappers per certainty and the `[[source]]` table), `lexicon/*.toml` (concepts, variants, register, `when`/`requires`/`not_certainty`), `collocations.toml`, `frames/*.toml` (`clause` frames are wrapped by certainty; `full` frames carry their own), `options.toml`, `voices.toml` (13 voice dimensions, outlet profiles), `corpus/*.toml` (test states). Add words and frames by editing TOML only. A frame's optional `weight` is added to its fit: a frame that can say something the others cannot
(a derby's name) is preferred whenever its facts are there; the repetition penalty still rotates the wording.

Template syntax is documented at the top of `crates/pw-lang/src/template.rs`.

## Checks

`cargo test -p pw-lang` runs: data lint (unknown events, facts, concepts, missing forms, facts used but not required), the corpus (`cargo run -p pw-lang --example show [prefix]` prints its outputs), and a fuzz over every event × channel × voice × random knowledge state that fails on placeholders, doubled words, punctuation, a/an, missing subjects, empty clauses, repeated fragments, duplicated names, date/tense contradictions, firm wording on weak certainty (and hedges on facts), channel limits and any leaked fact.

## In the simulation (`pw-narrate/src/lang.rs`)

The bridge is the only place the world is read for the engine. It writes for **every world**: its words are football's with no country in them,
and money takes the world's form (`lang::currency`: the Indian system in a world of Indian regions, `£` and short units elsewhere, the symbol
the pages use by default). Where the engine has no event for a story, or reports the article incomplete, the older text is used; nothing is
ever half written.

| World thing | Engine event | Where it shows |
| --- | --- | --- |
| transfer (news) | `transfer.completed` | press headline and body |
| transfer rumour, claim below 75 | `transfer.interest` (stage watching / keen / preparing) | press; never written as a bid |
| transfer rumour, claim 75 or more | `transfer.bid_made` | press |
| injury story | `injury.suffered` (player and club only) | press |
| manager sacked / appointed | `manager.departed` / `manager.appointed` | press |
| promotion / relegation, renewal | `competition.*`, `contract.renewed` | press |
| match report | `match.result` (with the standout player when the story has one, and `occasion` when the reference names the meeting a derby) | press |
| interview | `interview.quote` (stance only) | press |
| milestone, record | `milestone.reached`, `record.broken`, `transfer.record` | press |
| unhappy, praise, award stories | `player.unhappy`, `player.praise`, `award.won` | press |
| a post relaying a story | the story's event, channel `social`, voice from the account | social |
| feature or data piece on a player | `player.reading` (the media label: in form, underrated, breakthrough ...) | press |
| analysis the morning after a match | `match.analysis` | press |
| incident made public (18 kinds; private matters keep the older text) | `incident.reported` | press |
| pressure on a manager, discipline, criticism | `manager.pressure` (speculation), `player.discipline`, `player.criticism` | press |
| supporters' reaction | `fans.reaction` (angry / delighted / divided, from the story's tone) | press |
| trial invitation, talks about a move, university scholarship | `academy.invitation`, `transfer.bid_made`, `university.scholarship` | inbox subject, message, and options labelled with what each does, each tied to one of the decision's own choices |

Rules the bridge keeps (tested in `crates/pw-cli/tests/lang_bridge.rs`):

* **No firmer than the story**: a rumour is written at the stage its claim reached (never as a bid or a deal); a relayed post is no firmer than the story or its own claim.
* **Only what the public may know**: a published injury story has no diagnosis and no time out (the club's business), a renewal no length, and nothing from private negotiations.
* **As it was**: ages and clubs are those on the date of the story, so an old story does not change as the world ages.
* **Clean**: every written text passes `pw_lang::check::check_text`.
* **Pure**: the same story reads the same way every time (the seed is the story id). Headline and body share one render.

Measured on the tiny India world over 500 days: all but 87 stories are written by the engine. The rest are season wrap-ups and manager changes
(they rest on the event log, which forgets), denials and private matters.

To add coverage: add the event to `events.toml`, frames to `frames/*.toml`, an entry in `articles.toml`, then map the world story in `lang::story_event`. `cargo test -p pw-lang` lints the data and fuzzes it.
