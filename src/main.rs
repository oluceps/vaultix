#![feature(iterator_try_collect)]
use cmd::Args;
use eyre::Result;
use spdlog::formatter::{pattern, PatternFormatter};

mod cmd;
mod helper;
mod interop;
mod profile;
use std::os::unix::process::parent_id;

fn main() -> Result<()> {
    let _ = spdlog::init_env_level();
    let as_sd_unit = parent_id() == 1;
    if as_sd_unit {
        for sink in spdlog::default_logger().sinks() {
            sink.set_formatter(Box::new(PatternFormatter::new(pattern!(
                "{^{level}} - {payload}{eol}"
            ))))
        }
        spdlog::debug!("Detected running as systemd unit");
    }
    let args: Args = argh::from_env();
    args.ayaya()
}
