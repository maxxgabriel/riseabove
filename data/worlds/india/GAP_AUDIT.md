# India data gap audit (2026-09-29)

Scope: `crates/pw-import/src/india.rs` and `data/worlds/india/pack.toml` on `sim/expansion-audit`.
Status after this pass: only West Bengal state leagues were source-researched (quality B). Everything else
below was written from general knowledge (quality C/D, status inferred/unknown) after research was stopped
on request. It is a baseline to verify, not verified fact.

| Area | What the code does today | Class | State after this pass |
|---|---|---|---|
| Media outlets | `ensure_media` generates "{code} Sport", "The IND Sun", "{city} Evening Post", "{club} Fan Channel" | HIGH | 51 real outlets in `media/outlets.toml` (not yet read by the builder) |
| Broadcasters | none; "IND TV Football" generated | HIGH | 5 platform identities, no rights (unknown) |
| Brands/sponsors | `commerce::ensure` generates "{Surname} Bank/Motors" | MEDIUM | untouched; needs research |
| State associations | 31 names in pack, several likely wrong (e.g. Maharashtra) | HIGH | 32 named, flagged where pack differed; statuses unknown |
| Districts | 2-6 per state in pack | MEDIUM | pack districts only; full lists missing |
| National competitions | 4 tiers named ISL/IFL/IL2/IL3 in code, cup called Super Cup; sizes 13/12/16/20 hard-coded | HIGH | 26 competitions with unverified formats |
| Clubs | 44 in pack; rest generated ("Kohima United", "Aizawl Sporting") to fill tiers | HIGH | same 44 + 3, identity only; fills still generated |
| State leagues | all generated ("{district} FC") | HIGH | West Bengal researched; rest missing |
| Universities | 33 in pack; `founded`, prestige random | MEDIUM | 33 identities; AIU universities missing |
| Schools | all generated ("Malappuram Model School") | MEDIUM | none |
| Academies | 5 independent academies in pack | HIGH | 9 identities |
| Stadiums | none; capacity = rep*4+1500 | MEDIUM | none |
| Partnerships, rules, referee grades | none | HIGH | empty on purpose |
| Foreign clubs | "ESP Athletic Norte" etc. | LOW (fictional overseas clubs acceptable, but should not resemble real names) | untouched |
| Player names/people | generated pools | LOW | untouched (generation is correct here) |

Schema additions needed for the builder to consume this data: none for identity. Provenance fields
(`prov`) have no counterpart in `World`; loaders should read and drop them, keeping them for the debug view.
