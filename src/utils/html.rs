use regex::Regex;

pub fn parse_html(s: String) -> String {
    let re = Regex::new(r"&lt;|&gt;|\s<br/>\s").unwrap();

    re.replace_all(&s, |caps: &regex::Captures| match &caps[0] {
        "&lt;" => "<",
        "&gt;" => ">",
        _ => "\n",
    })
    .to_string()
}
