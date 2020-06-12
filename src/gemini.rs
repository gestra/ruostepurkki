extern crate openssl;
use openssl::ssl::{SslMethod, SslConnector, SslVerifyMode, SslStream};

use std::io::{Read, Write};
use std::net::TcpStream;
//use std::sync::Arc;

extern crate url;
use url::Url;

extern crate mime;

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
    ToggleFormatting,
    PreFormatted,
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
    for line in page.lines() {
        if preformatted {
            if line.len() >= 3 && &line[0..3] == "```" {
                preformatted = false;
                continue;
            }
            else {
                lines.push( GeminiLine {
                    linetype: LineType::PreFormatted,
                    main: Some(line.clone().to_string()),
                    alt: None
                });
                continue;
            }
        }
        else if line.len() >= 3 && &line[0..3] == "```"{
            preformatted = true;
            continue;
        }
        else if line.len() >= 2 && &line[0..2] == "=>" {
            let mut url = String::new();
            let mut name = String::new();
            let mut url_started = false;
            let mut url_passed = false;
            let mut name_started = false;

            for c in line[2..].chars() {
                if !url_started {
                    if c.is_whitespace() {
                        continue;
                    }
                    else {
                        url_started = true;
                        url.push(c);
                        continue;
                    }
                }
                if url_started && !url_passed {
                    if c.is_whitespace() {
                        url_passed = true;
                        continue;
                    }
                    else {
                        url.push(c);
                        continue;
                    }
                }
                if url_passed && !name_started {
                    if c.is_whitespace() {
                        continue;
                    }
                    else {
                        name_started = true;
                        name.push(c);
                        continue;
                    }
                }
                if name_started {
                    name.push(c);
                    continue;
                }
            }

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
        else if line.len() >= 2 && &line[0..2] == "* " {
            let mut item = String::new();
            let mut started = false;

            for c in line[2..].chars() {
                if started {
                    item.push(c);
                    continue;
                }
                else {
                    if c.is_whitespace() {
                        continue;
                    }
                    else {
                        item.push(c);
                        started = true;
                        continue;
                    }
                }
            }

            lines.push(GeminiLine {
                linetype: LineType::ListItem,
                main: Some(item),
                alt: None
            });
        }
        else if line.len() >= 1 && &line[0..1] == "#" {
            let mut s = String::new();
            let mut level = 0;
            let mut hashes_over = false;
            let mut heading_started = false;

            for c in line.chars() {
                if !hashes_over {
                    if c == '#' {
                        level += 1;
                        if level == 3 {
                            hashes_over = true;
                        }
                        continue;
                    }
                    else {
                        hashes_over = true;
                        if !c.is_whitespace() {
                            s.push(c);
                        }
                        continue;
                    }
                }
                else if !heading_started {
                    if c.is_whitespace() {
                        continue;
                    }
                    else {
                        heading_started = true;
                        s.push(c);
                        continue;
                    }
                }
                else {
                    s.push(c);
                    continue;
                }
            }

            let linetype = match level {
                1 => LineType::Heading1,
                2 => LineType::Heading2,
                3 => LineType::Heading3,
                _ => LineType::Text
            };

            lines.push(GeminiLine {
                linetype: linetype,
                main: Some(s),
                alt: None
            });
        }
        else if line.len() >= 1 && &line[0..1] == ">" {
            let mut s = String::new();
            for c in line[1..].chars() {
                s.push(c);
            }
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
                    assert!(line.linetype == LineType::PreFormatted);
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




fn parse_response_header(res: &String) -> Result<ResponseHeader, &str> {
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
    println!("Got status code {}", codeint);
    let code = match statuscode_from_u8(codeint) {
        Some(c) => c,
        None => { return Err("Status code not known"); }
    };

    return Ok(ResponseHeader{status: code, meta: Some(meta)});
}



pub fn make_request(request_url: &String) -> Result<GeminiResponse, &str> {
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

    println!("Parsed URL {}", request_url);
    println!("Scheme: {}", scheme);
    println!("Host: {}", host);
    println!("Port: {}", port);

    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    let connector = builder.build();
    let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
    let mut stream = connector.connect(host, stream).unwrap();

    match certificates::check_cert(&stream, &host) {
        Ok(_) => (),
        Err(_) => return Err("Certificate error")
    }



    let mut req = request_url.clone();
    req.push_str("\r\n");
    let req = req.into_bytes();
    stream.write_all(&req).unwrap();

    let mut buf = vec![0u8; 1029];
    let read = stream.read(&mut buf).unwrap();

    let header = buf[..read].to_vec();
    let headerstr = String::from_utf8(header).unwrap();

    println!("Header length: {}", headerstr.len());
    println!("Header: {}", headerstr);

    if read == 1029 && buf[1027..] != [13, 10] {
        return Err("Faulty header received");
    }

    let header = parse_response_header(&headerstr).unwrap();

    let meta = &header.meta.unwrap();

    println!("Status code: {} , meta: {}", header.status as u8, &meta);
    let mime = &meta.parse::<mime::Mime>().unwrap();

    if mime.type_() == "text" && mime.subtype() == "gemini" {
        println!("Gemini text here");
    }

    let mut content_buffer = Vec::<u8>::new();
    stream.read_to_end(&mut content_buffer).unwrap();

    let page = String::from_utf8(content_buffer.clone()).unwrap();
    println!("{}", page);

    let response = GeminiResponse {
        status: header.status,
        meta: Some(meta.to_string()),
        contents: Some(content_buffer)
    };

    return Ok(response);
}
