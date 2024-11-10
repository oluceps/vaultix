#![no_main]
use arbitrary::Arbitrary;
use lib::profile;
use libfuzzer_sys;

#[derive(Arbitrary, Debug)]
struct FuzzInput(String);

libfuzzer_sys::fuzz_target!(|input: FuzzInput| {
    let t = profile::Template {
        content: input.0,
        ..Default::default()
    };
    let _ = t.parse_hash_str_list();
});
