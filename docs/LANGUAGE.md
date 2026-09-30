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
* Money is rupees (`₹2.5 crore`, `Rs 75 lakh`); dates are resolved against `Request::now`.
* Options (inbox) carry an `effect` id. **The simulation must implement each effect**; the consequence line only describes it. Current ids: `transfer.open_bid`, `transfer.drop_target`, `medical.request_report`, `callup.accept`, `academy.accept/decline`, `university.accept/decline`, `inbox.dismiss`.

## Data (`data/lang/en/`)

`events.toml` (facts, `hidden` ones), `channels.toml` (slots, style, limits, forbidden marks), `articles.toml` (slot:topic order per event), `certainty.toml` (wrappers per certainty and the `[[source]]` table), `lexicon/*.toml` (concepts, variants, register, `when`/`requires`/`not_certainty`), `collocations.toml`, `frames/*.toml` (`clause` frames are wrapped by certainty; `full` frames carry their own), `options.toml`, `voices.toml` (13 voice dimensions, outlet profiles), `corpus/*.toml` (test states). Add words and frames by editing TOML only.

Template syntax is documented at the top of `crates/pw-lang/src/template.rs`.

## Checks

`cargo test -p pw-lang` runs: data lint (unknown events, facts, concepts, missing forms, facts used but not required), the corpus (`cargo run -p pw-lang --example show [prefix]` prints its outputs), and a fuzz over every event × channel × voice × random knowledge state that fails on placeholders, doubled words, punctuation, a/an, missing subjects, empty clauses, repeated fragments, duplicated names, date/tense contradictions, firm wording on weak certainty (and hedges on facts), channel limits and any leaked fact.

## In the simulation (`pw-narrate/src/lang.rs`)

The bridge is the only place the world is read for the engine. It is used **only in worlds with an ecosystem** (India): the engine's money is
rupees and its examples are Indian football, so other worlds keep the older templates in `pw-narrate`. Where the engine has no event for a story,
or reports the article incomplete, the older text is used; nothing is ever half written.

| World thing | Engine event | Where it shows |
| --- | --- | --- |
| transfer (news) | `transfer.completed` | press headline and body |
| transfer rumour, claim below 75 | `transfer.interest` (stage watching / keen / preparing) | press; never written as a bid |
| transfer rumour, claim 75 or more | `transfer.bid_made` | press |
| injury story | `injury.suffered` (player and club only) | press |
| manager sacked / appointed | `manager.departed` / `manager.appointed` | press |
| promotion / relegation, renewal | `competition.*`, `contract.renewed` | press |
| match report | `match.result` (with the standout player when the story has one) | press |
| interview | `interview.quote` (stance only) | press |
| milestone, record | `milestone.reached`, `record.broken`, `transfer.record` | press |
| unhappy, praise, award stories | `player.unhappy`, `player.praise`, `award.won` | press |
| a post relaying a story | the story's event, channel `social`, voice from the account | social |
| trial invitation, talks about a move | `academy.invitation`, `transfer.bid_made` | inbox subject and message; the options stay the simulation's |

Rules the bridge keeps (tested in `crates/pw-cli/tests/lang_bridge.rs`):

* **No firmer than the story**: a rumour is written at the stage its claim reached (never as a bid or a deal); a relayed post is no firmer than the story or its own claim.
* **Only what the public may know**: a published injury story has no diagnosis and no time out (the club's business), a renewal no length, and nothing from private negotiations.
* **As it was**: ages and clubs are those on the date of the story, so an old story does not change as the world ages.
* **Clean**: every written text passes `pw_lang::check::check_text`.
* **Pure**: the same story reads the same way every time (the seed is the story id). Headline and body share one render.

Measured on the tiny India world over 500 days: 90% of press stories are written by the engine (the rest are incident reports, analysis and features, discipline and
fan reaction, which have no event yet); about 0.24 ms per story.

To add coverage: add the event to `events.toml`, frames to `frames/*.toml`, an entry in `articles.toml`, then map the world story in `lang::story_event`. `cargo test -p pw-lang` lints the data and fuzzes it.
