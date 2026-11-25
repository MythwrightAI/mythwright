//! What one card cost, by round, as the daemon lays it out
//! (`GET /api/board/items/{id}/cost`): written down, and worked out not
//! at all here. Counted, never scored.

use serde_json::Value;

use crate::args::Args;
use crate::client::{Client, amount, encode, grouped, table, took};
use crate::work::{list, show, text};

/// `board cost <card>`.
pub async fn cost(client: &Client, args: &Args) -> Result<(), String> {
    let id = args.need(2, "the card")?;
    let laid = client.get(&format!("/api/board/items/{}/cost", encode(id))).await?;
    show(client, &laid, || written(id, &laid));
    Ok(())
}

fn number(value: &Value, key: &str) -> u64 {
    value[key].as_u64().unwrap_or(0)
}

/// What some turns cost: what those that said came to, and how many
/// said nothing of it, which is no cost of nought.
fn cost_of(spent: &Value) -> String {
    let (turns, unsaid) = (number(spent, "turns"), number(spent, "cost_unsaid"));
    let said = amount(spent["cost_usd"].as_f64().unwrap_or(0.0), "usd");
    match unsaid {
        0 => said,
        _ if unsaid == turns => "cost not reported".to_string(),
        unsaid => format!("{said} (and {unsaid} not reported)"),
    }
}

/// What some turns came to, in a line.
fn spent(spent: &Value) -> String {
    let turns = number(spent, "turns");
    let mut said = format!(
        "{turns} {}, {} tokens sent and {} written, {}, {} at work",
        if turns == 1 { "turn" } else { "turns" },
        grouped(number(spent, "sent")),
        grouped(number(spent, "wrote")),
        cost_of(spent),
        took(spent["took_ms"].as_i64().unwrap_or(0)),
    );
    match number(spent, "unsaid") {
        0 => {}
        unsaid if unsaid == turns => said.push_str("; none of them said what it took"),
        unsaid => said.push_str(&format!("; {unsaid} of them said nothing of what it took")),
    }
    said
}

/// What was decided at the end of a round, in the words of the card's
/// history where it says so.
fn decided(decided: &Value) -> String {
    let came = match text(decided, "came") {
        "passed" => "passed",
        "back" => "sent back",
        "left" => "left for a person",
        _ => return "under way: nothing decided yet".to_string(),
    };
    match text(decided, "said") {
        "" => came.to_string(),
        said => format!("{came}: {said}"),
    }
}

fn written(id: &str, laid: &Value) -> String {
    let rounds = list(&laid["rounds"]);
    if rounds.is_empty() {
        return format!("no agent has taken a turn on {id}");
    }
    let mut lines = vec![format!(
        "{id}: {} {}, {}",
        rounds.len(),
        if rounds.len() == 1 { "round" } else { "rounds" },
        spent(&laid["spent"])
    )];
    // Turns from before that name no asking are put in rounds by the order they were written.
    match number(laid, "by_order") {
        0 => {}
        placed => lines.push(format!(
            "({placed} {} from before {} nothing of what began {}, and {} put in rounds by the order {} written)",
            if placed == 1 { "turn" } else { "turns" },
            if placed == 1 { "says" } else { "say" },
            if placed == 1 { "it" } else { "them" },
            if placed == 1 { "is" } else { "are" },
            if placed == 1 { "it was" } else { "they were" },
        )),
    }
    for round in rounds {
        lines.push(String::new());
        lines.push(format!("round {}: {}", number(round, "n"), decided(&round["decided"])));
        let rows: Vec<Vec<String>> = list(&round["turns"])
            .iter()
            .map(|turn| {
                let figures = if turn["says"] == true {
                    vec![
                        format!("{} sent", grouped(number(turn, "sent"))),
                        format!("{} written", grouped(number(turn, "wrote"))),
                        match turn["cost_said"] == false {
                            true => "cost not reported".to_string(),
                            false => amount(turn["cost_usd"].as_f64().unwrap_or(0.0), "usd"),
                        },
                    ]
                } else {
                    vec!["said nothing of what it took".to_string(), String::new(), String::new()]
                };
                let chose = match (turn["kept_from"] == true, text(turn, "chose")) {
                    (true, _) => "(asked in a channel kept from you)".to_string(),
                    (false, chose) => chose.to_string(),
                };
                let at_work = turn["took_ms"].as_i64().map_or_else(|| "-".to_string(), took);
                [
                    vec![
                        format!("  {}", text(turn, "agent")),
                        text(turn, "does").to_string(),
                        text(turn, "status").to_string(),
                        chose,
                    ],
                    figures,
                    vec![at_work],
                ]
                .concat()
            })
            .collect();
        if !rows.is_empty() {
            lines.push(table(&rows));
        }
        lines.push(format!("  {}", spent(&round["spent"])));
    }
    lines.join("\n")
}
