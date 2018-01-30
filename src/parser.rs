use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use std::borrow::Cow;
use serde_json;

use std::io::{self, Write};
use tempdir::TempDir;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeBlock {
    hide: Option<bool>,
    name: Option<String>,
    cmd: Option<String>,
}

pub fn parse_markdown() -> String {
    let notebook_dir = TempDir::new("notebook").unwrap();

    let mut f = File::open(Path::new("res/test.md")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let options = Options::all();

    let parser = Parser::new_ext(&contents, options);

    fn get_blocks<'a>(
        notebook_dir: &'a TempDir,
        parser: Parser<'a>,
    ) -> (Vec<Event<'a>>, Vec<(usize, CodeBlock)>) {
        let mut code = String::new();
        let mut code_block = None;
        let mut blocks: Vec<(usize, CodeBlock)> = Vec::new();

        let events: Vec<Event> = parser
            .into_iter()
            .enumerate()
            .map(|(index, event)| match event {
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
                            code_block = Some(block);
                        }
                        Err(_) => {}
                    }

                    Event::Start(Tag::CodeBlock(Cow::from(language)))
                }

                Event::Text(text) => {
                    if let Some(_) = code_block {
                        code.push_str(&text);
                    }
                    Event::Text(text)
                }

                Event::End(Tag::CodeBlock(_)) => {
                    println!("code: {}\n,block: {:?}", code, code_block);

                    if let Some(ref block) = code_block {
                        // Save file
                        match block.name {
                            Some(ref file_name) => {
                                let path = notebook_dir.path().join(file_name);
                                let mut f = File::create(path).unwrap();
                                f.write_all(code.as_bytes()).unwrap();
                                f.sync_all().unwrap();
                            }
                            None => {}
                        }

                        blocks.push((index, block.clone()));
                    }

                    code = String::new();
                    event
                }

                _ => event,
            })
            .collect();

        (events, blocks)
    }

    let (mut events, blocks) = get_blocks(&notebook_dir, parser);
    let mut insert_offset = 0;
    for &(ref index, ref block) in blocks.iter() {
        match block.cmd {
            Some(ref cmd) => {
                println!("Running cmd: {}", cmd);

                let mut cmd_parts = cmd.split_whitespace();
                let program = cmd_parts.next().unwrap();
                let args: Vec<&str> = cmd_parts.collect();

                // Run command
                let output = Command::new(program)
                    .current_dir(notebook_dir.path())
                    .args(args)
                    .output()
                    .expect("ls command failed to start");

                let stdout = String::from_utf8_lossy(&output.stdout);

                // Insert node for output
                let output_html = format!(
                    "<pre><code class=\"language-nohighlight hljs\">{}</code></pre>",
                    stdout
                );
                events.insert(
                    index + 1 + insert_offset,
                    Event::Html(Cow::from(output_html)),
                );
                insert_offset += 1;
            }
            None => println!("No command"),
        }
    }

    // Transform events back into interator
    let parser = events.into_iter();

    // let parser = parser.map(|event| {println!("event: {:?}", event); event});

    let mut html_buf = String::new();
    html::push_html(&mut html_buf, parser);

    let code_highlighting = r#"
        <link rel="stylesheet" href="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/styles/atom-one-dark.min.css">
        <script src="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/highlight.min.js"></script>
        <script>hljs.initHighlightingOnLoad();</script>"#;
    let style = "<link rel=\"stylesheet\" type=\"text/css\" href=\"res/style.css\">";

    format!("{}\n\n{}\n\n{}", code_highlighting, style, html_buf)
}
