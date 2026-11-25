//! The `shell` action: run a command, capture what it prints, report how it ended.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{Action, ActionCx, ActionError, ActionResult};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

use crate::Hosting;
use crate::proc::{KillOnDrop, Launch, Proc, Tail};
use crate::watch::Watch;

/// Lines of each stream kept for the step's output; the full text is in the log.
const KEEP_LINES: usize = 40;

/// What `cli` means, said the same wherever a command is described.
pub(crate) const CLI_ARG: &str =
    "arguments for this program's own command line, run without a shell: what `$FACTORY_CLI` is to a script";

pub(crate) struct Shell {
    spec: ActionSpec,
    hosting: Hosting,
}

impl Shell {
    pub fn new(hosting: Hosting) -> Self {
        let spec = ActionSpec::new("shell", "Run a command and wait for it to finish.")
            .example(SHELL_ASKED)
            .runs("run")
            .runs("cmd")
            .runs("cli")
            .arg("run", T::String, crate::script::RUN_ARG)
            .arg("cmd", T::List, "a program and its arguments, run without a shell")
            .arg("cli", T::List, CLI_ARG)
            .arg("cwd", T::String, "directory to run in, relative to the project")
            .arg("env", T::Object, "extra environment variables")
            .arg("clear_env", T::Bool, "start from an almost empty environment")
            .arg("ok_codes", T::List, "exit codes that count as success (default: just 0)")
            .capability("shell")
            .output("code", "the exit code")
            .output("stdout", "the last lines of standard output")
            .output("stderr", "the last lines of standard error")
            .output("duration_ms", "how long it ran")
            // Stood in, it ran nothing: a command that ended with 0 and said nothing.
            .stands(serde_json::json!({ "code": 0, "stdout": "", "stderr": "", "duration_ms": 0 }));
        Self { spec, hosting }
    }
}

impl Action for Shell {
    fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        let hosting = self.hosting.clone();
        Box::pin(async move {
            let launch =
                launch_from(&cx, &args, &hosting, &format!("{}.{}.{}", cx.run(), cx.step(), cx.attempt()))?;
            let proc = launch.start()?;
            // From here on a restart can find the process again.
            cx.checkpoint(proc.saved()).await?;
            follow(cx, args, proc, hosting.watch()).await
        })
    }

    fn resume(&self, cx: ActionCx, args: Fields, saved: Value) -> BoxFut<'static, ActionResult> {
        let hosting = self.hosting.clone();
        Box::pin(async move {
            let Some(proc) = Proc::adopt(&saved, hosting.watch()) else {
                return Err(ActionError::new(
                    "was interrupted by a restart and its process could not be found again",
                ));
            };
            cx.log('s', &format!("picked up process {} after a restart", proc.pid));
            follow(cx, args, proc, hosting.watch()).await
        })
    }
}

/// Builds a launch from the arguments shared by `shell` and services.
pub(crate) fn launch_from(
    cx: &ActionCx,
    args: &Fields,
    hosting: &Hosting,
    tag: &str,
) -> Result<Launch, ActionError> {
    let script = args.get("run").and_then(Value::as_str).map(str::to_string);
    let list = |key: &str| -> Option<Vec<String>> {
        args.get(key)
            .and_then(Value::as_array)
            .map(|items| items.iter().map(blueprint::expr::display).collect())
    };
    let argv = match (&script, list("cmd").filter(|cmd| !cmd.is_empty()), list("cli")) {
        (Some(_), None, None) => Vec::new(),
        (None, Some(cmd), None) => cmd,
        (None, None, Some(cli)) => crate::proc::own(&cli).map_err(ActionError::new)?,
        (None, None, None) => {
            return Err(ActionError::new(
                "needs `run: <script>`, `cmd: [program, args...]` or `cli: [args...]`",
            ));
        }
        _ => return Err(ActionError::new("give one of `run`, `cmd` and `cli`, not more")),
    };
    let base = cx.workdir();
    let cwd = match args.get("cwd").and_then(Value::as_str) {
        Some(relative) => base.join(relative),
        None => base,
    };
    if !cwd.is_dir() {
        return Err(ActionError::new(format!("the working directory {} does not exist", cwd.display())));
    }
    let mut env: BTreeMap<String, String> = cx.env().clone();
    for (key, value) in args.get("env").and_then(Value::as_object).into_iter().flatten() {
        env.insert(key.clone(), blueprint::expr::display(value));
    }
    // Last, so that nothing written in the step stands in for one.
    hosting.hand_over(cx.secrets(), &mut env).map_err(|err| ActionError::new(err.message))?;
    Ok(Launch {
        script,
        argv,
        cwd,
        env,
        clear_env: args.get("clear_env").and_then(Value::as_bool).unwrap_or(false),
        address: hosting.address(),
        profile: hosting.profile(),
        dir: hosting.run_dir(),
        tag: tag.replace(['/', '\\', ':'], "_"),
        handed: cx.secrets().clone(),
        wraps: hosting.wraps(),
    })
}

