use std::io::Write;

use crate::{
    tests::utils::{OutputExtra, cargo_command, cargo_spawn_out},
    utils::rand_id,
};

mod utils;

#[test]
fn test_help() {
    let sandbox = utils::cargo_spawn_out(vec!["--help"]).unwrap();
    assert!(sandbox.status.success());
}

#[test]
fn test_bwrap() {
    // Test if home dir is missing
    let args = vec!["-f", "./profiles/bwrap-no-home.toml", "--", "ls", "/home"];
    let sandbox = utils::cargo_spawn_out(args).unwrap();
    assert!(sandbox.status.success());
    assert!(sandbox.stdout.is_empty());

    // Test if home dir exists
    let args = vec!["-f", "./profiles/bwrap-home.toml", "--", "ls", "/"];
    let sandbox = utils::cargo_spawn_out(args).unwrap();
    assert!(sandbox.stdout_str().contains("home"));
}

#[test]
fn test_seccomp() {
    // Use seccomp to restrict dir listings
    let args = vec!["-f", "./profiles/with-seccomp.toml", "--", "ls", "/"];
    let sandbox = utils::cargo_spawn_out(args).unwrap();
    assert!(sandbox.stderr_str().contains("not permitted"));
}

#[test]
fn test_env_mapper() {
    // Should be visible in sandbox
    let bwrap_test_key = "BWRAP_TEST";
    let bwrap_test_value = rand_id(12);

    // Should be not visible in sandbox
    let bwrap_fake_key = "BWRAP_FAKE";
    let bwrap_fake_value = rand_id(12);

    let mut args = vec![
        "-f",
        "<replace me>",
        "--",
        "printenv",
        bwrap_test_key,
        bwrap_fake_key,
    ];

    let run_cmd = |args: &[&str]| {
        cargo_command()
            .args(args)
            .env(bwrap_test_key, &bwrap_test_value)
            .env(bwrap_fake_key, &bwrap_fake_value)
            .output()
            .unwrap()
    };

    // Test without env mapper
    args[1] = "./profiles/bwrap-home.toml";
    let output = run_cmd(&args);
    let lines = output.stdout_str().lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], bwrap_test_value);
    assert_eq!(lines[1], bwrap_fake_value);

    // Test with env mapper
    args[1] = "./profiles/with-env.toml";
    let output = run_cmd(&args);
    let lines = output.stdout_str().lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], bwrap_test_value);
}

#[test]
fn test_dbus() {
    // TODO: Add some universal tests for blocked services
    // Only --talk=org.freedesktop.DBus allowed
    let args = vec![
        "-f",
        "./profiles/with-dbus.toml",
        "--",
        "dbus-send",
        "--type=method_call",
        "--print-reply",
        "--dest=org.freedesktop.DBus",
        "/",
        "org.freedesktop.DBus.ListNames",
    ];
    let output = utils::cargo_spawn_out(args).unwrap();
    let names = output
        .stdout_str()
        .lines()
        .filter(|line| line.contains("string \""))
        .filter_map(|line| line.split('"').nth(1))
        // Skip empty services and/or connections
        .filter(|name| !name.is_empty() && !name.starts_with(":"))
        .collect::<Vec<_>>();
    assert!(output.status.success());
    assert_eq!(names.len(), 1);
}

#[test]
fn test_slirp4netns_external() {
    let args = vec![
        "-f",
        "./profiles/with-slirp4netns.toml",
        "--",
        "curl",
        "-I",
        "example.com",
    ];
    let output = cargo_spawn_out(args).unwrap();
    let header = output.stdout_str().lines().next().unwrap();
    assert!(header.contains("200 OK"));
}

#[test]
fn test_slirp4netns_internal() {
    let local_addr = {
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = socket.local_addr().unwrap();
        std::thread::spawn(move || {
            while let Ok((mut stream, _)) = socket.accept() {
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello world!";
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        format!("http://127.0.0.1:{}", local_addr.port())
    };

    let mut args = vec!["-f", "<replace me>", "--", "curl", &local_addr];

    // Test without isolation
    args[1] = "./profiles/bwrap-no-home.toml";
    let output = cargo_spawn_out(args.clone()).unwrap();
    assert_eq!(output.stdout_str(), "Hello world!");

    // Test with isolation
    args[1] = "./profiles/with-slirp4netns.toml";
    let output = cargo_spawn_out(args).unwrap();
    assert!(output.stdout_str().is_empty());
    assert!(output.stderr_str().contains("Failed to"));
}

#[test]
fn test_app_image() {
    let mut args = vec![
        "-f",
        "<replace me>",
        "--",
        "printenv",
        "APPIMAGE_EXTRACT_AND_RUN",
    ];

    // Test without appimage service
    args[1] = "./profiles/bwrap-no-home.toml";
    let output = cargo_spawn_out(args.clone()).unwrap();
    assert!(output.stdout_str().is_empty());

    // Test with appimage service
    args[1] = "./profiles/with-appimage.toml";
    let output = cargo_spawn_out(args).unwrap();
    assert_eq!(output.stdout_str(), "1\n");
}
