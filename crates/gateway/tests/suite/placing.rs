//! Choosing a device for a piece of work: the one with room that is least
//! busy for its size, with what was picked a moment ago counted against
//! the device it was picked for.

use std::time::Duration;

use conductor::StepStatus;
use gateway::place::{Asked, FRESH, Standing, choose};

use crate::{Rig, step};

/// A device that is connected, with these figures and nothing picked.
fn device(name: &str, cpus: usize, load1: f64, free_mem_mb: u64) -> Standing {
    Standing {
        device: name.into(),
        here: false,
        connected: true,
        os: "Linux".into(),
        arch: "x86_64".into(),
        cpus,
        load1: Some(load1),
        free_mem_mb,
        free_disk_mb: Some(50_000),
        instances: 0,
        picked: 0,
        picked_mem_mb: 0,
    }
}

fn chosen(standing: &[Standing], asked: &Asked) -> String {
    choose(standing, asked).map(|device| device.device.clone()).unwrap_or_else(|why| panic!("{why}"))
}

#[test]
fn of_the_devices_with_room_the_least_busy_for_its_size_is_chosen() {
    let anywhere = Asked { here: true, ..Asked::default() };
    // Busy is for its size: a load of 6 on 32 processors is less than a
    // load of 2 on 4.
    let fleet = [device("small", 4, 2.0, 8_000), device("large", 32, 6.0, 64_000)];
    assert_eq!(chosen(&fleet, &anywhere), "large");
    // As busy as each other: the one with more memory free, then by name.
    let fleet = [device("b", 8, 1.0, 8_000), device("c", 8, 1.0, 16_000), device("a", 8, 1.0, 8_000)];
    assert_eq!(chosen(&fleet, &anywhere), "c");
    assert_eq!(chosen(&fleet[..1], &anywhere), "b");
    let twins = [device("b", 8, 1.0, 8_000), device("a", 8, 1.0, 8_000)];
    assert_eq!(chosen(&twins, &anywhere), "a");

    // One that lacks what is asked is passed over, however idle it is.
    let fleet = [device("idle", 4, 0.0, 2_000), device("busy", 16, 12.0, 32_000)];
    assert_eq!(chosen(&fleet, &Asked { mem_mb: 4_000, ..anywhere.clone() }), "busy");
    assert_eq!(chosen(&fleet, &Asked { cpus: 8, ..anywhere.clone() }), "busy");
    // Only those named, when some are; and this device only when it will do.
    let among = Asked { among: Some(vec!["busy".into()]), ..anywhere.clone() };
    assert_eq!(chosen(&fleet, &among), "busy");
    let mut fleet = fleet.to_vec();
    fleet[0].here = true;
    assert_eq!(chosen(&fleet, &anywhere), "idle");
    assert_eq!(chosen(&fleet, &Asked { here: false, ..anywhere.clone() }), "busy");
    // A processor of another kind is another kind of device.
    fleet[0].arch = "aarch64".into();
    assert_eq!(chosen(&fleet, &Asked { arch: Some("X86_64".into()), ..anywhere.clone() }), "busy");
    assert_eq!(chosen(&fleet, &Asked { arch: Some("aarch64".into()), ..anywhere }), "idle");
}

#[test]
fn what_was_just_picked_for_a_device_counts_against_it_so_work_asked_for_together_is_spread() {
    let asked = Asked { mem_mb: 6_000, here: true, ..Asked::default() };
    let mut fleet = vec![device("one", 8, 0.5, 16_000), device("two", 8, 0.6, 16_000)];
    assert_eq!(chosen(&fleet, &asked), "one");
    // It was given that, and has not yet said so of itself: the next goes
    // to the other, though by what each last said `one` is still idler.
    fleet[0].picked = 1;
    fleet[0].picked_mem_mb = 6_000;
    assert_eq!(chosen(&fleet, &asked), "two");
    fleet[1].picked = 1;
    fleet[1].picked_mem_mb = 6_000;
    assert_eq!(chosen(&fleet, &asked), "one");
    // And the memory asked for is taken as gone: a third of that size
    // fits nowhere, and is not squeezed in.
    fleet[0].picked = 2;
    fleet[0].picked_mem_mb = 12_000;
    assert_eq!(chosen(&fleet, &asked), "two");
    fleet[1].picked = 2;
    fleet[1].picked_mem_mb = 12_000;
    let why = choose(&fleet, &asked).unwrap_err();
    assert!(why.contains("one has 16000 MB of memory free, less the 12000 MB just placed there"), "{why}");
}

