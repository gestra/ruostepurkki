mod gemini;
mod certificates;
extern crate mime;

fn main() {
    //certificates::create_db();
/*
    let r = match gemini::make_request(&"gemini://localhost/".to_string()) {
        Ok(o) => o,
        Err(e) => { println!("Error: {}", e); return }
    };

    if r.status == gemini::StatusCode::Success {
        let meta = r.meta.unwrap_or("".to_string());
        let mime = &meta.parse::<mime::Mime>().unwrap();
        if mime.type_() == "text" && mime.subtype() == "gemini" {
            let doc = gemini::parse_gemini_doc(&String::from_utf8(r.contents.clone().unwrap()).unwrap());
        }

    }
*/
    let t = "# Level 1 heading\n\
    ##Level 2 heading\n\
    ### Level 3 heading";
    let r = gemini::parse_gemini_doc(&t);

    for l in r {
        println!("main: |{}|", l.main.unwrap());
    }
}
