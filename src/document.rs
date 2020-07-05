extern crate url;
use url::Url;

extern crate mime;

extern crate regex;
use regex::Regex;


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineType {
    Text,
    Link,
    Preformatted,
    Heading1,
    Heading2,
    Heading3,
    Quote,
    ListItem
}

#[derive(Debug)]
pub struct GeminiLine {
    pub linetype: LineType,
    pub main: Option<String>,
    pub alt: Option<String>
}

#[derive(Clone, PartialEq)]
pub enum Line {
    Text(String),
    Link(String, Option<String>),
    Preformatted(String),
    Heading1(String),
    Heading2(String),
    Heading3(String),
    Quote(String),
    ListItem(String)
}

pub fn parse_gemini_doc(page: &str) -> Vec<Line> {
    let mut lines = Vec::<Line>::new();
    let mut preformatted = false;

    let link_regex = Regex::new(r"^=>\s*(\S+)(?:\s+(.*))?").unwrap();
    let preformat_regex = Regex::new(r"^```").unwrap();
    let list_regex = Regex::new(r"^\* (.*)").unwrap();
    let heading_regex = Regex::new(r"^(#+)(?:\s*)?(.*)").unwrap();
    let quote_regex = Regex::new(r"^>(.*)").unwrap();

    for line in page.lines() {
        if preformatted {
            if preformat_regex.is_match(line) {
                preformatted = false;
                continue;
            }
            else {
                lines.push(Line::Preformatted(line.clone().to_string()));
                continue;
            }
        }
        else if preformat_regex.is_match(line) {
            preformatted = true;
            continue;
        }
        else if link_regex.is_match(line) {
            let groups = link_regex.captures(line).unwrap();
            let url = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());
            let name = groups.get(2).map_or("".to_string(), |u| u.as_str().to_string());

            let alt;
            if name == "" {
                alt = None;
            }
            else {
                alt = Some(name);
            }

            lines.push(Line::Link(url, alt));
        }
        else if list_regex.is_match(line) {
            let groups = list_regex.captures(line).unwrap();
            let item = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());

            lines.push(Line::ListItem(item));
        }
        else if heading_regex.is_match(line) {
            let groups = heading_regex.captures(line).unwrap();
            let hashes = groups.get(1).unwrap().as_str();
            let level = hashes.len();

            let s = groups.get(2).map_or("".to_string(), |u| u.as_str().to_string());

            match level {
                1 => {
                    lines.push(Line::Heading1(s));
                },
                2 => {
                    lines.push(Line::Heading2(s));
                },
                3 => {
                    lines.push(Line::Heading3(s));
                },
                _ => {
                    lines.push(Line::Text(s));
                }
            }
        }
        else if quote_regex.is_match(line) {
            let groups = quote_regex.captures(line).unwrap();
            let s = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());

            lines.push(Line::Quote(s));
        }
        else {
            lines.push(Line::Text(line.clone().to_string()));
        }
    }

    lines
}



pub fn is_valid_gemini_url(url: &str) -> bool {
    let url = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => { return false }
    };

    let scheme = url.scheme();
    if scheme != "gemini" {
        return false;
    }

    match url.host_str() {
        Some(_) => {return true},
        None => { return false; }
    };
}



pub fn print_gemini_doc(lines: &Vec<GeminiLine>) {
    for line in lines {
        match line.linetype {
            LineType::Text => println!("{}", line.main.as_ref().unwrap()),
            LineType::Link => println!("Link: {} {}", line.main.as_ref().unwrap(), line.alt.as_ref().unwrap()),
            LineType::Quote => println!(">{}", line.main.as_ref().unwrap()),
            LineType::ListItem => println!("* {}", line.main.as_ref().unwrap()),
            LineType::Heading1 => println!("Heading1: {}", line.main.as_ref().unwrap()),
            LineType::Heading2 => println!("Heading2: {}", line.main.as_ref().unwrap()),
            LineType::Heading3 => println!("Heading3: {}", line.main.as_ref().unwrap()),
            LineType::Preformatted => println!("Preformatted: {}", line.main.as_ref().unwrap()),
        };
    }
}

