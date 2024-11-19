#![feature(iterator_try_collect)]
use cmd::Args;
use eyre::Result;

mod cmd;
mod helper;
mod interop;
mod parser;
mod profile;

fn main() -> Result<()> {
    colog::init();
    let args: Args = argh::from_env();
    args.ayaya()
}
