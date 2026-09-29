//! A small, entirely fictional archive in the Transfermarkt layout, for tests: two countries with six clubs each, 22
//! players a club, two finished seasons of results and a cup. No real data; every name and number is made here.

use std::path::PathBuf;

// ---- fixture -------------------------------------------------------------------------------------------------

pub fn q(cells: &[&str]) -> String {
    cells.iter().map(|c| format!("\"{}\"", c.replace('"', "\"\""))).collect::<Vec<_>>().join(",")
}

#[derive(Default)]
pub struct Archive {
    pub countries: Vec<String>,
    pub comps: Vec<String>,
    pub clubs: Vec<String>,
    pub players: Vec<String>,
    pub transfers: Vec<String>,
    pub games: Vec<String>,
    pub appearances: Vec<String>,
    pub lineups: Vec<String>,
}

const FIRST: [&str; 24] = [
    "Bruno", "Carla", "Dario", "Elena", "Felix", "Greta", "Hugo", "Irene", "Jonas", "Klara", "Luca", "Maria", "Nico", "Olga", "Pablo", "Quinn", "Rosa", "Stefan", "Tara", "Ugo", "Vera", "Walid", "Yara", "Zoe",
];

const POS: [&str; 22] = [
    "Goalkeeper", "Goalkeeper", "Centre-Back", "Centre-Back", "Centre-Back", "Right-Back", "Left-Back", "Defensive Midfield", "Central Midfield", "Central Midfield", "Attacking Midfield", "Right Winger",
    "Left Winger", "Centre-Forward", "Centre-Forward", "Centre-Forward", "Centre-Back", "Central Midfield", "Right-Back", "Left-Back", "Goalkeeper", "Second Striker",
];

impl Archive {
    /// England (GB1) and Spain (ES1) with six clubs each, 22 players a club, two finished seasons of results, one cup.
    pub fn standard() -> Archive {
        let mut a = Archive::default();
        a.countries.push(q(&["1", "England", "GB1", "europa", "6", "132", "25", "u"]));
        a.countries.push(q(&["2", "Spain", "ES1", "europa", "6", "132", "25", "u"]));
        a.countries.push(q(&["3", "Argentina", "AR1", "amerika", "0", "0", "25", "u"]));
        a.comps.push(q(&["GB1", "premier", "premier-league", "first_tier", "domestic_league", "1", "England", "GB1", "europa", "6", "u"]));
        a.comps.push(q(&["ES1", "laliga", "laliga", "first_tier", "domestic_league", "2", "Spain", "ES1", "europa", "6", "u"]));
        a.comps.push(q(&["FAC", "fa-cup", "fa-cup", "domestic_cup", "domestic_cup", "1", "England", "", "europa", "", "u"]));
        a.comps.push(q(&["CLQ", "uefa-cl-q", "uefa-champions-league-qualifying", "uefa_champions_league_qualifying", "international_cup", "-1", "", "", "europa", "", "u"]));
        a.comps.push(q(&["CL", "cl", "uefa-champions-league", "uefa_champions_league", "international_cup", "-1", "", "", "europa", "", "u"]));
        for (league, base) in [("GB1", 100), ("ES1", 200)] {
            for i in 0..6 {
                let id = base + i;
                a.clubs.push(q(&[&id.to_string(), &format!("club-{id}"), &format!("Football Club {league} Number {i}"), league, "", "22", "25", "5", "20", "3", &format!("Ground {id}"), "20000", "", &format!("Coach{id} Surname{id}"), "2025", "", "u"]));
                for k in 0..22 {
                    let pid = id * 100 + k;
                    let value = 40_000_000 / (1 + i as i64 * 3) / (1 + k as i64 / 3);
                    let year = 1990 + (k % 16);
                    let country = if k == 5 { "Brazil" } else if league == "GB1" { "England" } else { "Spain" };
                    // Two players keep the first name the tests look up; the rest vary so generated names have room to avoid real ones.
                    let first = if pid == 10000 || pid == 10001 { "Ana" } else { FIRST[(pid as usize * 7 + k as usize) % FIRST.len()] };
                    a.players.push(q(&[&pid.to_string(), first, &format!("Player{pid}"), &format!("{first} Player{pid}"), "2025", &id.to_string(), &format!("p{pid}"), country, "City", country, &format!("{year}-03-1{} 00:00:00", k % 9), POS[k as usize], "", if k % 4 == 0 { "left" } else { "right" }, &format!("{}", 172 + k % 20), "2028-06-30", "Some Agent", "", "", "", "", "", "", "", &value.to_string(), ""]));
                }
            }
        }
        // A finished double round robin for the last two seasons of GB1; club 100 wins both.
        let mut gid = 1;
        for season in [2023, 2024] {
            for h in 0..6 {
                for aw in 0..6 {
                    if h == aw {
                        continue;
                    }
                    let (hg, ag) = if h == 0 { (3, 0) } else if aw == 0 { (0, 2) } else { (1, 1) };
                    a.games.push(q(&[&gid.to_string(), "GB1", &season.to_string(), "1", &format!("{season}-09-01"), &(100 + h).to_string(), &(100 + aw).to_string(), &hg.to_string(), &ag.to_string()]));
                    gid += 1;
                }
            }
        }
        a
    }

