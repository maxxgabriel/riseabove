# ADR 0002: Imported data keeps provenance and missing stays missing

**Status:** accepted (locked design §11). **Implemented:** `pw-import` (`model.rs`, `resolve.rs`, `assemble.rs`, `infer.rs`), `pw-world::origin`.

**Decision.** Sources parse into one validated representation (`ImportSet`); identities and foreign keys are resolved by evidence;
rows that cannot be placed are reported and left out. Each fact group of each imported person is labelled imported, inferred,
generated or unknown and saved with the world. Ability is estimated from several kinds of evidence and *sampled* from that
estimate; market value is one weak input and never the oracle. Personality is never inferred from price.

**Why.** A price decides ability only by turning one noisy proxy into a self-confirming loop (price → ability → performance →
price). Guessing club links or merging people by name silently corrupts the world.

**Consequences.** Adapters change; the assembler and the labels do not. Any new inference needs a calibration check by cohort
(`real_archive.rs`) and a provenance label. The world's price curve and the importer's reading of prices are the same formula
(tested).
