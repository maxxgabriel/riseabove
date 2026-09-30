#!/usr/bin/env python3
"""Data-level validation for data/worlds/india/. Exit 1 on errors. Run from repo root."""
import tomllib, pathlib, sys, collections

ROOT = pathlib.Path("data/worlds/india")
TABLES = ["state", "district", "association", "competition", "club", "stadium", "academy", "university", "school", "outlet", "broadcaster",
          "programme", "partnership", "rule", "team", "licence", "grade", "rivalry", "language", "term", "ownership", "membership", "rights", "sponsorship", "alias"]
STATUS = {"imported", "verified", "inferred", "scenario_seed", "generated", "unknown"}
errors, warns = [], []
ids = collections.defaultdict(list)
recs = []
for f in sorted(ROOT.rglob("*.toml")):
    if f.name == "pack.toml" or f.parent.name == "_manifest":
        continue
    try:
        d = tomllib.loads(f.read_text())
    except Exception as e:
        errors.append(f"{f}: parse error {e}"); continue
    if "meta" not in d:
        errors.append(f"{f}: missing [meta]")
    for t in TABLES:
        for r in d.get(t, []):
            recs.append((f, t, r))
            if "id" in r:
                ids[r["id"]].append(f)
            p = r.get("prov")
            if not p:
                errors.append(f"{f}: {t} {r.get('id', r.get('club'))} has no prov")
            elif p.get("status") not in STATUS or p.get("q") not in list("ABCDE"):
                errors.append(f"{f}: {r.get('id')} bad prov {p}")
for i, fs in ids.items():
    if len(fs) > 1:
        errors.append(f"duplicate id {i} in {sorted(set(map(str, fs)))}")
by = lambda p: {i for i in ids if i.startswith(p)}
ST, AS, CO, CL, LG, SD, AC = by("state."), by("assoc."), by("comp."), by("club."), by("lang."), by("stadium."), by("academy.")
def chk(f, what, ref, pool):
    if ref and ref not in pool:
        errors.append(f"{f}: {what} -> unknown {ref}")
for f, t, r in recs:
    for k in ("state", "home_state"):
        if isinstance(r.get(k), str): chk(f, f"{t}.{k}", r[k], ST)
    for k in ("parent",):
        if t == "association": chk(f, "association.parent", r.get(k), AS)
    if t == "competition":
        chk(f, "competition.organiser", r.get("organiser"), AS)
        for c in r.get("qualifies_to", []) + r.get("fed_by", []): chk(f, "competition link", c, CO)
    if t == "membership":
        chk(f, "membership.competition", r["competition"], CO)
        if r["club"].startswith("club."): chk(f, "membership.club", r["club"], CL)
        elif r["club"].startswith("assoc."): chk(f, "membership.assoc", r["club"], AS)
    if t == "club" and r.get("home_ground"): chk(f, "club.home_ground", r["home_ground"], SD)
    if t == "academy" and r.get("parent"): chk(f, "academy.parent", r["parent"], CL)
    if t in ("outlet", "broadcaster", "state"):
        for l in r.get("languages", []): chk(f, f"{t}.language", l, LG)
    if t == "rivalry":
        chk(f, "rivalry.a", r["a"], CL | ST); chk(f, "rivalry.b", r["b"], CL | ST)
    if t == "team": chk(f, "team.association", r.get("association"), AS)
    if t == "rule" and r.get("competition"): chk(f, "rule.competition", r["competition"], CO)
    if t == "ownership": chk(f, "ownership.club", r["club"], CL)
    if t == "district": chk(f, "district.state", r["state"], ST)
    if t == "licence" and r.get("prerequisite"): chk(f, "licence.prerequisite", r["prerequisite"], by("licence."))
    for k in ("effective_from", "effective_to"):
        pass
    if r.get("effective_from") and r.get("effective_to") and str(r["effective_from"]) > str(r["effective_to"]):
        errors.append(f"{f}: {r.get('id')} effective_from after effective_to")
    if t in ("outlet",) and any(k in r for k in ("credibility", "trust", "score")):
        errors.append(f"{f}: outlet {r['id']} carries a score field")
    if t in ("stadium",) and "capacity" in r and not r.get("capacity_as_of"):
        errors.append(f"{f}: stadium {r['id']} capacity without capacity_as_of")
# [[alias]]: every alias points at existing entities and (entity, alias) is unique
seen_alias = set()
for f, t, r in recs:
    if t != "alias":
        continue
    for e in ([r["entity"]] if "entity" in r else []) + list(r.get("entity_pair", [])):
        if e not in ids:
            errors.append(f"{f}: alias {r.get('id')} -> unknown entity {e}")
    if "entity" not in r and "entity_pair" not in r:
        errors.append(f"{f}: alias {r.get('id')} has neither entity nor entity_pair")
    key = (r.get("entity") or tuple(r.get("entity_pair", [])), r.get("alias", "").lower(), r.get("lang"))
    if key in seen_alias:
        errors.append(f"{f}: alias {r.get('id')} duplicates {key}")
    seen_alias.add(key)
# aliases must not collide across different entities of the same kind
seen = collections.defaultdict(set)
for f, t, r in recs:
    if t in ("club", "competition", "outlet", "association", "university"):
        for n in [r.get("name"), r.get("short")] + list(r.get("aliases", [])):
            if n: seen[(t, n.lower())].add(r["id"])
for (t, n), s in seen.items():
    if len(s) > 1: warns.append(f"alias/name '{n}' shared by {sorted(s)}")
# every state has an association or is flagged
st_assoc = {r.get("state") for f, t, r in recs if t == "association" and r.get("kind") == "state"}
for s in sorted(ST - st_assoc): warns.append(f"{s} has no state association record")
for w in warns: print("WARN ", w)
for e in errors: print("ERROR", e)
print(f"{len(recs)} records, {len(ids)} ids, {len(errors)} errors, {len(warns)} warnings")
sys.exit(1 if errors else 0)
