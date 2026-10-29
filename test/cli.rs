mod support;

use std::process::Command;
use support::{binary, Scratch};

fn run(config: &str, home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(args)
        .env("FPR_GYM_CONFIG_PATH", config)
        .env("HOME", home)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run the binary")
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn version_and_help_need_no_configuration() {
    let s = Scratch::new("meta");
    for args in [&["--version"][..], &["--help"][..], &["db", "--help"][..]] {
        let out = Command::new(binary())
            .args(args)
            .env("HOME", s.path())
            .env_remove("FPR_GYM_CONFIG_PATH")
            .stdin(std::process::Stdio::null())
            .output()
            .expect("run the binary");
        assert!(out.status.success(), "{:?}: {}", args, text(&out));
        let t = text(&out);
        if args[0] == "--version" {
            assert!(t.contains("gym v"), "{t}");
        } else {
            assert!(t.contains("Usage:"), "{t}");
        }
    }
}

#[test]
fn db_init_creates_a_database_and_refuses_a_second_time() {
    let s = Scratch::new("init");
    let cfg = s.config("/tmp/remote.db");

    let a = run(&cfg, s.path(), &["db", "init"]);
    assert!(a.status.success(), "{}", text(&a));
    assert!(s.path().join("gym.db").exists());
    assert!(text(&a).contains("Created"), "{}", text(&a));

    let b = run(&cfg, s.path(), &["db", "init"]);
    assert!(!b.status.success());
    assert!(text(&b).contains("already a database"), "{}", text(&b));
}

#[test]
fn a_missing_database_is_named_rather_than_created() {
    let s = Scratch::new("missing");
    let cfg = s.config("/tmp/remote.db");
    let out = run(&cfg, s.path(), &["food"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(t.contains("No database at"), "{t}");
    assert!(t.contains("gym db init"), "{t}");
}

#[test]
fn a_directory_as_db_is_refused_with_a_message() {
    let s = Scratch::new("dir");
    let p = s.path().join("gym.toml");
    std::fs::write(
        &p,
        format!(
            "db = \"{}\"\nremote = \"/tmp/remote.db\"\n",
            s.path().display()
        ),
    )
    .unwrap();
    let out = run(&p.to_string_lossy(), s.path(), &["food"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(t.contains("a directory"), "{t}");
    assert!(t.contains("path to the database file"), "{t}");
}

#[test]
fn the_config_is_toml_and_a_broken_one_is_reported() {
    let s = Scratch::new("cfg");
    let p = s.path().join("bad.toml");
    std::fs::write(&p, "db = 3\n").unwrap();
    let out = run(&p.to_string_lossy(), s.path(), &["food"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("Failed to parse"), "{}", text(&out));
}

#[test]
fn an_empty_remote_is_refused() {
    let s = Scratch::new("remote");
    let p = s.path().join("gym.toml");
    std::fs::write(&p, format!("db = \"{}\"\nremote = \"  \"\n", s.db())).unwrap();
    let out = run(&p.to_string_lossy(), s.path(), &["food"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("'remote' is empty"), "{}", text(&out));
}
