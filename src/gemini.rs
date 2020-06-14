extern crate openssl;
use openssl::ssl::{SslMethod, SslConnector, SslVerifyMode};

use std::io::{Read, Write};
use std::net::TcpStream;
//use std::sync::Arc;

extern crate url;
use url::Url;

extern crate mime;

extern crate regex;
use regex::Regex;

use crate::certificates;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StatusCode {
    Input                   = 10,
    Success                 = 20,
    SuccessEndCert          = 21,
    RedirectTemp            = 30,
    RedirectPerm            = 31,
    TemporaryFailure        = 40,
    ServerUnavailable       = 41,
    CgiError                = 42,
    ProxyError              = 43,
    SlowDown                = 44,
    PermanentFailure        = 50,
    NotFound                = 51,
    Gone                    = 52,
    ProxyReqRefused         = 53,
    BadRequest              = 59,
    ClientCertRequired      = 60,
    TransientCertRequested  = 61,
    AuthorizedCertRequired  = 62,
    CertNotAccepted         = 63,
    FutureCertRejected      = 64,
    ExpiredCertRejected     = 65
}

fn statuscode_from_u8(i: u8) -> Option<StatusCode> {
    let code = match i {
        10 => Some(StatusCode::Input),
        20 => Some(StatusCode::Success),
        21 => Some(StatusCode::SuccessEndCert),
        30 => Some(StatusCode::RedirectTemp),
        31 => Some(StatusCode::RedirectPerm),
        40 => Some(StatusCode::TemporaryFailure),
        41 => Some(StatusCode::ServerUnavailable),
        42 => Some(StatusCode::CgiError),
        43 => Some(StatusCode::ProxyError),
        44 => Some(StatusCode::SlowDown),
        50 => Some(StatusCode::PermanentFailure),
        51 => Some(StatusCode::NotFound),
        52 => Some(StatusCode::Gone),
        53 => Some(StatusCode::ProxyReqRefused),
        59 => Some(StatusCode::BadRequest),
        60 => Some(StatusCode::ClientCertRequired),
        61 => Some(StatusCode::TransientCertRequested),
        62 => Some(StatusCode::AuthorizedCertRequired),
        63 => Some(StatusCode::CertNotAccepted),
        64 => Some(StatusCode::FutureCertRejected),
        65 => Some(StatusCode::ExpiredCertRejected),
        _ => None
    };

    code
}

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

pub struct ResponseHeader {
    pub status: StatusCode,
    pub meta: Option<String>
}

pub struct GeminiResponse {
    pub status: StatusCode,
    pub meta: Option<String>,
    pub contents: Option<Vec<u8>>
}

pub fn parse_gemini_doc(page: &str) -> Vec<GeminiLine> {
    let mut lines = Vec::<GeminiLine>::new();
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
                lines.push( GeminiLine {
                    linetype: LineType::Preformatted,
                    main: Some(line.clone().to_string()),
                    alt: None
                });
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

            lines.push(GeminiLine {
                linetype: LineType::Link,
                main: Some(url),
                alt: alt
            }); 
        }
        else if list_regex.is_match(line) {
            let groups = list_regex.captures(line).unwrap();
            let item = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());

            lines.push(GeminiLine {
                linetype: LineType::ListItem,
                main: Some(item),
                alt: None
            });
        }
        else if heading_regex.is_match(line) {
            let groups = heading_regex.captures(line).unwrap();
            let hashes = groups.get(1).unwrap().as_str();
            let level = hashes.len();

            let linetype = match level {
                1 => LineType::Heading1,
                2 => LineType::Heading2,
                3 => LineType::Heading3,
                _ => LineType::Text
            };

            let s = groups.get(2).map_or("".to_string(), |u| u.as_str().to_string());

            lines.push(GeminiLine {
                linetype: linetype,
                main: Some(s),
                alt: None
            });
        }
        else if quote_regex.is_match(line) {
            let groups = quote_regex.captures(line).unwrap();
            let s = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());

            lines.push(GeminiLine {
                linetype: LineType::Quote,
                main: Some(s),
                alt: None
            });
        }
        else {
            lines.push(GeminiLine {
                linetype: LineType::Text,
                main: Some(line.clone().to_string()),
                alt: None
            });
        }
    }

    lines
}

