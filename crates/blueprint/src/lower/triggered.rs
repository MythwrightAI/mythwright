//! A handler, from the syntax tree to the model: what it is heard by
//! (`event`, `every`, `at`, `poll`), from where, and what it does.

use super::*;

impl Lowering {
    pub(super) fn triggers(&mut self, node: &Node) -> Vec<Trigger> {
        let Some(entries) = node.as_map() else {
            self.error("wrong-type", "`on` must be a mapping of handler name to handler", node.span);
            return Vec::new();
        };
        let mut triggers = Vec::new();
        for entry in entries {
            if self.ident(&entry.key, "handler", entry.key_span) {
                triggers.extend(self.trigger(entry));
            }
        }
        triggers
    }

    fn trigger(&mut self, entry: &Entry) -> Option<Trigger> {
        let name = &entry.key;
        let fields = self.fields(&entry.value, &format!("handler `{name}`"), &HANDLER)?;
        let sources: Vec<&Entry> =
            fields.iter().filter(|f| matches!(f.key.as_str(), "event" | "every" | "poll" | "at")).collect();
        let [only] = sources.as_slice() else {
            self.error(
                "missing-field",
                format!("handler `{name}` needs exactly one of `event: <topic>`, `every: <duration>`, `at: <time and zone>` or `poll: {{...}}`"),
                entry.key_span,
            );
            return None;
        };
        let source = match only.key.as_str() {
            "event" => TriggerSource::Event(self.string(&only.value, "`event`")?),
            "every" => TriggerSource::Every(self.millis(&only.value, "`every`")?),
            "at" => {
                // One with holes is read when its handler starts, from what they come to.
                let at = self.string(&only.value, "`at`")?;
                if let Err(why) =
                    crate::at::read(&at).map(drop).or_else(|why| at.contains("${").then_some(()).ok_or(why))
                {
                    self.error("bad-time", why, only.value.span);
                }
                TriggerSource::At(at)
            }
            _ => {
                let poll = self.fields(&only.value, "`poll`", &POLL)?;
                let every_ms = field(poll, "every").and_then(|node| self.millis(node, "`every`"));
                let action = field(poll, "do").and_then(|node| self.string(node, "`do`"));
                let (Some(every_ms), Some(action)) = (every_ms, action) else {
                    self.error(
                        "missing-field",
                        "`poll` needs `every: <duration>` and `do: <action>`",
                        only.key_span,
                    );
                    return None;
                };
                if every_ms < MIN_POLL_MS {
                    self.error(
                        "poll-too-fast",
                        format!(
                            "a poller may not run more often than every {}; subscribe to an event instead if you can",
                            duration::format(MIN_POLL_MS)
                        ),
                        only.value.span,
                    );
                }
                TriggerSource::Poll { every_ms, action, with: self.arguments(field(poll, "with")) }
            }
        };

        let scope = match field(fields, "scope")
            .and_then(|node| self.string(node, "`scope`").map(|s| (s, node.span)))
        {
            None => TriggerScope::Project,
            Some((scope, span)) => match scope.as_str() {
                "project" => TriggerScope::Project,
                "fleet" => TriggerScope::Fleet,
                "own" => TriggerScope::Own,
                "target" => TriggerScope::Target,
                other => {
                    self.error(
                        "bad-scope",
                        format!("unknown scope `{other}`; expected project, fleet, own or target"),
                        span,
                    );
                    TriggerScope::Project
                }
            },
        };

        let address = match fields.iter().find(|field| field.key == "for") {
            Some(entry) => {
                let (address, problems) = address::read(entry, &only.key);
                self.diagnostics.extend(problems);
                address
            }
            None => Map::new(),
        };

        // The body is a graph of steps, or a single `do`/`call` as shorthand.
        let body: Vec<&Entry> =
            fields.iter().filter(|f| matches!(f.key.as_str(), "steps" | "do" | "call")).collect();
        let flow = match body.as_slice() {
            [only] if only.key == "steps" => self.flow(&only.value, "`steps`"),
            [only] => {
                let target = self.string(&only.value, &format!("`{}`", only.key))?;
                let kind = if only.key == "do" { StepKind::Do(target) } else { StepKind::Call(target) };
                Flow { steps: vec![shorthand_step(kind, self.arguments(field(fields, "with")), entry)] }
            }
            _ => {
                self.error(
                    "missing-field",
                    format!("handler `{name}` needs exactly one of `steps`, `do` or `call`"),
                    entry.key_span,
                );
                return None;
            }
        };

        Some(Trigger {
            name: name.clone(),
            source,
            address,
            when: field(fields, "when").and_then(|node| self.cond(node, "`when`")),
            scope,
            debounce_ms: field(fields, "debounce").and_then(|node| self.millis(node, "`debounce`")),
            outside: match fields.iter().find(|field| field.key == "outside") {
                Some(entry) if only.key != "event" => {
                    self.error(
                        "outside-what",
                        "`outside` says where an event is said from, and goes with `event:` only".to_string(),
                        entry.key_span,
                    );
                    false
                }
                Some(entry) => self.boolean(&entry.value, "`outside`"),
                None => false,
            },
            turn: field(fields, "turn").and_then(|node| self.string(node, "`turn`")),
            flow,
            span: entry.key_span.to(entry.value.span),
        })
    }
}
