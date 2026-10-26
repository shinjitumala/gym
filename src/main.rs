use gym::cmd::{Food, Import, Init, New, Sync, Web, Weight};
use gym::com::*;
use gym::ctx;
use std::process::exit;

#[derive(Acts)]
#[acts(desc = "")]
#[allow(dead_code)]
pub struct Main(Weight, Web, Sync, New, Food, Db);

#[derive(Acts)]
#[acts(desc = "The database.")]
#[allow(dead_code)]
pub struct Db(Init, Import);

version!(C);

/// Whether the command line only asks about the binary — which build this is,
/// what it can do — rather than about the log.
fn about_the_binary(args: &[&str]) -> bool {
    args.iter().any(|a| *a == "--version" || *a == "--help")
}

/// Parse the command line and run it.
///
/// The framework's own `run` prints a failure and returns, which leaves a
/// failing command with exit status 0 — every script that runs `gym` would read
/// that as success. So the dispatch is spelled out here: the actions and their
/// help still come from the derives, and the exit status comes from whether
/// they returned an error.
fn dispatch() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    let exec = argv.first().map(String::as_str).unwrap_or("gym");
    let rest: Vec<&str> = argv.iter().skip(1).map(String::as_str).collect();

    let c = match ctx::config_path().and_then(|p| C::new(&p)) {
        Ok(c) => c,
        // Asking which build is installed, or what a command takes, is a
        // question about the binary, so it is answered on a machine that has
        // no configuration yet — which is where those questions are asked.
        Err(_) if about_the_binary(&rest) => C::unconfigured(),
        Err(e) => return Err(e),
    };

    if rest.iter().any(|a| *a == "--version") {
        return Version::next_impl(&c, &[]).map_err(|e| e.to_string());
    }

    let mut s = ParseCtx {
        exec,
        pfx: Vec::new(),
    };
    Main::next(&c, &mut s, &rest).map_err(|e| {
        let help = std::iter::once(s.exec)
            .chain(s.pfx.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{e}\nUse this command for help:\n{help} --help")
    })
}

fn main() {
    if let Err(e) = dispatch() {
        eprintln!("{e}\nAborting.");
        exit(1)
    }
}
