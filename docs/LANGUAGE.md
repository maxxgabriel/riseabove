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
