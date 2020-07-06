extern crate openssl;
use openssl::ssl::{SslMethod, SslConnector, SslVerifyMode};

use std::io::{Read, Write};
use std::net::TcpStream;

extern crate url;
use url::Url;

use crate::certificates;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StatusCode {
    Input                   = 10,
    SensitiveInput          = 11,
    Success                 = 20,
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
    CertNotAuthorized       = 61,
    CertNotValid            = 62,
}

fn statuscode_from_u8(i: u8) -> Option<StatusCode> {
    let code = match i {
        10 => Some(StatusCode::Input),
        11 => Some(StatusCode::SensitiveInput),
        20 => Some(StatusCode::Success),
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
        61 => Some(StatusCode::CertNotAuthorized),
        62 => Some(StatusCode::CertNotValid),
        _ => None
    };

    code
}

pub struct ResponseHeader {
    pub status: StatusCode,
    pub meta: Option<String>
}

pub enum Response {
    Input(String),
    SensitiveInput(String),

    Success(String, Vec<u8>),

    RedirectTemp(String),
    RedirectPerm(String),

    TemporaryFailure(Option<String>),
    ServerUnavailable(Option<String>),
    CgiError(Option<String>),
    ProxyError(Option<String>),
    SlowDown(Option<String>),

    PermanentFailure(Option<String>),
    NotFound(Option<String>),
    Gone(Option<String>),
    ProxyReqRefused(Option<String>),
    BadRequest(Option<String>),

    ClientCertRequired(Option<String>),
    CertNotAuthorized(Option<String>),
    CertNotValid(Option<String>)
}

fn parse_response_header(res: &str) -> Result<ResponseHeader, String> {
    if res.len() < 2 {
        return Err("No status code in response".to_string());
    }

    let codeint = match res[0..2].parse::<u8>() {
        Ok(c) => c,
        Err(_) => { return Err("Couldn't parse status code".to_string()); }
    };

    let meta;
    if res.len() > 3 {
        meta = Some(res[3..].to_string());
    } else {
        meta = None;
    }
    
    let code = match statuscode_from_u8(codeint) {
        Some(c) => c,
        None => { return Err(format!("Status code {} not known", codeint)); }
    };

    return Ok(ResponseHeader{status: code, meta: meta});
}

