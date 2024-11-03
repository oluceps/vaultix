#![feature(iterator_try_collect)]
use cmd::Args;
use eyre::Result;
use spdlog::formatter::{pattern, PatternFormatter};
use std::env;

mod cmd;
mod helper;
mod interop;
mod profile;

// > systemd v232
const SD_INVOCATION_ID_ENV: &str = "INVOCATION_ID";

fn main() -> Result<()> {
    let _ = spdlog::init_env_level();
    let as_sd_unit = env::var(SD_INVOCATION_ID_ENV).is_ok();
    if as_sd_unit {
        for sink in spdlog::default_logger().sinks() {
            sink.set_formatter(Box::new(PatternFormatter::new(pattern!(
                "{^{level}} - {payload}{eol}" // remove datetime
            ))))
        }
    }
    let args: Args = argh::from_env();
    args.ayaya()
}
