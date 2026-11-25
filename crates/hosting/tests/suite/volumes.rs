//! Which volume holds a path: a function of the path and the places
//! volumes are mounted, as text, so that a Windows drive is held here.

use hosting::{unplaced, volume};

/// What it is, the path, where volumes are mounted, whether on Windows,
/// and which of them holds it.
type Case<'a> = (&'a str, &'a str, &'a [&'a str], bool, Option<usize>);

#[test]
fn the_volume_that_holds_a_path_is_the_one_mounted_deepest_above_it() {
    let linux = ["/", "/srv", "/srv/ada/disk", "/mnt/c", "/boot/efi"];
    let twice = ["/", "/work", "/work"];
    let windows = [r"C:\", r"D:\", r"C:\Mounts\Data\"];
    let none: [&str; 0] = [];
    let long = format!(r"\\?\C:\Users\ada\{}", "n".repeat(300));
    let cases: &[Case] = &[
        ("a folder on the one volume there is", "/srv/shop", &linux[..1], false, Some(0)),
        ("a folder on a volume mounted further down", "/srv/ada/shop", &linux, false, Some(1)),
        ("a folder on one mounted inside that", "/srv/ada/disk/shop", &linux, false, Some(2)),
        ("the place a volume is mounted", "/srv/ada/disk", &linux, false, Some(2)),
        ("a folder whose name begins as a mount's does", "/srv/ada/disks", &linux, false, Some(1)),
        ("another that does", "/srvs/ada", &linux, false, Some(0)),
        ("a mount written with a stroke after it", "/boot/efi/EFI", &["/", "/boot/efi/"], false, Some(1)),
        ("two volumes mounted in one place", "/work/shop", &twice, false, Some(2)),
        ("a name with the other stroke in it", r"/mnt/c\d/shop", &linux, false, Some(0)),
        ("a name in another case", "/Home/ada", &linux, false, Some(0)),
        ("a path that does not say where it starts", "home/ada", &linux, false, None),
        ("no volume listed", "/srv/ada", &none, false, None),
        ("a drive", r"C:\Users\ada\shop", &windows, true, Some(0)),
        ("another drive", r"D:\work", &windows, true, Some(1)),
        ("a drive's own top", r"C:\", &windows, true, Some(0)),
        ("a drive in the long form", r"\\?\C:\Users\ada\shop", &windows, true, Some(0)),
        ("a path past the system's limit, which stays long", &long, &windows, true, Some(0)),
        ("a drive written small", r"c:\users\ada", &windows, true, Some(0)),
        ("a drive written with the other stroke", "C:/Work/ada", &windows, true, Some(0)),
        ("a volume mounted in a folder", r"C:\Mounts\Data\shop", &windows, true, Some(2)),
        ("the same in another case", r"\\?\C:\mounts\data\shop", &windows, true, Some(2)),
        ("a folder beside it", r"C:\Mounts\Database", &windows, true, Some(0)),
        ("a drive that is not listed", r"E:\work", &windows, true, None),
        ("a share on another machine", r"\\server\share\work", &windows, true, None),
        ("the same in the long form", r"\\?\UNC\server\share\work", &windows, true, None),
        (
            "a share that is listed",
            r"\\?\UNC\server\share\work",
            &[r"C:\", r"\\server\share\"],
            true,
            Some(1),
        ),
        ("a volume by its id", r"\\?\Volume{b75e2c83}\work", &windows, true, None),
        ("a path that does not say where it starts", r"work\shop", &windows, true, None),
        ("a path from the top of whichever drive", r"\Users\ada", &windows, true, None),
        // A path of one system read as the other's is on no volume.
        ("a drive, where there are none", r"C:\Users\ada", &windows, false, None),
        ("a path from the top, where there are drives", "/srv/ada", &windows, true, None),
    ];
    for (what, path, mounts, on_windows, which) in cases {
        assert_eq!(volume(path, mounts, *on_windows), *which, "{what}: `{path}` among {mounts:?}");
    }
}

#[test]
fn the_room_left_where_this_is_built_can_be_determined() {
    let here = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let found = hosting::resources(here).unwrap();
    assert!(found.free_disk_mb.is_some(), "no volume was found for {}", here.display());
}

#[test]
fn a_folder_that_is_on_no_volume_is_said_with_its_name_and_the_volumes_the_system_does_list() {
    let listed = |mounts: &[&str]| mounts.iter().map(|mount| mount.to_string()).collect::<Vec<String>>();
    let many: Vec<String> = (b'C'..=b'N').map(|drive| format!(r"{}:\", drive as char)).collect();
    let cases: &[(&str, &str, Option<Vec<String>>, &str)] = &[
        // (what it is, the folder, what the system lists, what is said after the same beginning)
        (
            "a share on another machine",
            r"\\server\share\shop",
            Some(listed(&[r"C:\", r"D:\"])),
            r"\\server\share\shop is on none of the volumes the system lists (C:\, D:\)",
        ),
        (
            "a folder of another system",
            "work/shop",
            Some(listed(&["/", "/srv"])),
            "work/shop is on none of the volumes the system lists (/, /srv)",
        ),
        (
            "a volume listed twice is said once",
            r"E:\work",
            Some(listed(&[r"C:\", r"C:\", r"D:\"])),
            r"E:\work is on none of the volumes the system lists (C:\, D:\)",
        ),
        (
            "more volumes than a sentence holds",
            r"Z:\work",
            Some(many),
            r"Z:\work is on none of the volumes the system lists (C:\, D:\, E:\, F:\, G:\, H:\, I:\, J:\ and 4 more)",
        ),
        (
            "a system that lists none",
            "/srv/shop",
            Some(Vec::new()),
            "the system lists no volume, so /srv/shop is on none",
        ),
        (
            "a system that would not list them",
            "/srv/shop",
            None,
            "the system would not list its volumes, so which of them /srv/shop is on is not known",
        ),
    ];
    for (what, folder, mounts, said) in cases {
        assert_eq!(
            unplaced(folder, mounts.as_deref()),
            format!("free disk space could not be determined, which is not the same as too little: {said}"),
            "{what}"
        );
    }
}
