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
            if r.status == gemini::StatusCode::Success {
                let meta = r.meta.unwrap_or("".to_string());
                let mime = &meta.parse::<mime::Mime>().unwrap();
                if mime.type_() == "text" && mime.subtype() == "gemini" {
                    let doc = gemini::parse_gemini_doc(&String::from_utf8(r.contents.unwrap()).unwrap());
                    gemini::print_gemini_doc(&doc);
                }
            }

        }

    }
}