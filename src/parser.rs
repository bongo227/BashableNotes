use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use std::borrow::Cow;
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
struct CodeBlock {
    hide: Option<bool>,
    name: Option<String>,
    cmd: Option<String>,
}

pub fn parse_markdown() -> String {
    let mut f = File::open(Path::new("res/test.md")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let options = Options::all();

    let parser = Parser::new_ext(&contents, options);

    let parser = parser.map(|event| match event {
        Event::Start(Tag::CodeBlock(settings)) => {
            let settings = settings.into_owned();
            let mut iter = settings.split_whitespace();
            let language = match iter.next() {
                Some(lang) => lang.to_owned(),
                None => String::from(""),
            };

            let json = &settings[language.len()..];
            let result: serde_json::Result<CodeBlock> = serde_json::from_str(&json);

            match result {
                Ok(block) => {
                    println!("{:?}", block);
                }
                Err(_) => {}
            }

            Event::Start(Tag::CodeBlock(Cow::from(language)))
        }
        // Event::Str(text) => Event::Str(text.replace("abbr", "abbreviation")),
        _ => event,
    });

    let mut html_buf = String::new();
    html::push_html(&mut html_buf, parser);

    let code_highlighting = r#"
        <link rel="stylesheet" href="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/styles/atom-one-dark.min.css">
        <script src="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/highlight.min.js"></script>
        <script>hljs.initHighlightingOnLoad();</script>"#;
    let style = "<link rel=\"stylesheet\" type=\"text/css\" href=\"res/style.css\">";

    format!("{}\n\n{}\n\n{}", code_highlighting, style, html_buf)
}
