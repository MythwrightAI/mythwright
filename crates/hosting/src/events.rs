//! The events hosting emits: what becomes of a service and of a router.

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let service = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary).field("service", String, "the service's name")
    };
    let router = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary).field("router", String, "the router's name")
    };
    let all = vec![
        service("service.started", "A service's program was started.")
            .maybe("pid", Int, "the program's process")
            .maybe("port", Int, "the port it was told to listen on"),
        service("service.healthy", "A service answered its health check, or has none to answer."),
        service("service.unhealthy", "A service stopped answering its health check.").field(
            "reason",
            String,
            "what the check found",
        ),
        service("service.exited", "A service's program ended by itself.")
            .maybe("code", Int, "the code it ended with, where there was one")
            .maybe("while", String, "what it was in the middle of, when that says more"),
        service("service.restarting", "A service that ended is to be started again.")
            .field("attempt", Int, "which restart this is")
            .field("in_ms", Int, "how long until it is started"),
        service("service.stopped", "A service was stopped, as it was asked to be."),
        service("service.failed", "A service was given up on.").field("reason", String, "why"),
        router("router.listening", "A router began listening.")
            .field("host", String, "the address it listens on")
            .field("port", Int, "the port it listens on"),
        router("router.routed", "A router was told where to send new connections.").field(
            "backends",
            List,
            "each backend and its share",
        ),
        router("router.failed", "A router could not be brought back at start.").field("error", String, "why"),
        EventSpec::new(
            "secret.set",
            "A secret was set, or set again. Its name is said and never what it is.",
        )
        .field("name", String, "the secret's name")
        .field("by_name", String, "who set it"),
        EventSpec::new("secret.removed", "A secret was taken away.")
            .field("name", String, "the secret's name")
            .field("by_name", String, "who took it away"),
        EventSpec::new("system.degraded", "Something this device does less well than it could.")
            .field("what", String, "what it is")
            .field("reason", String, "why it is so here")
            .field("effect", String, "what is different because of it"),
    ];
    // Who may be told of each: whoever may see this device's services.
    blueprint::events::heard(
        all,
        &[
            ("service", "fleet:read"),
            ("router", "fleet:read"),
            ("system.degraded", "fleet:read"),
            // That there is one, by name, is known to whoever may use them.
            ("secret", "secrets:use"),
        ],
    )
}
