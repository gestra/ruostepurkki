use crate::gemini;

use std::io;

extern crate regex;
use regex::Regex;

#[derive(Clone, Copy, PartialEq, Debug)]
enum CommandType {
    Go,
}

struct UICommand {
    command_type: CommandType,
    main: Option<String>
}

fn process_input(input: String) -> Option<UICommand> {
    let go_regex = Regex::new(r"^go?\s+([^\s]+)").unwrap();

    if go_regex.is_match(&input) {
        let groups = go_regex.captures(&input).unwrap();
        let url = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());
        if gemini::is_valid_gemini_url(&url) {
            return Some(UICommand{command_type: CommandType::Go, main: Some(url)});
        }
        else {
            println!("Not a valid gemini URL: {}", url);
            return None;
        }
        
    }
    else {
        println!("What?");
        return None;
    }
}

pub fn main_ui() {
    loop {
        let mut input = String::new();
        let command = match io::stdin().read_line(&mut input) {
            Ok(_) => {
                match process_input(input) {
                    Some(c) => c,
                    None => {
                        println!("No command found");
                        continue;
                    }
                }
            }
            Err(error) => {
                println!("Error reading input: {}", error);
                continue;
            }
        };

        if command.command_type == CommandType::Go {
            let url = command.main.unwrap();
            println!("Going to {}", url);
            let r = match gemini::make_request(&url) {
                Ok(o) => o,
                Err(e) => {
                    println!("Error: {}", e);
                    continue;
                }
            };
            match r.status {
                gemini::StatusCode::Input => {
                    println!("Server asked for user input. Not yet implemented");
                },
                gemini::StatusCode::SensitiveInput => {
                    println!("Server asked for sensitive user input. Not yet implemented");
                },
                gemini::StatusCode::Success => {
                    let meta = r.meta.unwrap_or("".to_string());
                    let mime = &meta.parse::<mime::Mime>().unwrap();
                    if mime.type_() == "text" && mime.subtype() == "gemini" {
                        let doc = gemini::parse_gemini_doc(&String::from_utf8(r.contents.unwrap()).unwrap());
                        gemini::print_gemini_doc(&doc);
                    }
                },
                gemini::StatusCode::SuccessEndCert => {
                    println!("End of client certificate session. Not yet implemented");
                    let meta = r.meta.unwrap_or("".to_string());
                    let mime = &meta.parse::<mime::Mime>().unwrap();
                    if mime.type_() == "text" && mime.subtype() == "gemini" {
                        let doc = gemini::parse_gemini_doc(&String::from_utf8(r.contents.unwrap()).unwrap());
                        gemini::print_gemini_doc(&doc);
                    }
                },
                gemini::StatusCode::TemporaryFailure => {
                    println!("Temporary failure {}: Temporary failure", gemini::StatusCode::TemporaryFailure as u8);
                },
                gemini::StatusCode::ServerUnavailable => {
                    println!("Temporary failure {}: Server unavailable", gemini::StatusCode::ServerUnavailable as u8);
                },
                gemini::StatusCode::CgiError => {
                    println!("Temporary failure {}: CGI error", gemini::StatusCode::CgiError as u8);
                },
                gemini::StatusCode::ProxyError => {
                    println!("Temporary failure {}: Proxy error", gemini::StatusCode::ProxyError as u8);
                },
                gemini::StatusCode::SlowDown => {
                    println!("Temporary failure {}: Slow down", gemini::StatusCode::SlowDown as u8);
                },
                gemini::StatusCode::PermanentFailure => {
                    println!("Permanent failure {}: Temporary failure", gemini::StatusCode::PermanentFailure as u8);
                },
                gemini::StatusCode::NotFound => {
                    println!("Permanent failure {}: Not found", gemini::StatusCode::NotFound as u8);
                },
                gemini::StatusCode::Gone => {
                    println!("Permanent failure {}: Gone", gemini::StatusCode::Gone as u8);
                },
                gemini::StatusCode::ProxyReqRefused => {
                    println!("Permanent failure {}: Proxy request refused", gemini::StatusCode::ProxyReqRefused as u8);
                },
                gemini::StatusCode::BadRequest => {
                    println!("Permanent failure {}: Bad request", gemini::StatusCode::BadRequest as u8);
                },
                gemini::StatusCode::ClientCertRequired => {
                    println!("Client certificate required. Not yet implemented.");
                },
                gemini::StatusCode::TransientCertRequested => {
                    println!("Trancient certificate requested. Not yet implemented.");
                },
                gemini::StatusCode::AuthorizedCertRequired => {
                    println!("Authorized certificate requested. Not yet implemented.");
                },
                gemini::StatusCode::CertNotAccepted => {
                    println!("Certificate not accepted. Not yet implemented.");
                },
                gemini::StatusCode::FutureCertRejected => {
                    println!("Certificate not accepted because its validity start date is in the future. Not yet implemented.");
                },
                gemini::StatusCode::ExpiredCertRejected => {
                    println!("Expired certificate rejected. Not yet implemented.");
                },
                gemini::StatusCode::RedirectPerm | gemini::StatusCode::RedirectTemp => {
                    // Redirect responses should never reach this far
                    assert!(false);
                },
            }

        }

    }
}