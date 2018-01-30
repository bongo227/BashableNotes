use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use std::borrow::Cow;
use serde_json;

use std::io::Write;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeBlockOptions {
    hide: Option<bool>,
    name: Option<String>,
    cmd: Option<String>,
}

#[derive(Debug, Clone)]
struct CodeBlock {
    options: CodeBlockOptions,
    start_index: usize,
    end_index: usize,
}

pub fn parse_markdown() -> String {
    // let notebook_dir = TempDir::new("notebook").unwrap();
    let notebook_path = Path::new("notebook/");
    fs::create_dir_all(notebook_path).unwrap();

    let mut f = File::open(Path::new("res/test.md")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let options = Options::all();

    let parser = Parser::new_ext(&contents, options);

    fn get_blocks<'a>(
        notebook_path: &Path,
        parser: Parser<'a>,
    ) -> (Vec<Event<'a>>, Vec<CodeBlock>) {
        let mut code = String::new();
        let mut code_block = None;
        let mut blocks: Vec<CodeBlock> = Vec::new();

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
                    let result: serde_json::Result<CodeBlockOptions> = serde_json::from_str(json);

                    if let Ok(options) = result {    
                        let block = CodeBlock {
                            options: options,
                            start_index: index,
                            end_index: 0,
                        };
                        code_block = Some(block);
                    }

                    Event::Start(Tag::CodeBlock(Cow::from(language)))
                }

                Event::Text(text) => {
                    if code_block.is_some() {
                        code.push_str(&text);
                    }
                    Event::Text(text)
                }

                Event::End(Tag::CodeBlock(_)) => {
                    println!("code: {}\n,block: {:?}", code, code_block);

                    if let Some(ref mut block) = code_block {
                        block.end_index = index;

                        // Save file
                        if let Some(ref file_name) = block.options.name {
                                let path = notebook_path.join(file_name);
                                let mut f = File::create(path).unwrap();
                                f.write_all(code.as_bytes()).unwrap();
                                f.sync_all().unwrap();
                        } 

                        blocks.push(block.clone());
                    }

                    code = String::new();
                    event
                }

                _ => event,
            })
            .collect();

        (events, blocks)
    }

    let (mut events, blocks) = get_blocks(notebook_path, parser);
    let mut insert_offset = 0;
    for (index, block) in blocks.iter().enumerate() {
        match block.options.cmd {
            Some(ref cmd) => {
                println!("Running cmd: {}", cmd);

                let mut cmd_parts = cmd.split_whitespace();
                let program = cmd_parts.next().unwrap();
                let args: Vec<&str> = cmd_parts.collect();

                // Run command
                let output = Command::new(program)
                    .current_dir(notebook_path)
                    .args(args)
                    .output()
                    .expect("ls command failed to start");
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let input_name = match block.options.name {
                    Some(ref name) => format!("INPUT: {}", name),
                    None => String::from("INPUT"),
                };

                let output_name = match block.options.cmd {
                    Some(ref cmd) => format!("OUTPUT: {}", cmd),
                    None => String::from("OUTPUT"),
                };

                let error_name = match block.options.cmd {
                    Some(ref cmd) => format!("ERROR: {}", cmd),
                    None => String::from("ERROR"),
                };

                let block_wrapper_begin = String::from(
                    r#"
                    <div class="block-wrapper">"#
                );

                let block_wrapper_end = String::from(
                    r#"
                    </div>"#
                );

                // Insert wrapper
                let wrapper_begin = |id: &str, box_name: &str| {
                    format!(
                        r#"
                    <div class="wrap-collabsible code-{1}">
                        <input id="collapsible-{0}-{1}" class="toggle" type="checkbox" checked>
                        <label for="collapsible-{0}-{1}" class="lbl-toggle">{2}</label>
                        <div class="collapsible-content">
                            <div class="content-inner">"#,
                        index, id, box_name
                    )
                };

                let wrapper_end = || {
                    String::from(
                        r#"
                    </div>
                        </div>
                            </div>"#,
                    )
                };

                // TODO clean up all the inserts
                // Consider popping the elements,
                // then appending a new vector with all the custom html
                events.insert(
                    block.start_index + insert_offset,
                    Event::Html(Cow::from(block_wrapper_begin)),
                );
                insert_offset += 1;

                events.insert(
                    block.start_index + insert_offset,
                    Event::Html(Cow::from(wrapper_begin("in", &input_name))),
                );
                insert_offset += 1;

                events.insert(
                    block.end_index + insert_offset + 1,
                    Event::Html(Cow::from(wrapper_end())),
                );
                insert_offset += 1;

                // Insert node for output
                if stdout != "" {
                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(wrapper_begin("out", &output_name))),
                    );
                    insert_offset += 1;

                    let output_html = format!(
                        "<pre><code class=\"language-nohighlight hljs\">{}</code></pre>",
                        stdout
                    );
                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(output_html)),
                    );
                    insert_offset += 1;

                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(wrapper_end())),
                    );
                    insert_offset += 1;
                }

                // Insert node for error
                if stderr != "" {
                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(wrapper_begin("error", &error_name))),
                    );
                    insert_offset += 1;

                    let output_html = format!(
                        "<pre><code class=\"language-nohighlight hljs\">{}</code></pre>",
                        stderr
                    );
                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(output_html)),
                    );
                    insert_offset += 1;

                    events.insert(
                        block.end_index + insert_offset + 1,
                        Event::Html(Cow::from(wrapper_end())),
                    );
                    insert_offset += 1;
                }

                events.insert(
                    block.end_index + insert_offset + 1,
                    Event::Html(Cow::from(block_wrapper_end)),
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
