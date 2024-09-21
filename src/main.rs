use cmd::Args;
use eyre::Result;

mod cmd;
mod helper;
mod interop;
mod profile;

fn main() -> Result<()> {
    let _ = spdlog::init_env_level();
    let args: Args = argh::from_env();
    args.ayaya()
}
