#![feature(iterator_try_collect)]
use cmd::Args;
use eyre::Result;
use spdlog::formatter::{pattern, PatternFormatter};

mod cmd;
mod helper;
mod interop;
mod profile;

fn main() -> Result<()> {
    let _ = spdlog::init_env_level();
    for sink in spdlog::default_logger().sinks() {
        sink.set_formatter(Box::new(PatternFormatter::new(pattern!(
            "{^{level}} - {payload}{eol}" // remove datetime
        ))))
    }
    let args: Args = argh::from_env();
    args.ayaya()
}
