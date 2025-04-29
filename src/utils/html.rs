use regex::Regex;

pub fn parse_html(s: String) -> String {
    let re = Regex::new(r"&lt;|&gt;|\s<br/>\s").unwrap();

    re.replace_all(&s, |caps: &regex::Captures| match &caps[0] {
        r"&lt;" => "<",
        r"&gt;" => ">",
        r"\s<br/>\s" => "\n",
        _ => "",
    })
    .to_string()
}
