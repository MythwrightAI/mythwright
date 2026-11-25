//! Doing the actions packages declare.
//!
//! A package's actions are there while the package is installed and gone
//! when it is removed, and packages are installed and removed while the
//! daemon runs. So they are not registered with the conductor once, as the
//! actions built into the application are: [`Offered`] is asked for them
//! each time, the way [`crate::brought`] is asked for a package's
//! blueprints.
//!
//! Doing one is running the package's program, once, with the arguments
//! the package said, and reading what it printed with the package's own
//! expressions (`extensions::performed`, offered here under the names it
//! always had). Nothing is run through a shell. It needs
//! `extensions:<package>` and not `shell`: all it can run is that one
//! program, in the ways its package wrote down.

use std::sync::Arc;

use blueprint::ActionSpec;
use conductor::{Action, ActionCx, ActionError, ActionResult, Brought};
use hosting::Hosting;
use kernel::{BoxFut, Fields};
use serde_json::json;

pub(crate) use extensions::performed::{At, once, user};

use crate::declared::Declared;
use crate::{Agents, Extension};

/// The actions of the packages installed on this device.
pub struct Offered {
    agents: Agents,
    hosting: Hosting,
}

impl Offered {
    pub(crate) fn new(agents: Agents, hosting: Hosting) -> Self {
        Self { agents, hosting }
    }
}

impl Brought for Offered {
    fn action(&self, name: &str) -> Option<Arc<dyn Action>> {
        let (package, action) = name.split_once('.')?;
        let extension = self.agents.extension(package)?;
        let declared = extension.actions.get(action)?.clone();
        Some(Arc::new(Performed { extension, declared, hosting: self.hosting.clone() }))
    }

    fn specs(&self) -> Vec<ActionSpec> {
        let installed = self.agents.extensions();
        installed
            .iter()
            .flat_map(|extension| extension.actions.values())
            .map(|action| action.spec.clone())
            .collect()
    }

    fn absent(&self, name: &str, once_here: bool) -> Option<String> {
        let (package, action) = name.split_once('.')?;
        // A name the application's own actions are under is not a package's.
        if self.agents.is_taken(package) {
            return None;
        }
        let cli = brand::CLI;
        if let Some(installed) = self.agents.extension(package) {
            let has: Vec<&str> = installed.actions.keys().map(String::as_str).collect();
            return Some(if has.is_empty() {
                format!("the extension package `{package}` brings no actions, so there is no `{name}`")
            } else {
                format!(
                    "the extension package `{package}` has no action called `{action}`; it has: {}",
                    has.join(", ")
                )
            });
        }
        if once_here {
            return Some(format!(
                "`{name}` came with the extension package `{package}`, which is not installed on this device \
                 any more. Install it again (`{cli} extension add {package}`), or stop this instance"
            ));
        }
        let on_offer = self.agents.offered().iter().any(|offered| offered.name == package);
        on_offer.then(|| {
            format!(
                "`{name}` comes with the extension package `{package}`, which is not installed on this \
                 device; install it with `{cli} extension add {package}`"
            )
        })
    }
}

/// One action of one package, as the conductor runs it.
struct Performed {
    extension: Arc<Extension>,
    declared: Arc<Declared>,
    hosting: Hosting,
}

impl Action for Performed {
    fn spec(&self) -> &ActionSpec {
        &self.declared.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        let (extension, declared, hosting) =
            (self.extension.clone(), self.declared.clone(), self.hosting.clone());
        Box::pin(async move {
            let daemon = json!({ "url": hosting.address().unwrap_or_default(), "user": user() });
            let says = |problem: String| ActionError::new(format!("`{}`: {problem}", declared.spec.name));
            let mut scope = declared.before(&args, &daemon).map_err(says)?;
            // Done first, and with the arguments of this one that it takes
            // too: what it gave out is there for this one to decide by.
            let log = |stream: char, line: &str| cx.log(stream, line);
            let at = At {
                workdir: cx.workdir(),
                env: cx.env(),
                secrets: cx.secrets(),
                asker: &cx.principal().name,
                may_give: cx.principal().can("shell"),
                cancel: cx.cancel(),
                log: &log,
            };
            if let Some(first) = declared.first().and_then(|first| extension.actions.get(first)) {
                let asked = first.before(&args, &daemon).map_err(says)?;
                let gave = once(&extension, first, &hosting, &at, asked).await?;
                scope.insert("first".to_string(), gave);
            }
            once(&extension, &declared, &hosting, &at, scope).await
        })
    }
}
