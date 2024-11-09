use crate::profile::Template;
use eyre::Result;
use nom::{
    bytes::complete::{is_not, tag, take_while_m_n},
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

fn pars<'a>(text: &'a str, res: &mut Vec<&'a str>) {
    let (remaining, _) = is_not::<&str, &str, Error<&str>>("{{")(text).expect("here");
    match parse_braced_hash(remaining) {
        Ok((remain, hashes)) => {
            res.push(hashes);
            if !remain.is_empty() {
                pars(remain, res);
            }
        }
        Err(_e) => {
            // warn!("parse template terminate: {:?}", e);
        }
    };
}

impl Template {
    pub fn parse_hash_str_list(&self) -> Result<Vec<Vec<u8>>> {
        use hex::decode;
        let text = &self.content;

        let mut res = vec![];
        let text = format!(" {}", text); // hack
        pars(text.as_str(), &mut res);
        Ok(res
            .into_iter()
            .map(|s| decode(s).expect("hex decode"))
            .collect())
    }
}

// pub struct Template

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use nom::AsBytes;

    impl Default for Template {
        fn default() -> Self {
            let string_default = String::default();
            Template {
                name: string_default.clone(),
                content: string_default.clone(),
                group: string_default.clone(),
                mode: string_default.clone(),
                owner: string_default.clone(),
                path: string_default,
                symlink: true,
            }
        }
    }

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
            t.parse_hash_str_list().unwrap().get(0).unwrap().as_bytes()
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
            l.get(0).unwrap().as_slice()
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
            l.get(0).unwrap().as_slice()
        )
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
            l.get(0).unwrap().as_slice()
        )
    }
    #[test]
    fn parse_template_none() {
        let str = "";

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().len() == 0)
    }
    #[test]
    fn parse_template_multi_line_truncate() {
        let str = r#"some {{ d9cd8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80
        }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().len() == 0)
    }
    #[test]
    fn parse_template_invalid_length_of_hash() {
        let str = r#"some {{ 8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80 }}"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().len() == 0)
    }
    #[test]
    fn parse_template_open() {
        let str = r#"some {{ 8155764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a80"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().len() == 0)
    }
    #[test]
    fn parse_template_whatever() {
        let str = r#"some {{ 8155{{764c3543f10fad8a480d743137466f8d55213c8eaefcd12f06d43a\8}}0"#;

        let t = Template {
            content: String::from(str),
            ..Template::default()
        };
        assert!(t.parse_hash_str_list().unwrap().len() == 0)
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