#[test]
fn when_no_device_will_do_each_says_what_it_lacks() {
    let mut fleet = vec![
        device("gone", 64, 0.0, 128_000),
        device("small", 4, 0.0, 3_900),
        device("arm", 8, 0.0, 16_000),
        device("full", 8, 0.0, 16_000),
        device("quiet", 8, 0.0, 16_000),
    ];
    fleet[0].connected = false;
    fleet[2].arch = "aarch64".into();
    fleet[3].free_disk_mb = Some(900);
    fleet[4].free_disk_mb = None;
    let asked =
        Asked { mem_mb: 8_000, disk_mb: 10_000, arch: Some("x86_64".into()), here: true, ..Asked::default() };
    let why = choose(&fleet, &asked).unwrap_err();
    for part in [
        "no device has room for this (8000 MB of memory, 10000 MB of disk, x86_64)",
        "gone is not connected",
        "small has 3900 MB of memory free",
        "arm is aarch64",
        "full has 900 MB of disk free",
        "quiet does not say how much disk it has free",
    ] {
        assert!(why.contains(part), "{part}: {why}");
    }
    // Asked about none at all, it says that instead.
    let nobody = Asked { among: Some(vec!["elsewhere".into()]), ..asked.clone() };
    assert_eq!(choose(&fleet, &nobody).unwrap_err(), "none of elsewhere is a device of this fleet");
    let alone = [Standing { here: true, ..device("hub", 8, 0.0, 16_000) }];
    let away = Asked { here: false, ..Asked::default() };
    assert_eq!(choose(&alone, &away).unwrap_err(), "there is no other device in this fleet to give this to");
}

#[tokio::test]
async fn a_blueprint_picks_a_device_with_room_and_gives_a_step_to_it() {
    let rig = Rig::linked().await;
    // A device can be given work once it has said what it is: its first
    // word after linking. Until then the hub knows only that it is there.
    rig.until("the member to say what it is", || {
        rig.hub.gateway.hub().seen("builder-1").is_some_and(|seen| seen.info.is_some())
    })
    .await;
    // The hub knows of two: itself, and the one that joined.
    let listed = rig.hub.run("blueprint: who\nsteps:\n  all: { do: device.list }\n").await;
    let devices = step(&listed, "all").output["devices"].as_array().unwrap().clone();
    let names: Vec<(&str, bool)> = devices
        .iter()
        .map(|device| (device["device"].as_str().unwrap(), device["here"].as_bool().unwrap()))
        .collect();
    assert_eq!(names, [("hub", true), ("builder-1", false)]);
    assert!(devices.iter().all(|device| device["connected"] == true && device["cpus"].as_u64() > Some(0)));

    // Somewhere other than here with room: the step after it is done there.
    const PLACED: &str = "\
blueprint: placed
steps:
  where: { do: device.pick, with: { here: false, mem_mb: 1 } }
  there: { needs: [where], do: test.where, device: \"${ steps.where.out.device }\" }
";
    let run = rig.hub.run(PLACED).await;
    let place = &step(&run, "where").output;
    assert_eq!(
        (&place["device"], &place["here"], &place["considered"]),
        (&"builder-1".into(), &false.into(), &1.into())
    );
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Ok, "{run:?}");
    assert_eq!(there.output["device"], "member-box", "it ran on the device that was picked");

    // What was picked is counted against it for a while, and then is not:
    // by then the device has said for itself what it is doing.
    let standing = rig.hub.gateway.standing().await.unwrap();
    assert_eq!((standing[1].picked, standing[1].picked_mem_mb), (1, 1));
    assert_eq!(standing[0].picked, 0);
    rig.clock.advance(Duration::from_millis(FRESH as u64));
    assert_eq!(rig.hub.gateway.standing().await.unwrap()[1].picked, 0);

    // More than any device has is refused in a sentence, and the step fails.
    let run = rig
        .hub
        .run("blueprint: huge\nsteps:\n  where: { do: device.pick, with: { mem_mb: 900000000 } }\n")
        .await;
    let why = step(&run, "where").error.clone().unwrap_or_default();
    assert!(why.contains("no device has room for this (900000000 MB of memory)"), "{why}");
    assert!(why.contains("builder-1 has ") && why.contains("hub has "), "{why}");
}
