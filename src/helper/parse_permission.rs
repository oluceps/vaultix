use nom::{bytes::complete::take_while_m_n, combinator::map_res, IResult};

fn is_oct_digit(c: char) -> bool {
    c.is_digit(8)
}

pub fn parse_octal_string(input: &str) -> Result<u32, String> {
    match parse_octal_permissions(input) {
        Ok((_, octal)) => Ok(octal),
        Err(_) => Err(format!("Failed to parse octal string: {}", input)),
    }
}

fn parse_octal_permissions(input: &str) -> IResult<&str, u32> {
    map_res(take_while_m_n(1, 3, is_oct_digit), |oct_str: &str| {
        u32::from_str_radix(oct_str, 8)
    })(input)
}