pub fn make_request(raw_url: &str) -> Result<Response, String> {
    let mut request_url = raw_url;
    let mut gemini_scheme = "gemini://".to_string();
    let url = match Url::parse(raw_url) {
        Ok(u) => u,
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            gemini_scheme.push_str(raw_url);
            match Url::parse(&gemini_scheme) {
                Ok(u) => {
                    request_url = &gemini_scheme;
                    u
                },
                Err(_) => { return Err("Failed parsing URL".to_string()); }
            }
        }
        Err(_e) => { return Err("Failed parsing URL".to_string()); }
    };

    let scheme = url.scheme();
    if scheme != "gemini" {
        return Err("Scheme not supported".to_string());
    }

    let host = match url.host_str() {
        Some(h) => h,
        None => { return Err("Did not find hostname".to_string()); }
    };

    let port = match url.port() {
        Some(p) => p,
        None => match scheme {
            "gemini" => 1965,
            _ => {return Err("No port known for given scheme".to_string())}
        }
    };

    let mut builder = match SslConnector::builder(SslMethod::tls()) {
        Ok(b) => b,
        Err(_) => { return Err("Error creating SSL connector builder".to_string()) }
    };
    builder.set_verify(SslVerifyMode::NONE);
    let connector = builder.build();
    let stream = match TcpStream::connect(format!("{}:{}", host, port)) {
        Ok(s) => s,
        Err(_) => { return Err("Unable to start TLS connection".to_string()); }
    };
    let mut stream = match connector.connect(host, stream) {
        Ok(s) => s,
        Err(_) => { return Err("Unable to connect".to_string()); }
    };

    match certificates::check_cert(&stream, &host) {
        Ok(_) => (),
        Err(_) => return Err("Certificate error".to_string())
    }

    let mut req = request_url.clone().to_string();
    req.push_str("\r\n");
    let req = req.into_bytes();
    match stream.write_all(&req) {
        Ok(_) => {},
        Err(_) => { return Err("Error writing to stream".to_string()); }
    }

    let mut buf = vec![0u8; 1029];
    let read = match stream.read(&mut buf) {
        Ok(r) => r,
        Err(_) => { return Err("Error reading header from stream".to_string()); }
    };

    if read == 1029 && buf[1027..] != [13, 10] {
        return Err("Too long header received".to_string());
    }

    let header;
    // Strip out CRLF from end of header
    if read > 2 && buf[read-2] == '\r' as u8 && buf[read-1] == '\n' as u8 {
        header = buf[..read-2].to_vec();
    } else {
        header = buf[..read].to_vec();
    }

    let headerstr = match String::from_utf8(header) {
        Ok(s) => s,
        Err(_) => { return Err("Could not parse header as UTF-8".to_string()); }
    };

    let header = match parse_response_header(&headerstr) {
        Ok(h) => h,
        Err(e) => { return Err(e.to_string()); }
    };

    let response;

    match header.status {
        StatusCode::Input => {
            let meta = match header.meta {
                Some(m) => m,
                None => String::new()
            };

            response = Response::Input(meta);
        },
        StatusCode::SensitiveInput => {
            let meta = match header.meta {
                Some(m) => m,
                None => String::new()
            };

            response = Response::SensitiveInput(meta);
        },
        StatusCode::Success => {
            let mut content_buffer = Vec::<u8>::new();
            stream.read_to_end(&mut content_buffer).unwrap();

            let metadata = match header.meta {
                Some(m) => m,
                None => String::new()
            };

            response = Response::Success(metadata, content_buffer);
        },
        StatusCode::RedirectTemp => {
            let meta = match header.meta {
                Some(m) => m,
                None => { return Err("Server returned status code for redirect but no URL provided".to_string()); }
            };

            response = Response::RedirectTemp(meta);
        },
        StatusCode::RedirectPerm => {
            let meta = match header.meta {
                Some(m) => m,
                None => { return Err("Server returned status code for redirect but no URL provided".to_string()); }
            };

            response = Response::RedirectPerm(meta);
        },
        StatusCode::TemporaryFailure => {
            response = Response::TemporaryFailure(header.meta);
        },

        StatusCode::ServerUnavailable => {
            response = Response::ServerUnavailable(header.meta);
        },
        StatusCode::CgiError => {
            response = Response::CgiError(header.meta);
        },
        StatusCode::ProxyError => {
            response = Response::ProxyError(header.meta);
        },
        StatusCode::SlowDown => {
            response = Response::SlowDown(header.meta);
        },
        StatusCode::PermanentFailure => {
            response = Response::PermanentFailure(header.meta);
        },
        StatusCode::NotFound => {
            response = Response::NotFound(header.meta);
        },
        StatusCode::Gone => {
            response = Response::Gone(header.meta);
        },
        StatusCode::ProxyReqRefused => {
            response = Response::ProxyReqRefused(header.meta);
        },
        StatusCode::BadRequest => {
            response = Response::BadRequest(header.meta);
        },
        StatusCode::ClientCertRequired => {
            response = Response::ClientCertRequired(header.meta);
        },
        StatusCode::CertNotAuthorized => {
            response = Response::CertNotAuthorized(header.meta);
        }
        StatusCode::CertNotValid => {
            response = Response::CertNotValid(header.meta);
        }
    }

    return Ok(response);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let headerstring = "10".to_string();
        let header = parse_response_header(&headerstring).unwrap();
        assert!(header.status == StatusCode::Input);
        assert!(header.meta == None);

        let headerstring = "10 Password please".to_string();
        let header = parse_response_header(&headerstring).unwrap();
        assert!(header.status == StatusCode::Input);
        assert!(header.meta == Some("Password please".to_string()));

        let headerstring = "11 Secret password please".to_string();
        let header = parse_response_header(&headerstring).unwrap();
        assert!(header.status == StatusCode::SensitiveInput);
        assert!(header.meta == Some("Secret password please".to_string()));

        let headerstring = "20 text/gemini".to_string();
        let header = parse_response_header(&headerstring).unwrap();
        assert!(header.status == StatusCode::Success);
        assert!(header.meta == Some("text/gemini".to_string()));

        let headerstring = "30 gemini://new.example.com/".to_string();
        let header = parse_response_header(&headerstring).unwrap();
        assert!(header.status == StatusCode::RedirectTemp);
        assert!(header.meta == Some("gemini://new.example.com/".to_string()));
    }
}