fn parse_response_header(res: &str) -> Result<ResponseHeader, &str> {
    let mut iter = res.split_whitespace();
    let codestr = match iter.next() {
        Some(c) => c,
        None => { return Err("Error parsing header"); }
    };
    let meta = match iter.next() {
        Some(m) => m.to_string(),
        None => { return Err("Error parsing header"); }
    };

    let codeint = match codestr.parse::<u8>() {
        Ok(c) => c,
        Err(_e) => { return Err("Error parsing code"); }
    };
    let code = match statuscode_from_u8(codeint) {
        Some(c) => c,
        None => { return Err("Status code not known"); }
    };

    return Ok(ResponseHeader{status: code, meta: Some(meta)});
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

pub fn make_request(request_url: &str) -> Result<GeminiResponse, &str> {
    let url = match Url::parse(request_url) {
        Ok(u) => { u },
        Err(_e) => { return Err("Failed parsing URL"); }
    };

    let scheme = url.scheme();
    if scheme != "gemini" {
        return Err("Scheme not supported");
    }

    let host = match url.host_str() {
        Some(h) => h,
        None => { return Err("Did not find hostname"); }
    };

    let port = match url.port() {
        Some(p) => p,
        None => match scheme {
            "gemini" => 1965,
            _ => {return Err("No port known for given scheme")}
        }
    };

    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    let connector = builder.build();
    let stream = match TcpStream::connect(format!("{}:{}", host, port)) {
        Ok(s) => s,
        Err(_) => { return Err("Unable to connect"); }
    };
    let mut stream = connector.connect(host, stream).unwrap();

    match certificates::check_cert(&stream, &host) {
        Ok(_) => (),
        Err(_) => return Err("Certificate error")
    }

    let mut req = request_url.clone().to_string();
    req.push_str("\r\n");
    let req = req.into_bytes();
    stream.write_all(&req).unwrap();

    let mut buf = vec![0u8; 1029];
    let read = stream.read(&mut buf).unwrap();

    let header = buf[..read].to_vec();
    let headerstr = String::from_utf8(header).unwrap();

    if read == 1029 && buf[1027..] != [13, 10] {
        return Err("Faulty header received");
    }

    let header = parse_response_header(&headerstr).unwrap();

    let meta = &header.meta.unwrap();

    let mut content_buffer = Vec::<u8>::new();
    stream.read_to_end(&mut content_buffer).unwrap();

    let response = GeminiResponse {
        status: header.status,
        meta: Some(meta.to_string()),
        contents: Some(content_buffer)
    };

    return Ok(response);
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
            assert!(line.linetype == LineType::Text);
            match i {
                0 => {
                    assert!(line.main == Some("This is a text line".to_string()));
                    assert!(line.alt == None);
                },
                1 => {
                    assert!(line.main == Some("this is one too.".to_string()));
                    assert!(line.alt == None);
                },
                2 => {
                    assert!(line.main == Some("日本語".to_string()));
                    assert!(line.alt == None);
                },
                _ => {}
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
            assert!(line.linetype == LineType::Link);
            match i {
                0 => {
                    assert!(line.main == Some("gemini://example.com".to_string()));
                    assert!(line.alt == Some("Link to example".to_string()));
                }
                1 => {
                    assert!(line.main == Some("gemini://another.site".to_string()));
                    assert!(line.alt == Some("This one has some more whitespace".to_string()));
                }
                2 => {
                    assert!(line.main == Some("gemini://third.one".to_string()));
                    assert!(line.alt == Some("漢字".to_string()));
                }
                3 => {
                    assert!(line.main == Some("gemini://no.name".to_string()));
                    assert!(line.alt == None);
                }
                _ => {}
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
                    assert!(line.linetype == LineType::Text);
                    assert!(line.main == Some("Normal line here".to_string()));
                    assert!(line.alt == None);
                }
                1 => {
                    assert!(line.linetype == LineType::Preformatted);
                    assert!(line.main == Some("This is preformatted".to_string()));
                    assert!(line.alt == None);
                }
                2 => {
                    assert!(line.linetype == LineType::Text);
                    assert!(line.main == Some("This is not".to_string()));
                    assert!(line.alt == None);
                }
                _ => {}
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
                    assert!(line.linetype == LineType::Heading1);
                    assert!(line.main == Some("Level 1 heading".to_string()));
                    assert!(line.alt == None);
                }
                1 => {
                    assert!(line.linetype == LineType::Heading2);
                    assert!(line.main == Some("Level 2 heading".to_string()));
                    assert!(line.alt == None);
                }
                2 => {
                    assert!(line.linetype == LineType::Heading3);
                    assert!(line.main == Some("レベル 3 ヘディング".to_string()));
                    assert!(line.alt == None);
                }
                _ => {}
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
                    assert!(line.linetype == LineType::ListItem);
                    assert!(line.main == Some("First item".to_string()));
                    assert!(line.alt == None);
                }
                1 => {
                    assert!(line.linetype == LineType::ListItem);
                    assert!(line.main == Some("Second item".to_string()));
                    assert!(line.alt == None);
                }
                2 => {
                    assert!(line.linetype == LineType::Text);
                    assert!(line.main == Some("*This is not a list item".to_string()));
                    assert!(line.alt == None);
                }
                3 => {
                    assert!(line.linetype == LineType::ListItem);
                    assert!(line.main == Some("これは new list".to_string()));
                    assert!(line.alt == None);
                }
                _ => {}
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
            assert!(line.linetype == LineType::Quote);
            match i {
                0 => {
                    assert!(line.main == Some("2020".to_string()));
                    assert!(line.alt == None);
                }
                1 => {
                    assert!(line.main == Some("quotes as standard".to_string()));
                    assert!(line.alt == None);
                }
                2 => {
                    assert!(line.main == Some("  what about whitespace?".to_string()));
                    assert!(line.alt == None);
                }
                3 => {
                    assert!(line.main == Some("錆".to_string()));
                    assert!(line.alt == None);
                }
                _ => {}
            }
        }
    }
    
}