/// Follows a process to its end, relaying its output to the step's log.
async fn follow(cx: ActionCx, args: Fields, mut proc: Proc, watch: &Watch) -> ActionResult {
    let started = cx.clock().now_ms();
    let mut guard = KillOnDrop::new(proc.pid, cx.engine().clone());
    let mut tails = [Tail::new(&proc.out, 'o'), Tail::new(&proc.err, 'e')];
    let mut kept: [VecDeque<String>; 2] = [VecDeque::new(), VecDeque::new()];
    let mut relay = |tails: &mut [Tail; 2], flush: bool| {
        for (tail, kept) in tails.iter_mut().zip(kept.iter_mut()) {
            tail.read(flush, &mut |stream, line| {
                cx.log(stream, line);
                if kept.len() == KEEP_LINES {
                    kept.pop_front();
                }
                kept.push_back(line.to_string());
            });
        }
    };

    // Told when the output grows, then one read for whatever was written
    // before asking: in that order nothing falls between the two.
    let mut output = watch.files(&[&proc.out, &proc.err]).or_looking();
    relay(&mut tails, false);
    let code = loop {
        tokio::select! {
            code = proc.wait() => break code,
            () = output.next() => relay(&mut tails, false),
            () = cx.cancel().cancelled() => {
                proc.stop(Duration::from_secs(2)).await;
                relay(&mut tails, true);
                guard.disarm();
                proc.clean();
                return Err(ActionError::cancelled());
            }
        }
    };
    relay(&mut tails, true);
    guard.disarm();
    proc.clean();

    let duration_ms = cx.clock().now_ms() - started;
    let text = |lines: &VecDeque<String>| lines.iter().cloned().collect::<Vec<_>>().join("\n");
    let (stdout, stderr) = (text(&kept[0]), text(&kept[1]));
    let ok_codes: Vec<i64> = args
        .get("ok_codes")
        .and_then(Value::as_array)
        .map(|codes| codes.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_else(|| vec![0]);
    match code {
        Some(code) if ok_codes.contains(&i64::from(code)) => {
            Ok(json!({ "code": code, "stdout": stdout, "stderr": stderr, "duration_ms": duration_ms }))
        }
        Some(code) => {
            let last = stderr.lines().last().or_else(|| stdout.lines().last()).unwrap_or_default();
            let detail = if last.is_empty() { String::new() } else { format!(": {last}") };
            Err(ActionError::new(format!("exited with code {code}{detail}")).with_data(
                json!({ "code": code, "stdout": stdout, "stderr": stderr, "duration_ms": duration_ms }),
            ))
        }
        // Killed by a signal, or its status was lost. That is not a zero.
        None => Err(ActionError::new("ended without an exit code (killed, or its status was lost)")
            .with_data(
                json!({ "code": null, "stdout": stdout, "stderr": stderr, "duration_ms": duration_ms }),
            )),
    }
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const SHELL_ASKED: &str = "\
blueprint: tests
permissions: [shell]

steps:
  test:
    do: shell
    with: { cmd: [make, test], ok_codes: [0, 2] }
";
