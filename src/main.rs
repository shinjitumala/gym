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
/// `--version` and `--help` are answered on an unconfigured context, because a
/// machine that has just installed the binary has no configuration yet; every
/// other command gets the configured one. A failure is returned for `main` to
/// report and exit 1.
fn dispatch() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    let exec = argv.first().map(String::as_str).unwrap_or("gym");
    let rest: Vec<&str> = argv.iter().skip(1).map(String::as_str).collect();

    let c = match ctx::config_path().and_then(|p| C::new(&p)) {
        Ok(c) => c,
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
