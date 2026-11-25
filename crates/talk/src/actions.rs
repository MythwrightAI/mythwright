//! What a blueprint can do in a channel: make sure of one, say something
//! there, and take part under a name of its own; and tell somebody
//! something, in their inbox ([`crate::telling`]).
//!
//! Each acts as the principal the instance runs as, and asks as that
//! principal whether the channel is one it may write in
//! ([`Channels::may_write`]): a channel kept to its members is not there
//! for a blueprint whose applier is not one of them. They are added to a
//! registry where they always were, among the studio's own
//! (`studio::Studio::register`): the schema kept for editors lists the
//! actions in the order they are added.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

use crate::Channels;

pub(crate) fn add<F, Fut>(registry: &mut Registry, channels: &Channels, spec: ActionSpec, run: F)
where
    F: Fn(Channels, ActionCx, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let channels = channels.clone();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(run(channels.clone(), cx, args))
    });
}

pub(crate) fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

pub(crate) fn register(registry: &mut Registry, channels: &Channels) {
    add(
        registry,
        channels,
        ActionSpec::new("channel.ensure", "Make sure a channel exists.")
.example(ENSURE_ASKED)
            .making("a channel", "name")
            .required("name", T::String, "the channel's name")
            .arg("otherwise", T::String, "what it is called where `name` says nothing")
            .arg("project", T::String, "the project it belongs to")
            .arg("topic", T::String, "what it is for")
            .arg(
                "kept",
                T::Bool,
                "kept to its members: read by whoever is in it and nobody else. `name` is then what it is called, \
                 which another may be called too, and it is found again as the kept channel of its project that \
                 is called so and has whoever applied this in it. Refused where an open channel has the name",
            )
            .arg("alone", T::Bool, "of a kept one: its agents are asked nowhere else")
            .capability("channels:write")
            .output("id", "the channel's id")
            .output("name", "its name: for one kept to its members, its own id")
            .output("speaker", "what the factory's bot is called now: the name what a blueprint says there is said under, and by which it is written to")
            .output(
                "archived",
                "whether the channel there is archived: it is found and not made again, nor given back, and nothing \
                 is said in it until whoever may write there gives it back",
            )
            .idempotent(),
        |channels, cx, args| async move {
            let name = text(&args, "name").or_else(|| text(&args, "otherwise")).unwrap_or_default();
            let project = text(&args, "project").or_else(|| cx.project().map(str::to_string));
            let topic = text(&args, "topic").unwrap_or_default();
            let said = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let asked = crate::Asked {
                name: &name,
                project: project.as_deref(),
                topic: &topic,
                kept: said("kept"),
                alone: said("alone"),
                members: &[],
            };
            let channel = channels.ensure_as(asked, cx.principal()).await?;
            let speaker = channels.speaker.name();
            let archived = channel.archived.is_some();
            Ok(json!({ "id": channel.id, "name": channel.name, "speaker": speaker, "archived": archived }))
        },
    );

    add(
        registry,
        channels,
        ActionSpec::new("channel.post", "Post a message to a channel.")
            .example(POST_ASKED)
            .required("channel", T::String, "the channel's name or id")
            .required("body", T::String, "the message")
            .arg(
                "as",
                T::String,
                "the bot to speak as (default: the factory's own, with this blueprint as what said it)",
            )
            .arg("reply_to", T::String, "the id of the message this answers")
            .arg("thread", T::String, "the id of a message to say it under, in that message's thread")
            .arg("view", T::String, "a page to show with it, as HTML: a chart, a diagram, anything drawn")
            .arg(
                "detail",
                T::String,
                "more than the message says, for whoever opens it: the whole of an error, say",
            )
            .arg(
                "about_run",
                T::String,
                "the id of a run the message is about, one that failed say: its page is opened from the message at the step that failed",
            )
            .capability("channels:write")
            .output("id", "the message's id")
            // The same done twice: a step that is done again because the
            // daemon stopped in the middle of it finds what it said, by
            // the run, the step and the element kept on the message.
            .idempotent(),
        |channels, cx, args| async move {
            let (channel, body) =
                (text(&args, "channel").unwrap_or_default(), text(&args, "body").unwrap_or_default());
            let (reply_to, thread) = (text(&args, "reply_to"), text(&args, "thread"));
            // Which step this is stays on the message, whoever it is said
            // as: the name is a bot's, and this is what was speaking.
            let said_by = crate::SaidBy {
                blueprint: cx.blueprint().to_string(),
                instance: cx.instance().to_string(),
                run: cx.run().to_string(),
                step: cx.step().to_string(),
                at: cx.at(),
                doing: Some(cx.doing().to_string()),
                detail: text(&args, "detail").filter(|detail| !detail.trim().is_empty()),
                about_run: text(&args, "about_run").filter(|run| !run.trim().is_empty()),
            };
            // Asked as whoever the instance runs as, for all of what follows.
            let reader = channels.may_write(&channel, cx.principal()).await?;
            if cx.again()
                && let Some(said) = channels.said_already(&reader, &said_by).await?
            {
                return Ok(json!({ "id": said.id }));
            }
            // A page to show with it is kept as something attached, for
            // whoever is speaking, and goes with the message like any other.
            let mut attached = Vec::new();
            if let Some(page) = text(&args, "view").filter(|page| !page.trim().is_empty()) {
                let speaker =
                    crate::channels::bot_actor(&text(&args, "as").unwrap_or_else(|| channels.speaker.name()));
                let found = channels.get(&reader, &channel).await?;
                let kept = channels
                    .attachments
                    .add_as(
                        &reader,
                        &found.id,
                        &speaker,
                        "view.html",
                        Some(crate::attachments::VIEW),
                        page.into_bytes(),
                    )
                    .await?;
                attached.push(kept.id);
            }
            let place = crate::Place {
                reply_to: reply_to.as_deref(),
                thread: thread.as_deref(),
                attachments: &attached,
                said_by: Some(&said_by),
                ..crate::Place::default()
            };
            let message = match text(&args, "as") {
                Some(bot) => channels.post_bot(&reader, &channel, &bot, &body, place).await?,
                // Said by the factory's one bot: a blueprint is no member
                // of a channel, and a dozen of them are not a dozen voices.
                None => channels.post_speaker(&reader, &channel, &body, place).await?,
            };
            Ok(json!({ "id": message.id }))
        },
    );

    add(
        registry,
        channels,
        ActionSpec::new("channel.join", "Take part in a channel as a bot, under a name of its own.")
            .example(JOIN_ASKED)
            .making("a bot", "name")
            .required("channel", T::String, "the channel's name or id")
            .required("name", T::String, "what the bot is called; people address it as @name")
            .capability("channels:write")
            .output("actor", "what the bot is known as in the channel")
            .output("called", "what it is named by after `@`, as a message's `mentions` say it")
            .idempotent(),
        |channels, cx, args| async move {
            let (channel, name) =
                (text(&args, "channel").unwrap_or_default(), text(&args, "name").unwrap_or_default());
            let actor = crate::channels::bot_actor(&name);
            if actor == "bot:" {
                return Err(ActionError::new("a bot needs a name with at least one letter or digit"));
            }
            let reader = channels.may_write(&channel, cx.principal()).await?;
            channels.join(&reader, &channel, &actor, &name).await?;
            let called = actor.strip_prefix("bot:").unwrap_or(&actor);
            Ok(json!({ "actor": actor, "called": called }))
        },
    );

    add(
        registry,
        channels,
        ActionSpec::new("channel.leave", "Stop taking part in a channel as a bot.")
            .example(LEAVE_ASKED)
            .required("channel", T::String, "the channel's name or id")
            .required("name", T::String, "the bot's name")
            .capability("channels:write")
            .idempotent(),
        |channels, cx, args| async move {
            let (channel, name) =
                (text(&args, "channel").unwrap_or_default(), text(&args, "name").unwrap_or_default());
            let reader = channels.may_write(&channel, cx.principal()).await?;
            channels.leave(&reader, &channel, &crate::channels::bot_actor(&name)).await?;
            Ok(json!({}))
        },
    );

    add(
        registry,
        channels,
        ActionSpec::new("inbox.tell", "Tell somebody something: a line in their inbox, from the factory's bot.")
.example(TELL_ASKED)
            .required(
                "to",
                T::Any,
                "whom, one or a list: `applier`, whoever applied this (a key is its maker); an actor \
                 (`user:<id>`, `agent:<name>`); or a handle, with or without its `@`, which tells everybody it \
                 names. One that names nobody, and a bot, are refused, and nobody is then told",
            )
            .required("says", T::String, "the words")
            .arg("why", T::String, "why, in one word of the blueprint's own, such as `asked` (default: `told`)")
            .arg(
                "about",
                T::String,
                "the id of what it is about: a card, a record, a channel, something running. Told again of the \
                 same thing for the same `why`, the line there is is raised and says the latest; about nothing, \
                 each telling is a line of its own",
            )
            .arg("place", T::String, "the id of what opens it, where that is not what it is about")
            .arg(
                "once",
                T::String,
                "which telling this is, in words of the blueprint's own: told again with the same words, of the \
                 same thing for the same `why`, by any run, nobody is told twice. Where it is not said, the same \
                 step done again is what tells nothing",
            )
            .arg(
                "parts",
                T::List,
                "what is shown with the words, each a part of its own kind: addresses to follow, or the cards of \
                 a board to be passed or failed where the line is read. Told again, the line says the latest",
            )
            .within("parts", kernel::parts::INSIDE)
            .capability(crate::inbox::WRITE)
            .output("told", "each actor the line is in the inbox of")
            .output("passed", "each that was told nothing, having no inbox: an agent the factory starts")
            // The same done twice: the line keeps which thing done under
            // which step of which run said it, and that one done again
            // finds it and tells nothing.
            .idempotent(),
        |channels, cx, args| async move {
            let to = match args.get("to") {
                Some(Value::String(one)) => vec![one.clone()],
                Some(Value::Array(several)) => {
                    let words: Option<Vec<String>> =
                        several.iter().map(|word| word.as_str().map(str::to_string)).collect();
                    words.ok_or_else(|| {
                        ActionError::new("`to` is whom to tell, each a word: an actor, a handle or `applier`")
                    })?
                }
                _ => {
                    return Err(ActionError::new(
                        "`to` is whom to tell, one or a list: an actor, a handle or `applier`",
                    ));
                }
            };
            let asked = crate::telling::Telling {
                to,
                says: text(&args, "says").unwrap_or_default(),
                why: text(&args, "why").unwrap_or_else(|| crate::telling::TOLD.to_string()),
                about: text(&args, "about").unwrap_or_default(),
                place: text(&args, "place").unwrap_or_default(),
                said_by: match text(&args, "once").filter(|once| !once.trim().is_empty()) {
                    Some(once) => format!("once {once}"),
                    None => format!("{} {} {}", cx.run(), cx.step(), cx.doing()),
                },
                parts: kernel::parts::read(args.get("parts").unwrap_or(&Value::Null), cx.project())
                    .map_err(ActionError::new)?,
            };
            let came = channels.tell(cx.principal(), asked).await?;
            Ok(json!({ "told": came.told, "passed": came.passed }))
        },
    );
    crate::discussed::register(registry, channels);
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const POST_ASKED: &str = "\
blueprint: said-hello
permissions: [channels:write]

steps:
  hello:
    do: channel.post
    with: { channel: general, body: The build is done. }
";
const ENSURE_ASKED: &str = "\
blueprint: releases
permissions: [channels:write]

steps:
  channel:
    do: channel.ensure
    with: { name: releases, topic: \"What is shipped, as it ships.\" }

  said:
    do: channel.post
    with: { channel: releases, body: Shipped. }
";
const JOIN_ASKED: &str = "\
blueprint: builder-bot
permissions: [channels:write]

steps:
  join:
    do: channel.join
    with: { channel: general, name: builder }

  hello:
    do: channel.post
    with: { channel: general, as: builder, body: The builder is here. }
";
const LEAVE_ASKED: &str = "\
blueprint: builder-gone
permissions: [channels:write]

steps:
  leave:
    do: channel.leave
    with: { channel: general, name: builder }
";
const TELL_ASKED: &str = "\
blueprint: tell-me
permissions: [inbox:write]

steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: The nightly copy is made. }
";
