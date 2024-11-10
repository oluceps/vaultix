use crate::profile::Template;
use eyre::Result;
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::verify,
    error::Error,
    sequence::delimited,
    IResult,
};

fn parse_braced_hash(input: &str) -> IResult<&str, &str, Error<&str>> {
    delimited(
        tag("{{ "),
        take_while_m_n(64, 64, |c: char| c.is_ascii_hexdigit()),
        tag(" }}"),
    )(input)
}

pub fn extract_all_hashes<'a>(input: &'a str, res: &mut Vec<&'a str>) {
    if let Ok((o, b)) = verify(parse_braced_hash, |_: &str| true)(input) {
        res.push(b);
        extract_all_hashes(o, res)
    } else if input.len() < 66 {
        // less than expected `{{ hash }}` length
        return;
    } else {
        let this = {
            // handle multibytes
            let res = input.char_indices().nth(1).map_or("", |(i, _)| &input[i..]);
            // skip to next `{`
            res.find('{').map_or("", |index| &res[index..])
        };

        extract_all_hashes(this, res)
    }
}

impl Template {
    pub fn parse_hash_str_list(&self) -> Result<Vec<Vec<u8>>> {
        use hex::decode;
        let text = &self.content;

        let mut res = vec![];
        extract_all_hashes(text.as_str(), &mut res);
        Ok(res
            .into_iter()
            .map(|s| decode(s).expect("hex decode"))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use nom::AsBytes;

    #[test]
    fn parse_template_single() {
        let str =
            "here has {{ dcd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440aa93 }} whch should be replaced";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert_eq!(
            hex!("dcd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440aa93"),
            t.parse_hash_str_list().unwrap().first().unwrap().as_bytes()
        )
    }
    #[test]
    fn parse_template_multi() {
        let str = "here {{ dcd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440aa93 }} {{ cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93 }}";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        let l = t.parse_hash_str_list().unwrap();
        assert_eq!(
            hex!("dcd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440aa93"),
            l.first().unwrap().as_slice()
        );
        assert_eq!(
            hex!("cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93"),
            l.get(1).unwrap().as_slice()
        )
    }
    #[test]
    fn parse_template_with_trailing_white() {
        let str = "{{ cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93 }} ";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        let l = t.parse_hash_str_list().unwrap();
        assert_eq!(
            hex!("cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93"),
            l.first().unwrap().as_slice()
        )
    }
    #[test]
    fn parse_template_normal() {
        let str = "{{ cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93 }}";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        let l = t.parse_hash_str_list().unwrap();
        assert_eq!(
            hex!("cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93"),
            l.first().unwrap().as_slice()
        )
    }
    #[test]
    fn parse_template_mix() {
        let str = "{{ cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93 }} {{ {{ c4e0ae1067d1ee736e051d7927d783bb70b032bf116f618454bf47122956d5ce }}";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        let l = t.parse_hash_str_list().unwrap();
        assert_eq!(
            hex!("cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93"),
            l.first().unwrap().as_slice()
        );
        assert_eq!(
            hex!("c4e0ae1067d1ee736e051d7927d783bb70b032bf116f618454bf47122956d5ce"),
            l.get(1).unwrap().as_slice()
        );
    }
    #[test]
    fn parse_template_with_heading_white() {
        let str = " {{ cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93 }}";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        let l = t.parse_hash_str_list().unwrap();
        assert_eq!(
            hex!("cd789434d890685da841b8db8a02b0173b90eac3774109ba9bca1b81440a2a93"),
            l.first().unwrap().as_slice()
        )
    }
    #[test]
    fn parse_template_none() {
        let str = "";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_brace() {
        let str = "{";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_multi_line_truncate() {
        let str = r#"some {{ d9cd8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80
        }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_multi_line_truncate_type1() {
        let str = r#"some {{ d9cd8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80 }
        }"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_pad() {
        let str = r#"some {{ d9cd8155764c3543f10fad8a480d743137466f8d55 13c8eaefcd12f06d43a80 }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_char_not_hex() {
        let str = r#"some {{ d9cd8155764c3543f10fad8a480d743137466f8d55l13c8eaefcd12f06d43a80 }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_no_hash() {
        let str = r#"some {{ }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_invalid_length_of_hash() {
        let str = r#"some {{ 8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80 }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_open() {
        let str = r#"some {{ 8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_whatever() {
        let str = r#"some {{ 8155{{764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a\8}}0"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().is_empty())
    }
    #[test]
    fn parse_template_fuzz_crash_1() {
        let str = r#"{{ EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE9EEEEEEEEEEEEEEEEEE1A }}{"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        t.parse_hash_str_list().unwrap();
    }
    #[test]
    fn render() {
        let s: String = String::from("{{ hash }}");

        assert_eq!(
            "some",
            s.replace(concat!("{{ ", "hash", " }}").trim(), "some")
        );
        assert_eq!(
            "some",
            // holy
            s.replace(format!("{{{{ {} }}}}", "hash").trim(), "some")
        );
    }
}
