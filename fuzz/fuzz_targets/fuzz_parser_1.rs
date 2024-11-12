#![no_main]
use arbitrary::Arbitrary;
use lib::extract_all_hashes;
use lib::parse_octal_str;
use libfuzzer_sys;

#[derive(Arbitrary, Debug)]
struct FuzzInput(String);

libfuzzer_sys::fuzz_target!(|input: FuzzInput| {
    let mut v = Vec::new();
    let _ = extract_all_hashes(input.0.as_str(), &mut v);
    let _ = parse_octal_str(input.0.as_str());
});