pub fn gemini_doc_as_str(lines: &Vec<GeminiLine>) -> String {
    let mut s = String::new();

    for line in lines {
        match line.linetype {
            LineType::Text => s += &format!("{}\n", line.main.as_ref().unwrap()),
            LineType::Link => s += &format!("Link: {} {}\n", line.main.as_ref().unwrap(), line.alt.as_ref().unwrap()),
            LineType::Quote => s += &format!(">{}\n", line.main.as_ref().unwrap()),
            LineType::ListItem => s += &format!("* {}\n", line.main.as_ref().unwrap()),
            LineType::Heading1 => s += &format!("Heading1: {}\n", line.main.as_ref().unwrap()),
            LineType::Heading2 => s += &format!("Heading2: {}\n", line.main.as_ref().unwrap()),
            LineType::Heading3 => s += &format!("Heading3: {}\n", line.main.as_ref().unwrap()),
            LineType::Preformatted => s += &format!("Preformatted: {}\n", line.main.as_ref().unwrap()),
        };
    }

    s
}

pub fn is_gemini_doc(mime: &str) -> bool {
    let m: mime::Mime = match mime.parse() {
        Ok(m) => m,
        Err(_) => {return false;}
    };

    m.type_() == "text" && m.subtype() == "gemini"
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn parse_textlines() {
        let t = "This is a text line\n\
                 this is one too.\n\
                 日本語";
        let r = parse_gemini_doc(&t);
        assert_eq!(r.len(), 3);
        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::Text("This is a text line".to_string()));
                },
                1 => {
                    assert!(line == Line::Text("this is one too.".to_string()));
                },
                2 => {
                    assert!(line == Line::Text("日本語".to_string()));
                },
                _ => {assert!(false)}
            }
        }
    }

    #[test]
    fn parse_linklines() {
        let t = "=> gemini://example.com Link to example\n\
                 =>        gemini://another.site       This one has some more whitespace\n\
                 =>gemini://third.one 漢字\n\
                 =>gemini://no.name";
        let r = parse_gemini_doc(&t);

        assert_eq!(r.len(), 4);
        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::Link("gemini://example.com".to_string(), Some("Link to example".to_string())));
                }
                1 => {
                    assert!(line == Line::Link("gemini://another.site".to_string(), Some("This one has some more whitespace".to_string())));
                }
                2 => {
                    assert!(line == Line::Link("gemini://third.one".to_string(), Some("漢字".to_string())));
                }
                3 => {
                    assert!(line == Line::Link("gemini://no.name".to_string(), None));
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[test]
    fn parse_preformatted() {
        let t = "Normal line here\n\
                 ```\n\
                 This is preformatted\n\
                 ```This shouldn't appear in the result\n\
                 This is not";
        let r = parse_gemini_doc(&t);

        assert_eq!(r.len(), 3);

        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::Text("Normal line here".to_string()));
                }
                1 => {
                    assert!(line == Line::Preformatted("This is preformatted".to_string()));
                }
                2 => {
                    assert!(line == Line::Text("This is not".to_string()));
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[test]
    fn parse_heading() {
        let t = "# Level 1 heading\n\
                 ##Level 2 heading\n\
                 ### レベル 3 ヘディング";
        let r = parse_gemini_doc(&t);

        assert_eq!(r.len(), 3);
        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::Heading1("Level 1 heading".to_string()));
                }
                1 => {
                    assert!(line == Line::Heading2("Level 2 heading".to_string()));
                }
                2 => {
                    assert!(line == Line::Heading3("レベル 3 ヘディング".to_string()));
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[test]
    fn parse_list_item() {
        let t = "* First item\n\
                 * Second item\n\
                 *This is not a list item\n\
                 * これは new list";
        let r = parse_gemini_doc(&t);

        assert_eq!(r.len(), 4);
        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::ListItem("First item".to_string()));
                }
                1 => {
                    assert!(line == Line::ListItem("Second item".to_string()));
                }
                2 => {
                    assert!(line == Line::Text("*This is not a list item".to_string()));
                }
                3 => {
                    assert!(line == Line::ListItem("これは new list".to_string()));
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[test]
    fn parse_quote() {
        let t = ">2020\n\
                 >quotes as standard\n\
                 >  what about whitespace?\n\
                 >錆";
        let r = parse_gemini_doc(&t);

        assert_eq!(r.len(), 4);
        for (i, line) in r.into_iter().enumerate() {
            match i {
                0 => {
                    assert!(line == Line::Quote("2020".to_string()));
                }
                1 => {
                    assert!(line == Line::Quote("quotes as standard".to_string()));
                }
                2 => {
                    assert!(line == Line::Quote("  what about whitespace?".to_string()));
                }
                3 => {
                    assert!(line == Line::Quote("錆".to_string()));
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }
    
}