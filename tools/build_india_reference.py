#!/usr/bin/env python3
"""Generates the general-knowledge baseline files under data/worlds/india/ that no source-verified
worker produced. Everything emitted here is prov status "inferred", quality C: written from general
knowledge without being checked against a source. Refresh by researching and replacing records.
Run from the repo root:  python3 tools/build_india_reference.py
"""
import json, tomllib, pathlib

ROOT = pathlib.Path("data/worlds/india")
GK = "general knowledge; not verified against a source"


def q(v):
    return json.dumps(v, ensure_ascii=False)


def val(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(val(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {val(x)}" for k, x in v.items()) + " }"
    return q(v)


def prov(status="inferred", qual="C", note=None, src=None):
    p = {"status": status, "q": qual, "src": src or []}
    if note is None and not src:
        note = GK
    if note:
        p["note"] = note
    return p


def write(path, meta, tables):
    out = ["[meta]"] + [f"{k} = {val(v)}" for k, v in meta.items()] + [""]
    for name, rows in tables:
        for r in rows:
            out.append(f"[[{name}]]")
            out += [f"{k} = {val(v)}" for k, v in r.items()]
            out.append("")
    p = ROOT / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text("\n".join(out))


def meta(dataset, qual, coverage, gaps, season=None):
    m = {"dataset": dataset, "quality": qual, "effective": "2026-07-01"}
    if season:
        m["season"] = season
    m.update({"last_checked": "2026-09-29", "coverage": coverage, "gaps": gaps})
    return m


pack = tomllib.loads((ROOT / "pack.toml").read_text())
manifest = []


def man(path, researched, coverage, gaps, conf):
    manifest.append({"path": path, "researched": researched, "sources": [], "effective": "2026-07-01",
                     "coverage": coverage, "gaps": gaps, "confidence": conf, "last_checked": "2026-09-29"})


# ---------------------------------------------------------------- languages
LANGS = [("en", "English", "Latin"), ("hi", "Hindi", "Devanagari"), ("bn", "Bengali", "Bengali"), ("ml", "Malayalam", "Malayalam"),
         ("kok", "Konkani", "Devanagari/Latin"), ("pa", "Punjabi", "Gurmukhi"), ("or", "Odia", "Odia"), ("ta", "Tamil", "Tamil"),
         ("te", "Telugu", "Telugu"), ("mr", "Marathi", "Devanagari"), ("as", "Assamese", "Bengali-Assamese"), ("mni", "Manipuri (Meitei)", "Meitei Mayek/Bengali"),
         ("lus", "Mizo", "Latin"), ("kha", "Khasi", "Latin"), ("ne", "Nepali", "Devanagari"), ("kn", "Kannada", "Kannada"),
         ("gu", "Gujarati", "Gujarati"), ("ur", "Urdu", "Perso-Arabic"), ("ks", "Kashmiri", "Perso-Arabic/Devanagari"), ("sat", "Santali", "Ol Chiki"),
         ("grt", "Garo", "Latin"), ("njz", "Nyishi/Arunachal languages", "Latin")]
write("languages/languages.toml", meta("languages", "C", "Languages relevant to Indian football media and regions.", ["Region-level language environment (as opposed to per-person) is only recorded per state in geography/states.toml."]),
      [("language", [{"id": f"lang.{c}", "code": c, "name": n, "script": s, "prov": prov()} for c, n, s in LANGS])])
man("languages/languages.toml", "Language list from general knowledge", "22 languages", ["Not source-checked"], "C")

# ---------------------------------------------------------------- states
ST = {  # key: (kind, capital, [languages])
 "AP": ("state", "Amaravati", ["te"]), "AR": ("state", "Itanagar", ["en"]), "AS": ("state", "Dispur", ["as", "bn"]), "BR": ("state", "Patna", ["hi"]),
 "CG": ("state", "Raipur", ["hi"]), "GA": ("state", "Panaji", ["kok"]), "GJ": ("state", "Gandhinagar", ["gu"]), "HR": ("state", "Chandigarh", ["hi"]),
 "HP": ("state", "Shimla", ["hi"]), "JH": ("state", "Ranchi", ["hi"]), "KA": ("state", "Bengaluru", ["kn"]), "KL": ("state", "Thiruvananthapuram", ["ml"]),
 "MP": ("state", "Bhopal", ["hi"]), "MH": ("state", "Mumbai", ["mr"]), "MN": ("state", "Imphal", ["mni"]), "ML": ("state", "Shillong", ["en", "kha", "grt"]),
 "MZ": ("state", "Aizawl", ["lus"]), "NL": ("state", "Kohima", ["en"]), "OD": ("state", "Bhubaneswar", ["or"]), "PB": ("state", "Chandigarh", ["pa"]),
 "RJ": ("state", "Jaipur", ["hi"]), "SK": ("state", "Gangtok", ["ne", "en"]), "TN": ("state", "Chennai", ["ta"]), "TS": ("state", "Hyderabad", ["te"]),
 "TR": ("state", "Agartala", ["bn"]), "UP": ("state", "Lucknow", ["hi"]), "UK": ("state", "Dehradun", ["hi"]), "WB": ("state", "Kolkata", ["bn"]),
 "AN": ("union_territory", "Port Blair", ["hi", "en"]), "CH": ("union_territory", "Chandigarh", ["hi", "en"]), "DN": ("union_territory", "Daman", ["gu", "hi"]),
 "DL": ("union_territory", "New Delhi", ["hi"]), "JK": ("union_territory", "Srinagar/Jammu", ["ks", "ur", "hi"]), "LA": ("union_territory", "Leh", ["hi", "en"]),
 "LD": ("union_territory", "Kavaratti", ["ml"]), "PY": ("union_territory", "Puducherry", ["ta"]),
}
NAMES = {"AN": "Andaman and Nicobar Islands", "DN": "Dadra and Nagar Haveli and Daman and Diu", "JK": "Jammu and Kashmir", "LA": "Ladakh", "LD": "Lakshadweep"}
packst = {s["key"]: s for s in pack["state"]}
states = []
for k, (kind, cap, langs) in ST.items():
    name = packst[k]["name"] if k in packst else NAMES[k]
    states.append({"id": f"state.{k.lower()}", "key": k, "name": name, "kind": kind, "capital": cap, "languages": [f"lang.{l}" for l in langs], "prov": prov()})
write("geography/states.toml", meta("geography.states", "C", "All 28 states and 8 union territories.", ["Population and official-language claims are unverified; several states have more than one principal language."]), [("state", states)])
dists = []
for s in pack["state"]:
    for d in s["districts"]:
        slug = d.lower().replace(" ", "-").replace("&", "and")
        dists.append({"id": f"district.{s['key'].lower()}.{slug}", "state": f"state.{s['key'].lower()}", "name": d, "prov": prov("scenario_seed", "D", "Real district or city name carried over from the scenario pack; the state's full district list is NOT researched.")})
write("geography/districts.toml", meta("geography.districts", "D", f"{len(dists)} real districts/cities the scenario pack already uses.", ["Full district lists per state not compiled (about 780 nationally). Districts absent here must be researched before use."]), [("district", dists)])
man("geography/*.toml", "States/UTs from general knowledge; districts carried from pack", f"36 states/UTs, {len(dists)} districts", ["Full district list missing"], "C/D")

# ---------------------------------------------------------------- associations
ASSOC = {  # key: (name, abbr, note)
 "KL": ("Kerala Football Association", "KFA"), "WB": ("Indian Football Association", "IFA"), "GA": ("Goa Football Association", "GFA"),
 "OD": ("Football Association of Odisha", "FAO"), "PB": ("Football Association of Punjab", "FAP"), "MN": ("Football Association of Manipur", "FAM"),
 "MZ": ("Mizoram Football Association", "MFA"), "ML": ("Meghalaya Football Association", "MFA-ML"), "KA": ("Karnataka State Football Association", "KSFA"),
 "TN": ("Tamil Nadu Football Association", "TNFA"), "MH": ("Western India Football Association", "WIFA"), "AS": ("Assam Football Association", "AFA"),
 "DL": ("Football Delhi", "FD"), "JH": ("Jharkhand Football Association", "JFA"), "NL": ("Nagaland Football Association", "NFA"),
 "SK": ("Sikkim Football Association", "SFA"), "TR": ("Tripura Football Association", "TFA"), "JK": ("Jammu and Kashmir Football Association", "JKFA"),
 "TS": ("Telangana Football Association", "TFA-TS"), "AP": ("Andhra Pradesh Football Association", "APFA"), "RJ": ("Rajasthan Football Association", "RFA"),
 "GJ": ("Gujarat State Football Association", "GSFA"), "UP": ("Uttar Pradesh Football Association", "UPFA"), "HR": ("Haryana Football Association", "HFA"),
 "HP": ("Himachal Pradesh Football Association", "HPFA"), "MP": ("Madhya Pradesh Football Association", "MPFA"), "UK": ("Uttarakhand Football Association", "UFA"),
 "BR": ("Bihar Football Association", "BFA"), "CG": ("Chhattisgarh Football Association", "CFA"), "AR": ("Arunachal Pradesh Football Association", "APFA-AR"),
 "PY": ("Puducherry Football Association", "PFA"), "CH": ("Chandigarh Football Association", "CFA-CH"), "LA": ("Ladakh Football Association", "LFA"),
}
assoc = [{"id": "assoc.aiff", "name": "All India Football Federation", "abbr": "AIFF", "kind": "national", "hq_city": "New Delhi", "website": "https://www.the-aiff.com", "aiff_member": False, "status": "active",
          "prov": prov("inferred", "C", "Federation identity is well known; website not fetched in this session.")}]
for k, (n, ab) in ASSOC.items():
    a = {"id": f"assoc.{ab.lower()}", "name": n, "abbr": ab, "kind": "state", "state": f"state.{k.lower()}", "parent": "assoc.aiff", "aiff_member": True, "status": "unknown", "prov": prov()}
    if k in packst and packst[k]["assoc"] != n:
        a["prov"]["note"] = f"{GK}. The scenario pack called it '{packst[k]['assoc']}'; the actual official name must be checked."
    assoc.append(a)
for sid, n, ab in [("ssb", "Services Sports Control Board", "SSCB"), ("rspb", "Railway Sports Promotion Board", "RSPB")]:
    assoc.append({"id": f"assoc.{ab.lower()}", "name": n, "abbr": ab, "kind": "institutional", "parent": "assoc.aiff", "aiff_member": True, "status": "unknown", "prov": prov()})
write("associations/associations.toml", meta("associations", "C", "AIFF, 32 state/UT associations named in the scenario pack, and the Services and Railways institutional boards.",
      ["Not source-checked: exact official names/abbreviations; associations for Andaman & Nicobar, Lakshadweep and Dadra & Nagar Haveli and Daman & Diu not listed; suspension status of every association unknown."]), [("association", assoc)])
man("associations/associations.toml", "Association names from general knowledge", f"{len(assoc)} bodies", ["Names unverified; AN/LD/DN missing"], "C")

# ---------------------------------------------------------------- competitions
def comp(id, name, short, org, level, age, kind, tier=None, teams=None, fmt=None, gender="men", prom=None, cal=None, aliases=None, extra=None, note=None):
    r = {"id": id, "name": name, "short": short}
    if aliases: r["aliases"] = aliases
    r.update({"organiser": org, "level": level})
    if tier: r["tier"] = tier
    r.update({"age": age, "gender": gender, "kind": kind, "season": "2026-27"})
    if teams: r["teams"] = teams
    if fmt: r["format"] = fmt
    if prom: r["promotion"] = prom
    if cal: r["calendar"] = cal
    r["status"] = "active"
    r["prov"] = prov("inferred", "C", note or GK + "; format and team count are the general understanding and must be checked against the season regulations.")
    if extra: r.update(extra)
    return r


C = [
 comp("comp.isl", "Indian Super League", "ISL", "assoc.aiff", "national", "senior", "league", 1, 13, "Double round-robin league followed by playoffs (format has varied by season)", cal="Sep-Apr",
      aliases=["Hero ISL"], note="The AIFF confirmed ISL 2026-27 will proceed with 13 clubs (headline seen in search results, page not opened). Commercial operator and rights after the FSDL agreement are NOT verified."),
 comp("comp.ifl", "Indian Football League", "IFL", "assoc.aiff", "national", "senior", "league", 2, None, "League", aliases=["I-League"], cal="Oct-Apr",
      note="Second tier. Official current name unverified: earlier the I-League. Promotion to the ISL has been discussed and changed between seasons; do not treat as settled."),
 comp("comp.ifl2", "Indian Football League 2", "IFL2", "assoc.aiff", "national", "senior", "league", 3, None, "Group stage plus final round", aliases=["I-League 2"], cal="Jan-Apr", note=GK + "; third tier."),
 comp("comp.ifl3", "Indian Football League 3", "IFL3", "assoc.aiff", "national", "senior", "league", 4, None, "Clubs nominated by state associations", aliases=["I-League 3"], cal="Jan-Apr", note=GK + "; fourth tier."),
 comp("comp.super-cup", "Super Cup", "Super Cup", "assoc.aiff", "national", "senior", "cup", None, None, "Knockout, ISL and I-League clubs plus qualifiers", aliases=["Hero Super Cup"], cal="Apr or Oct (varied)"),
 comp("comp.durand-cup", "Durand Cup", "Durand", "assoc.aiff", "national", "senior", "tournament", None, None, "Group stage and knockouts, hosted in multiple cities; Indian armed-forces teams take part",
      cal="Jul-Aug", note=GK + "; run with the Indian Army, one of the oldest tournaments in Asia."),
 comp("comp.santosh-trophy", "Santosh Trophy", "Santosh", "assoc.aiff", "national", "senior", "championship", None, None, "Zonal qualifiers then final round; state associations plus Services and Railways",
      aliases=["National Football Championship for the Santosh Trophy"], cal="Oct-Feb varying"),
 comp("comp.rfdl", "Reliance Foundation Development League", "RFDL", "assoc.aiff", "national", "u21", "league", None, None, "League of ISL and I-League club development sides", cal="Dec-Apr"),
 comp("comp.youth-league-u17", "Elite Youth League U17", "U17", "assoc.aiff", "national", "u17", "league", note=GK + "; AIFF youth leagues, exact names and structure unverified."),
 comp("comp.youth-league-u15", "Elite Youth League U15", "U15", "assoc.aiff", "national", "u15", "league", note=GK + "; AIFF youth leagues, exact names and structure unverified."),
 comp("comp.youth-league-u13", "Youth League U13", "U13", "assoc.aiff", "national", "u13", "league", note=GK + "; AIFF youth leagues, exact names and structure unverified."),
 comp("comp.junior-nfc", "Junior National Football Championship", "Junior NFC", "assoc.aiff", "national", "u19", "championship", note=GK + "; state teams; age band unverified."),
 comp("comp.sub-junior-nfc", "Sub-Junior National Football Championship", "Sub-Junior NFC", "assoc.aiff", "national", "u16", "championship", note=GK + "; state teams; age band unverified."),
 comp("comp.iwl", "Indian Women's League", "IWL", "assoc.aiff", "national", "senior", "league", 1, None, "League", gender="women", cal="Varies"),
 comp("comp.iwl2", "Indian Women's League 2", "IWL2", "assoc.aiff", "national", "senior", "league", 2, None, "League", gender="women"),
 comp("comp.senior-women-nfc", "Senior Women's National Football Championship", "Women's NFC", "assoc.aiff", "national", "senior", "championship", gender="women"),
 comp("comp.subroto-cup", "Subroto Cup International Football Tournament", "Subroto Cup", "assoc.ssb-organiser", "national", "u17", "tournament", note=GK + "; school tournament run by the Indian Air Force's Subroto Mukerjee Sports Education Society; age categories unverified."),
 comp("comp.kiyg", "Khelo India Youth Games (football)", "KIYG", "assoc.gov-sports", "national", "u18", "tournament", gender="mixed", note=GK + "; organised by the Ministry of Youth Affairs and Sports."),
 comp("comp.kiug", "Khelo India University Games (football)", "KIUG", "assoc.gov-sports", "national", "senior", "tournament", gender="mixed", note=GK),
 comp("comp.aiu-north", "AIU North Zone Inter-University Football", "AIU North", "assoc.aiu", "zonal", "senior", "tournament"),
 comp("comp.aiu-south", "AIU South Zone Inter-University Football", "AIU South", "assoc.aiu", "zonal", "senior", "tournament"),
 comp("comp.aiu-east", "AIU East Zone Inter-University Football", "AIU East", "assoc.aiu", "zonal", "senior", "tournament"),
 comp("comp.aiu-west", "AIU West Zone Inter-University Football", "AIU West", "assoc.aiu", "zonal", "senior", "tournament"),
 comp("comp.aiu-inter-zone", "AIU All India Inter-University Football", "AIU All India", "assoc.aiu", "national", "senior", "tournament"),
 comp("comp.sgfi-u14", "School Games Federation of India Football U14", "SGFI U14", "assoc.sgfi", "national", "u14", "tournament"),
 comp("comp.sgfi-u17", "School Games Federation of India Football U17", "SGFI U17", "assoc.sgfi", "national", "u17", "tournament"),
 comp("comp.sgfi-u19", "School Games Federation of India Football U19", "SGFI U19", "assoc.sgfi", "national", "u19", "tournament"),
]
write("competitions/national.toml", meta("competitions.national", "C", "The national competitions the game references.",
      ["Official formats, team counts, promotion/relegation and prize money not verified for 2026-27.", "ISL operator and commercial arrangements after 2025 not verified.", "Foreign-player and registration rules not recorded (see rules/registration.toml)."], "2026-27"), [("competition", C)])
ORG = [{"id": "assoc.aiu", "name": "Association of Indian Universities", "abbr": "AIU", "kind": "national", "status": "active", "aiff_member": False, "prov": prov()},
       {"id": "assoc.sgfi", "name": "School Games Federation of India", "abbr": "SGFI", "kind": "national", "status": "active", "aiff_member": False, "prov": prov()},
       {"id": "assoc.gov-sports", "name": "Ministry of Youth Affairs and Sports (Khelo India)", "abbr": "MYAS", "kind": "national", "status": "active", "aiff_member": False, "prov": prov()},
       {"id": "assoc.ssb-organiser", "name": "Subroto Mukerjee Sports Education Society", "abbr": "SMSES", "kind": "national", "status": "active", "aiff_member": False, "prov": prov()}]
write("associations/other_organisers.toml", meta("associations.organisers", "C", "Non-AIFF organisers referenced by competitions.", ["Names from general knowledge, unverified."]), [("association", ORG)])
man("competitions/national.toml", "National competition list from general knowledge", f"{len(C)} competitions", ["Formats/rules unverified"], "C")

# ---------------------------------------------------------------- clubs and membership
def cid(name):
    return "club." + name.lower().replace(" fc", "").replace("&", "and").replace(" ", "-").replace("--", "-")


special = {"Mohun Bagan Super Giant": "club.mohun-bagan-sg", "Mohammedan SC": "club.mohammedan", "Bengaluru FC": "club.bengaluru", "FC Goa": "club.goa", "Chennaiyin FC": "club.chennaiyin",
           "Jamshedpur FC": "club.jamshedpur", "Mumbai City FC": "club.mumbai-city", "Odisha FC": "club.odisha", "Punjab FC": "club.punjab", "Hyderabad FC": "club.hyderabad",
           "Aizawl FC": "club.aizawl", "Delhi FC": "club.delhi", "Sudeva Delhi FC": "club.sudeva-delhi", "Dempo SC": "club.dempo", "Kenkre FC": "club.kenkre", "Bhawanipore FC": "club.bhawanipore"}
tiername = {1: "comp.isl", 2: "comp.ifl", 3: "comp.ifl2", 4: "comp.ifl3"}
clubs, mem = [], []
for c in pack["club"]:
    id_ = special.get(c["name"]) or cid(c["name"])
    kind = "academy" if c["tier"] == 0 else "professional"
    clubs.append({"id": id_, "name": c["name"], "city": c["city"], "state": f"state.{c['state'].lower()}", "kind": kind, "status": "unknown",
                  "prov": prov("unknown", "C", "The club is real. Its current status, tier for 2026-27 and every other fact beyond name/city/state are unverified: the scenario pack listed it and general knowledge agrees it exists.")})
    if c["tier"] in tiername:
        mem.append({"club": id_, "competition": tiername[c["tier"]], "season": "2026-27", "note": "Tier is the scenario pack's Day-0 seed, NOT a verified 2026-27 participant list.", "prov": prov("scenario_seed", "D", "Carried over from scenario pack.")})
for id_, n, city, st in [("club.diamond-harbour", "Diamond Harbour FC", "Diamond Harbour", "WB"), ("club.united-sc", "United SC", "Kolkata", "WB"), ("club.namdhari", "Namdhari FC", "Bhaini Sahib", "PB")]:
    clubs.append({"id": id_, "name": n, "city": city, "state": f"state.{st.lower()}", "kind": "professional", "status": "unknown", "prov": prov("unknown", "C", "Real club referenced by state_leagues/wb.toml; every fact beyond name/state is unverified.")})
write("clubs/national.toml", meta("clubs.national", "D", f"{len(clubs)} real clubs carried over from the scenario pack, identity only.",
      ["Founded year, home ground, ownership, city detail, current status unverified.", "Many ISL/IFL/IFL2/IFL3 clubs missing (the pack listed only 44).", "Clubs may have been renamed, relocated, withdrawn or dissolved since the pack was written."]), [("club", clubs)])
write("competitions/membership.toml", meta("competitions.membership", "D", "Tier membership as seeded by the scenario pack.", ["No verified 2025-26 or 2026-27 participant lists."], "2026-27"), [("membership", mem)])
man("clubs/national.toml + competitions/membership.toml", "Clubs and tiers carried from the scenario pack", f"{len(clubs)} clubs", ["Participant lists not researched"], "D")

# ---------------------------------------------------------------- media
def O(id, name, kind, medium, langs, reach, emph, state=None, city=None, focus=None, owner=None, site=None):
    r = {"id": f"media.{id}", "name": name, "kind": kind, "medium": medium, "languages": [f"lang.{l}" for l in langs], "reach": reach}
    if state: r["home_state"] = f"state.{state}"
    if city: r["home_city"] = city
    if focus: r["focus"] = focus
    r["football_emphasis"] = emph
    if owner: r["owner"] = owner
    if site: r["website"] = site
    r["active"] = True
    r["prov"] = prov("inferred", "C", GK + "; the outlet is real and its language/type is well known, but emphasis and reach category are a broad characterisation only.")
    return r


OUT = [
 O("times-of-india-sport", "The Times of India (sports desk)", "general_news", ["print", "digital"], ["en"], "national", "general", owner="Bennett, Coleman & Co."),
 O("hindustan-times-sport", "Hindustan Times (sports desk)", "general_news", ["print", "digital"], ["en"], "national", "general", owner="HT Media"),
 O("the-hindu-sport", "The Hindu (sport)", "general_news", ["print", "digital"], ["en"], "national", "general", city="Chennai"),
 O("indian-express-sport", "The Indian Express (sport)", "general_news", ["print", "digital"], ["en"], "national", "general"),
 O("telegraph-india-sport", "The Telegraph India (sport)", "general_news", ["print", "digital"], ["en"], "multi_state", "strong", state="wb", city="Kolkata", focus=["professional", "regional_leagues"]),
 O("deccan-herald-sport", "Deccan Herald (sport)", "general_news", ["print", "digital"], ["en"], "state", "general", state="ka", city="Bengaluru"),
 O("sportstar", "Sportstar", "national_sports", ["print", "digital"], ["en"], "national", "general", owner="Kasturi & Sons"),
 O("pti", "Press Trust of India", "news_agency", ["digital"], ["en"], "national", "general"),
 O("ians", "IANS", "news_agency", ["digital"], ["en", "hi"], "national", "general"),
 O("khel-now", "Khel Now", "football_specialist", ["digital", "video"], ["en"], "national", "specialist", focus=["professional", "national_team", "youth", "transfers"]),
 O("the-bridge", "The Bridge", "digital_sports", ["digital"], ["en"], "national", "general"),
 O("indian-football-network", "Indian Football Network", "football_specialist", ["digital"], ["en"], "national", "specialist"),
 O("sportskeeda-football", "Sportskeeda (football)", "digital_sports", ["digital"], ["en"], "national", "general"),
 O("goal-india", "Goal India", "football_specialist", ["digital"], ["en", "hi"], "national", "specialist"),
 O("espn-india-football", "ESPN India (football)", "digital_sports", ["digital"], ["en"], "national", "general"),
 O("anandabazar", "Anandabazar Patrika", "regional", ["print", "digital"], ["bn"], "state", "strong", state="wb", city="Kolkata", focus=["professional", "state", "regional_leagues"], owner="ABP Group"),
 O("bartaman", "Bartaman", "regional", ["print", "digital"], ["bn"], "state", "strong", state="wb", city="Kolkata"),
 O("sangbad-pratidin", "Sangbad Pratidin", "regional", ["print", "digital"], ["bn"], "state", "strong", state="wb", city="Kolkata"),
 O("aajkaal", "Aajkaal", "regional", ["print", "digital"], ["bn"], "state", "general", state="wb", city="Kolkata"),
 O("mathrubhumi-sports", "Mathrubhumi (sports)", "regional", ["print", "digital", "tv"], ["ml"], "state", "strong", state="kl", city="Kozhikode", focus=["professional", "state", "regional_leagues"]),
 O("malayala-manorama-sports", "Malayala Manorama (sports)", "regional", ["print", "digital"], ["ml"], "state", "strong", state="kl", city="Kottayam"),
 O("madhyamam", "Madhyamam", "regional", ["print", "digital"], ["ml"], "state", "strong", state="kl", city="Kozhikode"),
 O("deshabhimani", "Deshabhimani", "regional", ["print", "digital"], ["ml"], "state", "general", state="kl"),
 O("herald-goa", "The Herald (Goa)", "local_newspaper", ["print", "digital"], ["en"], "state", "strong", state="ga", city="Panaji", focus=["professional", "state", "regional_leagues"]),
 O("navhind-times", "The Navhind Times", "local_newspaper", ["print", "digital"], ["en"], "state", "strong", state="ga", city="Panaji"),
 O("gomantak", "Gomantak", "regional", ["print", "digital"], ["mr", "kok"], "state", "general", state="ga", city="Panaji"),
 O("sambad", "Sambad", "regional", ["print", "digital"], ["or"], "state", "general", state="od", city="Bhubaneswar"),
 O("dharitri", "Dharitri", "regional", ["print", "digital"], ["or"], "state", "general", state="od", city="Bhubaneswar"),
 O("assam-tribune", "The Assam Tribune", "regional", ["print", "digital"], ["en"], "state", "general", state="as", city="Guwahati"),
 O("asomiya-pratidin", "Asomiya Pratidin", "regional", ["print", "digital"], ["as"], "state", "general", state="as", city="Guwahati"),
 O("sentinel-assam", "The Sentinel (Assam)", "regional", ["print", "digital"], ["en"], "state", "general", state="as", city="Guwahati"),
 O("sangai-express", "The Sangai Express", "regional", ["print", "digital"], ["en", "mni"], "state", "strong", state="mn", city="Imphal", focus=["state", "regional_leagues"]),
 O("imphal-free-press", "Imphal Free Press", "regional", ["print", "digital"], ["en"], "state", "strong", state="mn", city="Imphal"),
 O("vanglaini", "Vanglaini", "regional", ["print", "digital"], ["lus"], "state", "strong", state="mz", city="Aizawl"),
 O("shillong-times", "The Shillong Times", "local_newspaper", ["print", "digital"], ["en"], "state", "strong", state="ml", city="Shillong", focus=["state", "regional_leagues"]),
 O("highland-post", "Highland Post", "local_newspaper", ["print", "digital"], ["en", "kha"], "state", "general", state="ml", city="Shillong"),
 O("ajit", "Ajit", "regional", ["print", "digital"], ["pa"], "state", "general", state="pb", city="Jalandhar"),
 O("jagbani", "Jagbani", "regional", ["print", "digital"], ["pa"], "state", "general", state="pb", city="Jalandhar"),
 O("dinamalar", "Dinamalar", "regional", ["print", "digital"], ["ta"], "state", "general", state="tn"),
 O("dinamani", "Dinamani", "regional", ["print", "digital"], ["ta"], "state", "general", state="tn"),
 O("eenadu", "Eenadu", "regional", ["print", "digital"], ["te"], "multi_state", "general", state="ts"),
 O("sakshi", "Sakshi", "regional", ["print", "digital"], ["te"], "multi_state", "general", state="ts"),
 O("lokmat", "Lokmat", "regional", ["print", "digital"], ["mr"], "state", "general", state="mh"),
 O("maharashtra-times", "Maharashtra Times", "regional", ["print", "digital"], ["mr"], "state", "general", state="mh", city="Mumbai"),
 O("prajavani", "Prajavani", "regional", ["print", "digital"], ["kn"], "state", "general", state="ka", city="Bengaluru"),
 O("dainik-jagran", "Dainik Jagran", "regional", ["print", "digital"], ["hi"], "multi_state", "general"),
 O("amar-ujala", "Amar Ujala", "regional", ["print", "digital"], ["hi"], "multi_state", "general"),
 O("dainik-bhaskar", "Dainik Bhaskar", "regional", ["print", "digital"], ["hi", "gu"], "multi_state", "general"),
 O("star-sports", "Star Sports (network)", "tv_sports", ["tv"], ["en", "hi", "bn", "ml", "ta", "te", "kn", "mr"], "national", "general"),
 O("sony-sports", "Sony Sports Network", "tv_sports", ["tv"], ["en", "hi", "ta", "te"], "national", "general"),
 O("dd-sports", "DD Sports", "tv_sports", ["tv"], ["en", "hi"], "national", "general", owner="Prasar Bharati"),
 O("fancode", "FanCode", "digital_sports", ["digital", "video"], ["en", "hi"], "national", "general"),
 O("aiff-media", "AIFF official channels", "official_federation", ["digital", "social", "video"], ["en"], "national", "specialist", owner="All India Football Federation", site="https://www.the-aiff.com"),
 O("isl-official", "Indian Super League official channels", "official_league", ["digital", "social", "video"], ["en"], "national", "specialist", site="https://www.indiansuperleague.com"),
]
write("media/outlets.toml", meta("media.outlets", "C", f"{len(OUT)} real outlets across national, football-specialist, regional-language, TV/digital and official layers.",
      ["Reach, emphasis and languages are broad general-knowledge characterisations; not verified per outlet.", "Official club media, local newspapers below state level, radio and campus media not listed.", "Football-specialist outlets can close or change; active flag not checked.", "No journalists are named on purpose: journalists are simulated people attached to these outlets."]), [("outlet", OUT)])
man("media/outlets.toml", "Outlet list from general knowledge", f"{len(OUT)} outlets", ["Not source-checked", "Local/radio/campus/official club media missing"], "C")

BC = [{"id": "bcast.jiostar", "name": "JioStar (Star Sports / JioHotstar)", "medium": ["tv", "digital"], "languages": [f"lang.{l}" for l in ["en", "hi", "bn", "ml", "ta", "te", "kn", "mr"]], "region": "national", "prov": prov()},
      {"id": "bcast.sony-sports", "name": "Sony Sports Network / SonyLIV", "medium": ["tv", "digital"], "languages": ["lang.en", "lang.hi", "lang.ta", "lang.te"], "region": "national", "prov": prov()},
      {"id": "bcast.dd-sports", "name": "DD Sports", "medium": ["tv"], "languages": ["lang.en", "lang.hi"], "region": "national", "owner": "Prasar Bharati", "prov": prov()},
      {"id": "bcast.fancode", "name": "FanCode", "medium": ["digital"], "languages": ["lang.en", "lang.hi"], "region": "national", "prov": prov()},
      {"id": "bcast.aiff-tv", "name": "AIFF YouTube / official streaming", "medium": ["digital"], "languages": ["lang.en"], "region": "national", "prov": prov()}]
write("broadcasters/broadcasters.toml", meta("broadcasters", "C", "Platforms that have carried Indian football; NO rights records.", ["Which broadcaster carries which competition for 2025-26 and 2026-27 is not verified, so no [[rights]] rows are written. ISL rights changed around 2025; research before writing any."], "2026-27"), [("broadcaster", BC)])
man("broadcasters/broadcasters.toml", "Broadcaster identities only", "5 platforms", ["Rights unknown"], "C")

# ---------------------------------------------------------------- universities
UN = []
for u in pack["university"]:
    sl = u["name"].lower().replace("university of ", "").replace(" university", "").replace(" ", "-")
    UN.append({"id": f"uni.{sl}", "name": u["name"], "city": u["city"], "state": f"state.{u['state'].lower()}",
               "prov": prov("inferred", "C", "Institution is real (identity from scenario pack, checked against general knowledge). Type, zone, football participation and any achievements are NOT recorded because they were not verified.")})
write("universities/universities.toml", meta("universities", "C", f"{len(UN)} real universities from the scenario pack, identity only.",
      ["Football participation, AIU zone, type, facilities and achievements unrecorded.", "Many universities that play AIU football are missing.", "Scholarship counts from the pack are scenario seeds and are not carried here."]), [("university", UN)])
man("universities/universities.toml", "University identities from pack", f"{len(UN)}", ["Most AIU universities missing"], "C")

# ---------------------------------------------------------------- academies / programmes / partnerships
AC = [("rf-young-champs", "Reliance Foundation Young Champs", "Navi Mumbai", "MH", "corporate", True), ("minerva-academy", "Minerva Academy", "Mohali", "PB", "independent", None),
      ("rkm-narendrapur", "Ramakrishna Mission Football Academy (Narendrapur)", "Narendrapur", "WB", "independent", True), ("pune-krida-prabodhini", "Pune Krida Prabodhini", "Pune", "MH", "state", None),
      ("bhaichung-bhutia-schools", "Bhaichung Bhutia Football Schools", "Gangtok", "SK", "independent", None), ("aiff-elite-academy", "AIFF Elite Academy", None, None, "state", True),
      ("tata-football-academy", "Tata Football Academy", "Jamshedpur", "JH", "corporate", True), ("zinc-football-academy", "Zinc Football Academy", "Udaipur", "RJ", "corporate", True),
      ("sesa-football-academy", "SESA Football Academy", "Sanquelim", "GA", "corporate", True)]
ACD = []
for id_, n, city, st, kind, res in AC:
    r = {"id": f"academy.{id_}", "name": n}
    if city: r["city"] = city
    if st: r["state"] = f"state.{st.lower()}"
    r["kind"] = kind
    if res is not None: r["residential"] = res
    r["status"] = "unknown"
    r["prov"] = prov("inferred", "C", "The institution is real and widely known. Accreditation, age groups and facilities are not recorded.")
    ACD.append(r)
write("academies/academies.toml", meta("academies", "C", f"{len(ACD)} well-known academies.", ["AIFF accreditation lists not researched.", "ISL/IFL club academies and state government academies not itemised."]), [("academy", ACD)])
PR = [("blue-cubs", "AIFF Baby Leagues / Blue Cubs", "assoc.aiff", "grassroots", "national"), ("khelo-india", "Khelo India (football talent identification and accredited academies)", "assoc.gov-sports", "talent_id", "national"),
      ("sai-ncoe", "SAI National Centres of Excellence and Training Centres (football)", "Sports Authority of India", "residential", "national")]
write("grassroots/programmes.toml", meta("grassroots", "C", "Three structural programmes only.", ["Ages, active periods, state and club programmes not recorded."]),
      [("programme", [{"id": f"prog.{i}", "name": n, "operator": o, "kind": k, "region": r, "description": "Real programme; details unverified.", "prov": prov()} for i, n, o, k, r in PR])])
write("partnerships/partnerships.toml", meta("partnerships", "E", "Empty on purpose.", ["No development partnership was verified. Real ones exist and must be researched with source and status; none is guessed."]), [])
man("academies/ grassroots/ partnerships/", "Small general-knowledge lists", "9 academies, 3 programmes, 0 partnerships", ["Almost everything"], "C")

# ---------------------------------------------------------------- coach education, referees, rivalries, terminology
LI = [("aiff-grassroots", "AIFF Grassroots Certificate", 1), ("aiff-d", "AIFF D Licence", 2), ("aiff-c", "AIFF C Licence", 3), ("aiff-b", "AIFF B Licence", 4), ("aiff-a", "AIFF A Licence", 5), ("aiff-pro", "AIFF Pro Licence", 6)]
write("development/coach_education.toml", meta("development.coach_education", "C", "Level ladder only.",
      ["Prerequisites, minimum ages, experience and which licence a head coach needs for each tier are not recorded.", "Whether AFC licence names/levels map one-to-one to these labels is unverified."]),
      [("licence", [{"id": f"licence.{i}", "name": n, "body": "AIFF", "order": o, "prov": prov()} for i, n, o in LI])])
write("development/referees.toml", meta("development.referees", "E", "Empty on purpose.", ["Referee progression not researched."]), [])
RV = [("kolkata-derby", "Kolkata Derby", "club.east-bengal", "club.mohun-bagan-sg", "derby", "Long-standing rivalry between two Kolkata clubs, widely documented."),
      ("southern-derby", "Southern Derby", "club.bengaluru", "club.kerala-blasters", "rivalry", "Rivalry between Bengaluru FC and Kerala Blasters that developed in the ISL era.")]
write("culture/rivalries.toml", meta("culture.rivalries", "C", "Two well-known rivalries only.", ["Goa, Mizoram, Manipur and Meghalaya derbies not recorded."]),
      [("rivalry", [{"id": f"rivalry.{i}", "name": n, "a": a, "b": b, "kind": k, "basis": bs, "prov": prov()} for i, n, a, b, k, bs in RV])])
TERM = [("competition.top", "Indian Super League", ["ISL", "the Super League"], "headline"), ("competition.state-team", "Santosh Trophy", ["National Football Championship", "Santosh"], "neutral"),
        ("competition.youth-dev", "Reliance Foundation Development League", ["RFDL", "Development League"], "neutral"), ("org.federation", "AIFF", ["All India Football Federation", "the federation"], "neutral"),
        ("org.state-assoc", "state association", ["state football association", "state FA"], "neutral"), ("org.academy", "academy", ["development centre", "residential academy"], "neutral")]
write("languages/terminology.toml", meta("languages.terminology", "C", "Seed vocabulary only.", ["Region/language-specific vocabulary not researched."]),
      [("term", [{"id": f"term.{c}", "concept": c, "canonical": cn, "synonyms": s, "register": r, "lang": "lang.en", "prov": prov()} for c, cn, s, r in TERM])])
man("development/ culture/ languages/terminology.toml", "Small general-knowledge seeds", "6 licences, 2 rivalries, 6 terms", ["Referee grades, rules, partnerships empty"], "C/E")

# ---------------------------------------------------------------- rules: none verified
write("rules/registration.toml", meta("rules.registration", "E", "Empty on purpose.", ["Foreign-player limits, registration windows, U-age requirements and salary cap are NOT recorded: they were not verified and are volatile. Do not fill from memory."], "2026-27"), [])
man("state_leagues/wb.toml", "West Bengal leagues (CFL Premier and divisions, IFA Shield, Bengal Super League) researched via search and Wikipedia", "Premier Division 2025/2026 lineup; lower divisions structure only", ["Lower division participants", "district leagues"], "B")
write("_manifest/index.toml", {"dataset": "manifest", "generated_by": "tools/build_india_reference.py", "last_checked": "2026-09-29"}, [("dataset", manifest)])
print("wrote", len(manifest), "manifest rows")
