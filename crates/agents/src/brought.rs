//! The blueprints a package brings, as a layer of the library: while the
//! package is installed its blueprints are in the library under
//! `<package>/<name>`, and when it is removed they are gone again. What a
//! manifest says of them is read with the package (`extensions::brought`).

use conductor::Library;

use crate::Agents;

/// The blueprints of the packages installed on this device, as a layer of
/// the library. It asks each time, so a package installed or removed while
/// the daemon runs is found or gone at the next lookup.
pub struct Brought {
    agents: Agents,
}

impl Brought {
    pub(crate) fn new(agents: Agents) -> Self {
        Self { agents }
    }
}

impl Library for Brought {
    fn load(&self, name: &str, _project: Option<&str>) -> Option<String> {
        let (package, _) = name.split_once('/')?;
        let extension = self.agents.extension(package)?;
        // A file a blueprint of the package's links is found as one of its texts.
        extension.blueprints.get(name).or_else(|| extension.prompts.get(name)).cloned()
    }

    fn names(&self, _project: Option<&str>) -> Vec<String> {
        self.agents.extensions().iter().flat_map(|extension| extension.blueprints.keys().cloned()).collect()
    }

    /// A package's blueprints come with it, and are another when it is.
    fn ships(&self, name: &str, project: Option<&str>) -> bool {
        self.load(name, project).is_some()
    }
}
