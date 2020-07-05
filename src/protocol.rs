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
    TransientCertRequested  = 61,
    AuthorizedCertRequired  = 62,
    CertNotAccepted         = 63,
    FutureCertRejected      = 64,
    ExpiredCertRejected     = 65
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
        61 => Some(StatusCode::TransientCertRequested),
        62 => Some(StatusCode::AuthorizedCertRequired),
        63 => Some(StatusCode::CertNotAccepted),
        64 => Some(StatusCode::FutureCertRejected),
        65 => Some(StatusCode::ExpiredCertRejected),
        _ => None
    };

    code
}

pub struct ResponseHeader {
    pub status: StatusCode,
    pub meta: Option<String>
}

pub enum Response {
    Input {
        prompt: String
    },
    SensitiveInput {
        prompt: String
    },

    Success {
        mime: String,
        contents: Vec<u8>
    },

    RedirectTemp {
        new_url: String
    },
    RedirectPerm {
        new_url: String
    },

    TemporaryFailure {
        info: Option<String>
    },
    ServerUnavailable {
        info: Option<String>
    },
    CgiError {
        info: Option<String>
    },
    ProxyError {
        info: Option<String>
    },
    SlowDown {
        info: Option<String>
    },

    PermanentFailure {
        info: Option<String>
    },
    NotFound {
        info: Option<String>
    },
    Gone {
        info: Option<String>
    },
    ProxyReqRefused {
        info: Option<String>
    },
    BadRequest {
        info: Option<String>
    },

    ClientCertRequired {
        info: Option<String>
    },
    TransientCertRequested {
        info: Option<String>
    },
    AuthorizedCertRequired {
        info: Option<String>
    },
    CertNotAccepted {
        info: Option<String>
    },
    FutureCertRejected {
        info: Option<String>
    },
    ExpiredCertRejected {
        info: Option<String>
    },
}

fn parse_response_header(res: &str) -> Result<ResponseHeader, &str> {
    if res.len() < 2 {
        return Err("No status code in response");
    }

    let codeint = match res[0..2].parse::<u8>() {
        Ok(c) => c,
        Err(_) => { return Err("Couldn't parse status code"); }
    };

    let meta;
    if res.len() > 3 {
        meta = Some(res[3..].to_string());
    } else {
        meta = None;
    }
    
    let code = match statuscode_from_u8(codeint) {
        Some(c) => c,
        None => { return Err("Status code not known"); }
    };

    return Ok(ResponseHeader{status: code, meta: meta});
}

pub fn make_request(request_url: &str) -> Result<Response, String> {
    let url = match Url::parse(request_url) {
        Ok(u) => { u },
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

    let header = buf[..read].to_vec();
    let headerstr = match String::from_utf8(header) {
        Ok(s) => s,
        Err(_) => { return Err("Could not parse header as UTF-8".to_string()); }
    };

    if read == 1029 && buf[1027..] != [13, 10] {
        return Err("Faulty header received".to_string());
    }

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

            response = Response::Input {
                prompt: meta
            };
        },
        StatusCode::SensitiveInput => {
            let meta = match header.meta {
                Some(m) => m,
                None => String::new()
            };

            response = Response::SensitiveInput {
                prompt: meta
            };
        },
        StatusCode::Success => {
            let mut content_buffer = Vec::<u8>::new();
            stream.read_to_end(&mut content_buffer).unwrap();

            let metadata = match header.meta {
                Some(m) => m,
                None => String::new()
            };

            response = Response::Success {
                mime: metadata,
                contents: content_buffer
            }
        },
        StatusCode::RedirectTemp => {
            let meta = match header.meta {
                Some(m) => m,
                None => { return Err("Server returned status code for redirect but no URL provided".to_string()); }
            };

            response = Response::RedirectTemp {
                new_url: meta
            };
        },
        StatusCode::RedirectPerm => {
            let meta = match header.meta {
                Some(m) => m,
                None => { return Err("Server returned status code for redirect but no URL provided".to_string()); }
            };

            response = Response::RedirectPerm {
                new_url: meta
            };
        },
        StatusCode::TemporaryFailure => {
            response = Response::TemporaryFailure {
                info: header.meta
            };
        },
        StatusCode::ServerUnavailable => {
            response = Response::ServerUnavailable {
                info: header.meta
            };
        },
        StatusCode::CgiError => {
            response = Response::CgiError {
                info: header.meta
            };
        },
        StatusCode::ProxyError => {
            response = Response::ProxyError {
                info: header.meta
            };
        },
        StatusCode::SlowDown => {
            response = Response::SlowDown {
                info: header.meta
            };
        },
        StatusCode::PermanentFailure => {
            response = Response::PermanentFailure {
                info: header.meta
            };
        },
        StatusCode::NotFound => {
            response = Response::NotFound {
                info: header.meta
            };
        },
        StatusCode::Gone => {
            response = Response::Gone {
                info: header.meta
            };
        },
        StatusCode::ProxyReqRefused => {
            response = Response::ProxyReqRefused {
                info: header.meta
            };
        },
        StatusCode::BadRequest => {
            response = Response::BadRequest {
                info: header.meta
            };
        },
        StatusCode::ClientCertRequired => {
            response = Response::ClientCertRequired {
                info: header.meta
            };
        },
        StatusCode::TransientCertRequested => {
            response = Response::TransientCertRequested {
                info: header.meta
            };
        },
        StatusCode::AuthorizedCertRequired => {
            response = Response::AuthorizedCertRequired {
                info: header.meta
            };
        },
        StatusCode::CertNotAccepted => {
            response = Response::CertNotAccepted {
                info: header.meta
            };
        },
        StatusCode::FutureCertRejected => {
            response = Response::FutureCertRejected {
                info: header.meta
            };
        },
        StatusCode::ExpiredCertRejected => {
            response = Response::ExpiredCertRejected {
                info: header.meta
            };
        },
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