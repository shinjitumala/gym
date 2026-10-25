use crate::com::*;

#[derive(Args)]
#[args(desc = "Runs a local web server.")]
pub struct Web {
    #[arg(desc = "Socket address.", s = ("127.0.0.1:8080"))]
    _addr: String,
}
impl Run<C> for Web {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(web(c, a)?)
    }
}

#[tokio::main]
async fn web(_c: &C, _a: Web) -> Res<()> {
    todo!()
}
