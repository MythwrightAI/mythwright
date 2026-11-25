//! The events the gateway emits: the devices of a fleet being invited,
//! let in, refused, coming and going. No invitation and no key is in any
//! of them; a key is named only by its short name (`id`).

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let device = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary).field("device", String, "the device's name")
    };
    let hub = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary).field("hub", String, "the address of the hub")
    };
    let all =
        vec![
            device("device.invited", "An invitation was made for a device to join this fleet.").field(
                "expires",
                Int,
                "when the invitation stops working, in milliseconds since 1970",
            ),
            device("device.added", "A device was let into this fleet: it redeemed its invitation.").field(
                "id",
                String,
                "the short name of the key it proved",
            ),
            device("device.removed", "A device was taken out of this fleet."),
            EventSpec::new("device.refused", "A device that shook hands with this hub was not let in.")
                .field("id", String, "the short name of the key it proved")
                .field("from", String, "the address it came from")
                .field("why", String, "what it was told"),
            device("device.connected", "A device of this fleet opened its link."),
            device("device.disconnected", "A device of this fleet lost its link."),
            device("device.changed", "What a device says of itself changed."),
            hub("device.joined", "This device joined a hub's fleet.").field(
                "id",
                String,
                "the short name of the key the hub proved",
            ),
            hub("device.left", "This device left a hub's fleet."),
            hub("device.unjoined", "This device is no longer joined to the hub it was, and was not asked.")
                .field("why", String, "why, and what to do"),
            EventSpec::new("device.unproven", "What answers where this device's hub was is not its hub.")
                .field("why", String, "what was found, and what to do"),
        ];
    // Who may be told of each: whoever may see the fleet's devices.
    blueprint::events::heard(all, &[("device", "fleet:read")])
}
