#!/usr/bin/env python3
"""Second general-knowledge pass (thread A): remaining union-territory associations, national and
state team structures, rule placeholders, and more universities. Same conventions as
build_india_reference.py: everything is prov "inferred"/"unknown", quality C or D, written from
general knowledge and NOT checked against a source. Run from the repo root."""
import json, pathlib, tomllib

ROOT = pathlib.Path("data/worlds/india")
GK = "general knowledge; not verified against a source"


def val(v):
    if isinstance(v, bool): return "true" if v else "false"
    if isinstance(v, (int, float)): return str(v)
    if isinstance(v, list): return "[" + ", ".join(val(x) for x in v) + "]"
    if isinstance(v, dict): return "{ " + ", ".join(f"{k} = {val(x)}" for k, x in v.items()) + " }"
    return json.dumps(v, ensure_ascii=False)


def prov(status="inferred", q="C", note=GK):
    return {"status": status, "q": q, "src": [], "note": note}


def write(path, meta, tables):
    out = ["[meta]"] + [f"{k} = {val(v)}" for k, v in meta.items()] + [""]
    for name, rows in tables:
        for r in rows:
            out.append(f"[[{name}]]")
            out += [f"{k} = {val(v)}" for k, v in r.items()]
            out.append("")
    (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
    (ROOT / path).write_text("\n".join(out))


def meta(ds, q, cov, gaps, season=None):
    m = {"dataset": ds, "quality": q, "effective": "2026-07-01"}
    if season: m["season"] = season
    m.update({"last_checked": "2026-09-30", "coverage": cov, "gaps": gaps})
    return m


# -------------------------------------------------- associations for the three remaining UTs
write("associations/associations_a.toml", meta("associations.ut_remaining", "D", "Union territories that had no association record.",
      ["Whether these bodies exist as AIFF members, their exact names, and their status are all unverified. Dadra & Nagar Haveli and Daman & Diu may have two separate football bodies. Records are placeholders so every UT resolves to a body; replace after research."]),
      [("association", [
          {"id": "assoc.anifa", "name": "Andaman and Nicobar Islands Football Association", "kind": "state", "state": "state.an", "parent": "assoc.aiff", "status": "unknown", "prov": prov("unknown", "D", "Name and membership unverified.")},
          {"id": "assoc.lfa-ld", "name": "Lakshadweep Football Association", "kind": "state", "state": "state.ld", "parent": "assoc.aiff", "status": "unknown", "prov": prov("unknown", "D", "Name and membership unverified.")},
          {"id": "assoc.dnhdd-fa", "name": "Dadra and Nagar Haveli and Daman and Diu Football Association", "kind": "state", "state": "state.dn", "parent": "assoc.aiff", "status": "unknown", "prov": prov("unknown", "D", "Name and membership unverified; may be two separate bodies.")}])])

# -------------------------------------------------- national and state teams
T = lambda id, name, kind, gender, age, assoc, comps, elig=None: {"id": f"team.{id}", "name": name, "kind": kind, "gender": gender, "age": age, "association": assoc, "competitions": comps,
                                                                    **({"eligibility": elig} if elig else {}), "prov": prov()}
teams = [
    T("india-men", "India men's national team", "national", "men", "senior", "assoc.aiff", ["FIFA World Cup qualifying (AFC)", "AFC Asian Cup and qualifying", "SAFF Championship", "friendlies"], "Indian citizenship plus FIFA eligibility rules; India does not allow dual citizenship in law, which constrains who can be selected."),
    T("india-u23-men", "India men's U23 team", "national", "men", "u23", "assoc.aiff", ["AFC U23 Asian Cup and qualifying", "Asian Games (when held)"], "Age band set by each competition's regulations."),
    T("india-u20-men", "India men's U20 team", "national", "men", "u20", "assoc.aiff", ["AFC U20 Asian Cup and qualifying", "SAFF U20 Championship"]),
    T("india-u17-men", "India men's U17 team", "national", "men", "u17", "assoc.aiff", ["AFC U17 Asian Cup and qualifying", "SAFF U17 Championship"]),
    T("india-women", "India women's national team", "national", "women", "senior", "assoc.aiff", ["AFC Women's Asian Cup and qualifying", "SAFF Women's Championship"]),
    T("india-u20-women", "India women's U20 team", "national", "women", "u20", "assoc.aiff", ["AFC U20 Women's Asian Cup and qualifying"]),
    T("india-u17-women", "India women's U17 team", "national", "women", "u17", "assoc.aiff", ["AFC U17 Women's Asian Cup and qualifying"]),
    T("services", "Services (Indian armed forces team)", "institutional", "men", "senior", "assoc.sscb", ["comp.santosh-trophy"], "Institutional team: personnel of the armed forces, not a geographic association team."),
    T("railways", "Railways (Indian Railways team)", "institutional", "men", "senior", "assoc.rspb", ["comp.santosh-trophy"], "Institutional team: Railways employees, not a geographic association team."),
]
teams.append({"id": "team.state-representative", "name": "State association representative teams", "kind": "state_representative", "gender": "men", "age": "senior", "association": "assoc.aiff",
              "competitions": ["comp.santosh-trophy", "comp.junior-nfc", "comp.sub-junior-nfc"], "eligibility": "Each state or UT association fields a team. Which registration or residence basis qualifies a player is NOT verified; the simulation's current default is in pack.toml [eligibility] and is a scenario setting.", "prov": prov("scenario_seed", "D", "Structure from general knowledge; eligibility left as scenario setting.")})
write("national_teams/teams_a.toml", meta("national_teams", "C", f"{len(teams)} team structures. No squads on purpose.",
      ["Age cut-offs per tournament cycle, U15/U16 programmes, women's youth structure detail, and the actual 2025-26 Santosh Trophy participants are not recorded."]), [("team", teams)])

# -------------------------------------------------- rules: existence recorded, numbers unknown
def R(id, topic, statement, comp=None):
    r = {"id": f"rule.{id}", "topic": topic, "statement": statement}
    if comp: r["competition"] = comp
    r["prov"] = prov("unknown", "D", "Such a rule is believed to exist; its numbers and effective dates were NOT verified. Do not use the parameters until researched.")
    return r
rules = [
    R("isl-foreign-players", "foreign_players", "The ISL limits foreign players in the squad and on the pitch; the numbers for 2026-27 are unknown.", "comp.isl"),
    R("ifl-foreign-players", "foreign_players", "The Indian Football League limits foreign players; numbers for 2026-27 are unknown.", "comp.ifl"),
    R("isl-young-players", "u23", "Top-flight clubs must field or register young domestic players; the exact requirement is unknown.", "comp.isl"),
    R("isl-squad-size", "squad_size", "Squad registration limits exist per league; numbers unknown."),
    R("transfer-windows", "transfer_window", "AIFF sets summer and winter transfer windows each season; 2026-27 dates unknown."),
    R("club-licensing", "licensing", "Clubs need AIFF/AFC club licences to enter the top tiers; criteria and current requirements unknown."),
    R("santosh-eligibility", "state_eligibility", "Santosh Trophy has eligibility restrictions on who may represent an association; the rules are unknown.", "comp.santosh-trophy"),
    R("promotion-relegation", "promotion", "Whether the ISL and Indian Football League are linked by promotion and relegation for 2026-27 is unknown.", "comp.isl"),
]
write("rules/registration_a.toml", meta("rules.placeholders", "D", f"{len(rules)} rules recorded only as existing, with no parameters.", ["All numeric rule parameters and effective dates."], "2026-27"), [("rule", rules)])

# -------------------------------------------------- universities
existing = set()
for f in ROOT.rglob("universities*.toml"):
    for r in tomllib.loads(f.read_text()).get("university", []): existing.add(r["id"])
U = [("Punjabi University", "Patiala", "PB"), ("Bharathiar University", "Coimbatore", "TN"), ("Anna University", "Chennai", "TN"), ("Visva-Bharati", "Santiniketan", "WB"),
     ("University of Burdwan", "Bardhaman", "WB"), ("University of Kalyani", "Kalyani", "WB"), ("Vidyasagar University", "Midnapore", "WB"), ("Sri Venkateswara University", "Tirupati", "AP"),
     ("Andhra University", "Visakhapatnam", "AP"), ("Mangalore University", "Mangalore", "KA"), ("Kuvempu University", "Shivamogga", "KA"), ("Bangalore University", "Bengaluru", "KA"),
     ("Cochin University of Science and Technology", "Kochi", "KL"), ("Rashtrasant Tukadoji Maharaj Nagpur University", "Nagpur", "MH"), ("Jamia Millia Islamia", "New Delhi", "DL"),
     ("Banaras Hindu University", "Varanasi", "UP"), ("University of Lucknow", "Lucknow", "UP"), ("Ranchi University", "Ranchi", "JH"), ("Sambalpur University", "Sambalpur", "OD"),
     ("Ravenshaw University", "Cuttack", "OD"), ("Dibrugarh University", "Dibrugarh", "AS"), ("Tezpur University", "Tezpur", "AS"), ("Assam University", "Silchar", "AS"),
     ("Tripura University", "Agartala", "TR"), ("Nagaland University", "Lumami", "NL"), ("Maharshi Dayanand University", "Rohtak", "HR"), ("Kurukshetra University", "Kurukshetra", "HR"),
     ("Himachal Pradesh University", "Shimla", "HP"), ("University of Kashmir", "Srinagar", "JK"), ("University of Jammu", "Jammu", "JK")]
ur = []
for n, c, s in U:
    sl = n.lower().replace("university of ", "").replace(" university", "").replace(" ", "-")
    i = f"uni.{sl}"
    if i in existing or any(x["id"] == i for x in ur): continue
    ur.append({"id": i, "name": n, "city": c, "state": f"state.{s.lower()}", "prov": prov("inferred", "C", "Institution is real (general knowledge). Football participation, zone, type and achievements not recorded.")})
write("universities/universities_a.toml", meta("universities.more", "C", f"{len(ur)} more real universities, identity only.", ["Which of them actually field AIU football teams is unverified.", "Type, zone and facilities not recorded."]), [("university", ur)])
print(len(teams), len(rules), len(ur), "records")
