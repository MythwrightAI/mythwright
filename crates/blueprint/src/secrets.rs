//! A secret, as a blueprint names one.
//!
//! A blueprint never holds a secret. Where a program it runs needs one, the
//! variable the program reads it from is given the secret's name in place
//! of a value:
//!
//! ```yaml
//! env:
//!   STRIPE_KEY: { secret: stripe }
//! ```
//!
//! That is the only place one can be named: the environment of the
//! blueprint or of a step. It is not something an expression can read, so
//! it cannot be put in an argument, in `state`, in an event or in a line
//! of a log by anything written in a blueprint.
//!
//! The name is text like any other, and may be worked out: the secret of
//! this profile (`"mail-${ profile.name }"`), or the one a param
//! says (`"${params.key}"`). A name that comes to the empty text names
//! none, and the variable is not set: a blueprint that can be told which
//! key to use is told none that way.

use serde_json::Value;

use crate::syntax::Span;
use crate::validate::Validator;

/// The secret `value` names, where it is `{ secret: <name> }` and nothing
/// more.
pub fn named(value: &Value) -> Option<&str> {
    let mapping = value.as_object().filter(|mapping| mapping.len() == 1)?;
    mapping.get("secret")?.as_str()
}

impl Validator<'_> {
    /// Holds a variable of an environment to being text, or a secret's
    /// name. Says whether it is something to look at for what it reads.
    pub(crate) fn variable(&mut self, name: &str, value: &Value, span: Span) -> bool {
        if named(value).is_some() {
            self.needs("secrets:use", span);
            // The name is read like any text: what it reads has to be there.
            return true;
        }
        if value.is_object() || value.is_array() {
            self.error(
                "bad-env",
                format!(
                    "`{name}` in `env` is text, a number, or a secret's name written as `{{ secret: <name> }}`"
                ),
                span,
            );
            return false;
        }
        true
    }
}
