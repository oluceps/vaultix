#![feature(iterator_try_collect)]
mod parser;
pub use parser::extract_all_hashes;
pub use parser::parse_octal_str;
