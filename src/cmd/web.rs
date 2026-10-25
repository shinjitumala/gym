use crate::com::*;

#[derive(Args)]
#[args(desc = "Runs the local web UI on loopback.")]
pub struct Web {
    #[arg(desc = "Socket address.", s = ("127.0.0.1:8080"))]
    pub addr: String,
}
impl Run<C> for Web {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(web(c, a)?)
    }
}

#[cfg(feature = "ssr")]
fn web(_c: &C, a: Web) -> Res<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to start the runtime because '{e}'."))?
        .block_on(crate::web::server::run(&a.addr))
        .map_err(|e| crate::util::Err::Str(e))
}

#[cfg(not(feature = "ssr"))]
fn web(_c: &C, a: Web) -> Res<()> {
    Err(crate::util::Err::Str(format!(
        "This build has no web UI (it was compiled without the 'ssr' feature). Run 'cargo leptos build --release' or ./install.sh, then 'gym web --addr {}'.",
        a.addr
    )))
}
