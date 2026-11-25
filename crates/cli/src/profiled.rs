//! Which profile, a copy of the factory, a command is asked of.
//!
//! A profile is said by `--profile NAME`, or for every command typed in a
//! terminal by `<PREFIX>_PROFILE`. Before 0.1.0 a profile was called an
//! environment, and was said by `--env` and `<PREFIX>_ENV`, which a
//! person's own scripts may still say: both are taken for a while, and
//! each time one is, a line says that it is the old spelling and what the
//! word means now. The line matters: `--environment` is the environment
//! of a project's something is applied under, and `--env prod` beside it
//! goes to another daemon altogether.

use std::sync::Once;

use crate::args::Args;

/// The profile these arguments ask for, if they ask for one: the flag
/// before the variable, the new spelling before the old.
pub fn asked(args: &Args) -> Option<String> {
    if let Some(name) = args.value("profile") {
        return Some(name.to_string());
    }
    if let Some(name) = args.value("env") {
        old("`--env`", "`--profile`");
        return Some(name.to_string());
    }
    if let Some(name) = var("PROFILE") {
        return Some(name);
    }
    let name = var("ENV")?;
    old(&format!("`{}`", brand::env_var("ENV")), &format!("`{}`", brand::env_var("PROFILE")));
    Some(name)
}

/// Whether the profile is said by a flag, of either spelling.
pub fn flagged(args: &Args) -> Option<&str> {
    args.value("profile").or_else(|| args.value("env"))
}

fn var(name: &str) -> Option<String> {
    std::env::var(brand::env_var(name)).ok().filter(|value| !value.is_empty())
}

/// Says, once a run, that an old spelling was taken.
fn old(said: &str, now: &str) {
    static TOLD: Once = Once::new();
    TOLD.call_once(|| {
        eprintln!(
            "{said} is the old spelling of {now}: which copy of the factory a command goes to, its profile. The \
             word environment is kept for what a project has of its own (dev, staging, prod)."
        );
    });
}
