//! A club's nickname is for supporters and local voices: a later mention in a local or slangy voice, or a supporter's post. Never a
//! formal voice's.

use pw_core::Date;
use pw_lang::{Engine, Event, Ref, Request, Speaker, Tracker, Value};

#[test]
fn local_voices_call_a_club_by_its_nickname_on_later_mentions_and_formal_ones_never_do() {
    let eng = Engine::builtin();
    let date = Date::from_ymd(2026, 8, 1);
    let bagan = || Value::Ent(Ref::new("club.1", "Mohun Bagan Super Giant", "Mohun Bagan").with_desc("nickname", "Mariners"));
    let other = || Value::Ent(Ref::new("club.2", "Kerala Blasters FC", "Kerala Blasters"));
    let player = || Value::Ent(Ref::new("person.1", "Sahal Abdul Samad", "Samad").with_desc("role", "midfielder"));
    let comp = || Value::Ent(Ref::new("comp.1", "Indian Super League", "ISL"));
    let manager = || Value::Ent(Ref::new("person.2", "Jose Molina", "Molina").with_desc("title", "manager"));
    let events: Vec<(&str, Vec<(&str, Value)>)> = vec![
        ("contract.renewed", vec![("player", player()), ("club", bagan())]),
        ("transfer.completed", vec![("player", player()), ("from", other()), ("to", bagan()), ("fee", Value::Money { money: 25_000_000 })]),
        ("manager.appointed", vec![("manager", manager()), ("club", bagan())]),
        ("manager.departed", vec![("manager", manager()), ("club", bagan()), ("manner", Value::Text("sacked".into()))]),
        ("competition.promotion", vec![("club", bagan()), ("competition", comp())]),
        ("competition.relegation", vec![("club", bagan()), ("competition", comp())]),
        ("player.released", vec![("player", player()), ("club", bagan())]),
        ("injury.suffered", vec![("player", player()), ("club", bagan())]),
    ];
    let (mut local_uses, mut formal_uses, mut renders) = (0, 0, 0);
    for (kind, facts) in &events {
        let ev = Event { kind: (*kind).into(), date, facts: facts.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect() };
        let keys: Vec<&str> = facts.iter().map(|(k, _)| *k).collect();
        for (name, v) in &eng.lang.voices {
            let voice = eng.voice(name);
            let local = (voice.local >= 0.6 || voice.slang >= 0.5) && voice.formality < 0.7 || v.role == "fan";
            for ch in eng.lang.channels.keys() {
                for seed in 0..20 {
                    let sp = Speaker::new("s", &v.role, voice.clone()).knows_all(&keys);
                    let req = Request::new(&ev, &sp, ch, date, seed);
                    let text = eng.render(&req, &mut Tracker::new()).text();
                    renders += 1;
                    if text.contains("Mariners") {
                        assert!(text.contains("the Mariners") || text.contains("The Mariners"), "a nickname takes its article: {text}");
                        if local { local_uses += 1 } else { formal_uses += 1 }
                    }
                }
            }
        }
    }
    assert_eq!(formal_uses, 0, "a formal voice never uses the nickname");
    assert!(local_uses > 0, "no supporter or local voice used the nickname in {renders} renders");
}
