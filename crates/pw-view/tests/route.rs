//! Beginning a life on the route: the options depend on the world, and the person made is a real one the human then lives as.

use std::time::{Duration, Instant};

use pw_view::Api;
use serde_json::{Value, json};

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-view-route-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, what: &str) {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[what]["running"].as_bool().unwrap_or(false) {
            assert!(s["task"]["error"].is_null(), "task failed: {}", s["task"]["error"]);
            return;
        }
        assert!(t0.elapsed() < Duration::from_secs(300), "{what} timed out");
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn new_world(api: &Api, args: Value) {
    api.call("world.new", args).unwrap();
    wait(api, "task");
}

#[test]
fn a_world_without_a_route_offers_no_starts() {
    let api = api();
    new_world(&api, json!({"kind": "synthetic", "scale": "tiny"}));
    let o = api.call("route.options", json!({})).unwrap();
    assert_eq!(o["available"], false);
    assert!(api.call("route.begin", json!({"start": "school_standout"})).is_err());
}

#[test]
fn the_india_world_offers_every_start_and_the_person_made_is_lived_as() {
    let api = api();
    new_world(&api, json!({"kind": "india", "scale": "tiny", "seed": 5}));
    let o = api.call("route.options", json!({})).unwrap();
    assert_eq!(o["available"], true);
    assert_eq!(o["starts"].as_array().unwrap().len(), 6);
    let states = o["states"].as_array().unwrap();
    assert!(!states.is_empty() && states.iter().all(|s| !s["districts"].as_array().unwrap().is_empty()));
    let district = states[0]["districts"][0]["id"].as_u64().unwrap();

    assert!(api.call("route.begin", json!({"start": "nowhere"})).is_err(), "unknown start");
    assert!(api.call("route.begin", json!({"start": "school_standout", "district": states[0]["id"]})).is_err(), "a state is not a district");

    let made = api.call("route.begin", json!({"start": "school_standout", "district": district, "first": "Asha", "last": "Devi"})).unwrap();
    let status = api.call("world.status", json!({})).unwrap();
    assert_eq!(status["perspective"]["mode"], "inhabit");
    assert_eq!(status["perspective"]["name"], "Asha Devi", "{status}");
    let page = api.call("person", json!({"id": made["person"]})).unwrap();
    assert_eq!(page["is_me"], true);

    // Every other start either works or says plainly why not; none panics or leaves a half-made person.
    for start in ["released_academy", "university_freshman", "university_star", "state_league", "semi_pro"] {
        match api.call("route.begin", json!({"start": start, "district": district})) {
            Ok(v) => assert!(v["person"].is_u64(), "{start}"),
            Err(e) => assert!(format!("{e:?}").contains("no school, university or club"), "{start}: {e:?}"),
        }
    }
    // The life goes on: the days pass and today still answers.
    api.call("advance.start", json!({"mode": "days", "n": 30})).unwrap();
    wait(&api, "job");
    api.call("me.today", json!({})).unwrap();
}