    pub fn write(&self, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pw-archive-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let put = |file: &str, header: &[&str], rows: &[String]| {
            let mut text = format!("{}\n", q(header));
            for r in rows {
                text.push_str(r);
                text.push('\n');
            }
            std::fs::write(dir.join(file), text).unwrap();
        };
        put("countries.csv", &["country_id", "country_name", "country_code", "confederation", "total_clubs", "total_players", "average_age", "url"], &self.countries);
        put(
            "competitions.csv",
            &["competition_id", "competition_code", "name", "sub_type", "type", "country_id", "country_name", "domestic_league_code", "confederation", "total_clubs", "url"],
            &self.comps,
        );
        put(
            "clubs.csv",
            &["club_id", "club_code", "name", "domestic_competition_id", "total_market_value", "squad_size", "average_age", "foreigners_number", "foreigners_percentage", "national_team_players", "stadium_name", "stadium_seats", "net_transfer_record", "coach_name", "last_season", "filename", "url"],
            &self.clubs,
        );
        put(
            "players.csv",
            &["player_id", "first_name", "last_name", "name", "last_season", "current_club_id", "player_code", "country_of_birth", "city_of_birth", "country_of_citizenship", "date_of_birth", "sub_position", "position", "foot", "height_in_cm", "contract_expiration_date", "agent_name", "image_url", "international_caps", "international_goals", "current_national_team_id", "url", "current_club_domestic_competition_id", "current_club_name", "market_value_in_eur", "highest_market_value_in_eur"],
            &self.players,
        );
        put("transfers.csv", &["player_id", "transfer_date", "transfer_season", "from_club_id", "to_club_id", "from_club_name", "to_club_name", "transfer_fee", "market_value_in_eur", "player_name"], &self.transfers);
        put("games.csv", &["game_id", "competition_id", "season", "round", "date", "home_club_id", "away_club_id", "home_club_goals", "away_club_goals"], &self.games);
        put("appearances.csv", &["appearance_id", "game_id", "player_id", "competition_id", "goals", "minutes_played", "player_name"], &self.appearances);
        put("game_lineups.csv", &["game_lineups_id", "date", "game_id", "player_id", "club_id", "number"], &self.lineups);
        dir
    }
}


pub fn player_row(id: &str, club: &str, dob: &str, extra: impl Fn(&mut Vec<String>)) -> String {
    let mut cells: Vec<String> = ["", "Ana", "X", "Ana X", "2025", club, "", "England", "", "England", dob, "Centre-Back", "", "right", "180", "2028-06-30", "", "", "", "", "", "", "", "", "1000000", ""].iter().map(|s| s.to_string()).collect();
    cells[0] = id.to_string();
    extra(&mut cells);
    q(&cells.iter().map(String::as_str).collect::<Vec<_>>())
}

