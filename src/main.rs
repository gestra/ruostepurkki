mod gemini;
mod certificates;
extern crate mime;

fn main() {
    let r = match gemini::make_request("gemini://localhost/") {
        Ok(o) => o,
        Err(e) => { println!("Error: {}", e); return }
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
