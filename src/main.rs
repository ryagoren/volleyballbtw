use scraper::{Html, Selector};
use serde::Serialize;
use std::io::Write;
use clap::Parser;

#[derive(Debug, Serialize)]
struct TeamStats {
    position: String,
    team: String,
    played: String,
    wins: String,
    losses: String,
    sets_for: String,
    sets_against: String,
    sets_difference: String,
    points_for: String,
    points_against: String,
    points_quotient: String,
    points: String,
}

#[derive(Debug, Serialize)]
struct TablePls {
    competition_id: String,
    #[serde(rename = "pageTitle")]
    page_title: String,
}

async fn fetch_html(competition_id: &str) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static(
            "application/x-www-form-urlencoded; charset=UTF-8",
        ),
    );

    let request_body = TablePls {
        competition_id: competition_id.to_string(),
        page_title: "Fixture and Results".to_string(),
    };

    let response = client
        .post("https://competitions.volleyzone.co.uk/wp-admin/admin-ajax.php?action=fetch_table_by_competition")
        .headers(headers)
        .form(&request_body)
        .send()
        .await?;

    let text = response.text().await?;
    let jval: serde_json::Value = serde_json::from_str(&text)?;
    Ok(jval["CompTables"]
        .as_str()
        .map(|ct| ct.to_string())
        .expect("no table we tried"))
}

fn parse_volleyball_table(html: &str) -> Result<Vec<TeamStats>, anyhow::Error> {
    let document = Html::parse_document(html);
    let table_row_selector = Selector::parse("tr.tableContents").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    let mut teams = Vec::new();

    for row in document.select(&table_row_selector) {
        let cells: Vec<String> = row
            .select(&td_selector)
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .collect();

        if cells.len() >= 12 {
            let team_stats = TeamStats {
                position: cells[0].clone(),
                team: cells[1].clone(),
                played: cells[2].clone(),
                wins: cells[3].clone(),
                losses: cells[4].clone(),
                sets_for: cells[5].clone(),
                sets_against: cells[6].clone(),
                sets_difference: cells[7].clone(),
                points_for: cells[8].clone(),
                points_against: cells[9].clone(),
                points_quotient: cells[10].clone(),
                points: cells[11].clone(),
            };
            teams.push(team_stats);
        }
    }
    Ok(teams)
}

fn save_csv(teams: &[TeamStats], writer: &mut impl Write) -> anyhow::Result<()> {
    writeln!(writer, "Position,Team,Played,Wins,Losses,Sets For,Sets Against,Sets Difference,Points For,Points Against,Points Quotient,Points")?;
    for team in teams {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            team.position,
            team.team,
            team.played,
            team.wins,
            team.losses,
            team.sets_for,
            team.sets_against,
            team.sets_difference,
            team.points_for,
            team.points_against,
            team.points_quotient,
            team.points
        )?;
    }
    Ok(())
}

#[derive(clap::Parser, Debug)]
struct Args {
    output_dir: Option<String>
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let Args {output_dir} = Args::parse();

    let output_dir = output_dir.map(|x| format!("{x}/")).unwrap_or_default();

    let teams = vec![
        ("division_1_men_nvl", "196048"),
        ("div_2a_men", "198880"),
        ("div_3a_men", "198882"),
        ("div_1a_women", "198885"),
        ("div_1b_women", "198886"),
        ("div_2a_women", "198887"),
        ("div_2b_women", "198888"),
    ];

    for (label, id) in teams {
        eprintln!("Retrieving table for team={label}, id={id}...");
        let html = fetch_html(id).await?;
        let teams = parse_volleyball_table(&html)?;
        let mut file = std::fs::File::create(format!("{output_dir}{label}.csv"))?;
        save_csv(&teams, &mut file);
        eprintln!("Saved: {:?}", id);
    }
    Ok(())
}
