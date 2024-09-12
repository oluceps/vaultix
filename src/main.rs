use cmd::Args;
use eyre::Result;

mod cmd;
mod interop;

fn main() -> Result<()> {
    let args: Args = argh::from_env();
    args.ayaya()
}
