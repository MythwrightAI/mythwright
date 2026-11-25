//! What the factory's bot takes is the command line's own commands, in the
//! same words: each line of what it takes (`api::commands::TAKES`) is
//! written in this program's help as it is, and each flag the bot reads is
//! one this program reads for that command. So the two cannot come apart:
//! a command changed here and not there, or there and not here, fails.

use crate::{run_in, scratch};

/// Whitespace made one space, as a line is read.
fn plain(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[tokio::test]
async fn every_command_the_bot_takes_is_one_of_this_programs_in_the_same_words_and_flags() {
    let dir = scratch("bot-takes");
    // No daemon is asked: what is refused for a flag is refused before anything is sent.
    let nowhere = "http://127.0.0.1:1";
    let help = plain(&run_in(&dir, nowhere, &[], &["help"]).await.ok());
    for (written, _) in api::commands::TAKES {
        // Said in the help, or in the help of its command, which writes each of its forms out whole.
        let mut said = help.clone();
        for word in
            written.split(' ').take(2).filter(|word| word.chars().all(|letter| letter.is_ascii_lowercase()))
        {
            let topic = run_in(&dir, nowhere, &[], &["help", word]).await;
            said.push(' ');
            said.push_str(&plain(&topic.out.replace(&format!("{} ", brand::CLI), "")));
        }
        assert!(said.contains(written), "the bot takes `{written}`, which `help` does not say");
        // Each flag, given with something after it, is one the command reads.
        let words: Vec<&str> = written.split(' ').collect();
        let fixed: Vec<&str> =
            words.iter().copied().take_while(|word| !word.starts_with(['<', '['])).collect();
        let flags: Vec<&str> = words
            .iter()
            .filter_map(|word| word.strip_prefix("[--"))
            .map(|flag| flag.trim_end_matches(']'))
            .collect();
        if flags.is_empty() {
            continue;
        }
        let mut asked: Vec<String> = fixed.iter().map(|word| word.to_string()).collect();
        asked.extend(words.iter().filter(|word| word.starts_with('<')).map(|_| "one".to_string()));
        for flag in &flags {
            asked.extend([format!("--{flag}"), "x=y".to_string()]);
        }
        let asked: Vec<&str> = asked.iter().map(String::as_str).collect();
        let ran = run_in(&dir, nowhere, &[], &asked).await;
        assert!(!ran.err.contains("takes no `--"), "`{}`: {}", asked.join(" "), ran.err);
    }
    let _ = std::fs::remove_dir_all(dir);
}
