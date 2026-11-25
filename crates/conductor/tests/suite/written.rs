//! What a play kept, for whoever reads how it went: who began what was
//! written down and not yet begun, and everything it kept, written to a
//! file where a play came to a fault.

use crate::anywhere::{Place, Play};

impl Play {
    /// What began each thing that was written down and not yet begun,
    /// since the conductor was last started: its blueprint and who came
    /// to it. Only a measurement says, and what was measured before a
    /// stop went with that conductor.
    pub(crate) async fn begun(&self) -> Vec<(String, String, String)> {
        let query = journal::MetricQuery {
            name: "instance.begun".into(),
            group_by: vec!["blueprint".into(), "by".into(), "owed".into()],
            ..Default::default()
        };
        let label =
            |series: &journal::MetricSeries, key: &str| series.labels.get(key).cloned().unwrap_or_default();
        let measured = self.rig.journal.query_metrics(query).await.unwrap_or_default();
        measured
            .iter()
            .map(|series| (label(series, "blueprint"), label(series, "by"), label(series, "owed")))
            .collect()
    }

    /// Writes everything the play kept to a file, for whoever reads a
    /// fault: where the file is.
    pub(crate) async fn written_down(&self, name: &str, place: &Place) -> String {
        let mut lines = String::new();
        for event in self.rig.events("").await {
            let mut data = event.data.to_string();
            if data.len() > 200 {
                let cut = (0..=200).rev().find(|at| data.is_char_boundary(*at)).unwrap_or(0);
                data.truncate(cut);
            }
            lines.push_str(&format!(
                "{:>4} {:<18} {:<10} {:<12} {}\n",
                event.seq,
                event.topic,
                event.source.instance.as_deref().map_or("", |id| &id[id.len().saturating_sub(6)..]),
                event.source.step.clone().unwrap_or_default(),
                data
            ));
        }
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("anywhere");
        std::fs::create_dir_all(&dir).unwrap();
        let file: String =
            format!("{name}-{place}").chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
        let path = dir.join(format!("{file}.txt"));
        std::fs::write(&path, lines).unwrap();
        path.display().to_string()
    }
}
