use pulldown_cmark::{html, Event, Options, Parser, Tag};
use docker;
use std::path::Path;
use tempdir::TempDir;
use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::env;
use serde_json;

pub struct Renderer {
    markdown: String,
    notebook_dir: TempDir,
    env_exec_cmd: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeBlockOptions {
    hide: Option<bool>,
    name: Option<String>,
    pub cmd: Option<String>,
}

#[derive(Clone)]
pub struct CodeBlock {
    options: CodeBlockOptions,
    start_index: usize,
    end_index: usize,
    code: String,
}

impl CodeBlock {
    fn new(settings: &str) -> Self {
        let mut iter = settings.split_whitespace();
        let language = match iter.next() {
            Some(lang) => lang.to_owned(),
            None => String::from(""),
        };

        let json = &settings[language.len()..];
        let result: serde_json::Result<CodeBlockOptions> = serde_json::from_str(json);

        let options = match result {
            Ok(options) => options,
            Err(_) => CodeBlockOptions::default(),
        };

        CodeBlock {
            options: options,
            start_index: 0,
            end_index: 0,
            code: String::new(),
        }
    }

    fn set_start(&mut self, index: usize) {
        self.start_index = index;
    }

    fn set_end(&mut self, index: usize) {
        self.end_index = index;
    }

    fn push_code(&mut self, code: &str) {
        self.code.push_str(code);
    }
}

impl Renderer {
    pub fn new(file: &Path) -> io::Result<Self> {
        let notebook_dir = TempDir::new("notebook").unwrap();
        info!("created notebook directory");

        let env_exec_cmd = env::var("EXEC_CMD").unwrap_or_default() == "1";
        info!("enviroment variable EXEC_CMD = {}", env_exec_cmd);

        info!("reading markdown file");
        let mut f = File::open(file)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        info!("markdown file read");

        Ok(Renderer {
            markdown: contents,
            notebook_dir,
            env_exec_cmd,
        })
    }

    fn parse(&self) -> (Vec<CodeBlock>, Vec<Event>) {
        let mut blocks: Vec<CodeBlock> = Vec::new();
        let mut in_block = false;

        let options = Options::all();
        let parser = Parser::new_ext(&self.markdown, options);

        let events: Vec<Event> = parser
            .into_iter()
            .enumerate()
            .map(|(index, event)| {
                match event {
                    Event::Start(Tag::CodeBlock(ref settings)) => {
                        let mut block = CodeBlock::new(&settings.clone());
                        block.set_start(index);
                        blocks.push(block);
                        in_block = true;
                    }

                    Event::Text(ref text) => {
                        if in_block {
                            blocks
                                .last_mut()
                                .map(|block| block.push_code(&text));
                        }
                    }

                    Event::End(Tag::CodeBlock(_)) => {
                        if in_block {
                            blocks
                                .last_mut()
                                .map(|block| block.set_end(index));
                            in_block = false;
                        }
                    }

                    _ => {}
                }
                event
            })
            .collect();

        (blocks, events)
    }

    fn collabsible_wrapper_begin(&self, title: &str, subtext: &str) -> String {
        format!(r##"
            <li class="uk-open">
                <a class="uk-accordion-title uk-text-small" href="#"><span class="uk-text-bold">{}</span> <span class="uk-text-muted">{}</span></a>
                <div class="uk-accordion-content">"##,
            title, subtext
        )
    }

    fn collabsible_wrapper_end(&self) -> String {
        String::from(
            r#"
                </div>
            </li>"#,
        )
    }

    fn internal_error(&self) -> String {
        return String::from("Internal server error");
    }

    pub fn render(&self) -> String {
        info!("rendering started");

        // parse markdown
        info!("parsing markdown");
        let (blocks, mut events) = self.parse();
        info!("markdown parsed");

        // save files
        for block in blocks.clone() {
            if let Some(ref file_name) = block.options.name {
                let path = self.notebook_dir.path().join(file_name);
                let mut f = File::create(path).unwrap();
                f.write_all(block.code.as_bytes()).unwrap();
                f.sync_all().unwrap();

                info!(r#"saved file "{}""#, file_name);
            }
        }

        // create docker container
        let docker_file = self.notebook_dir.path().join("Dockerfile");
        if !docker_file.as_path().exists() {
            info!("no Dockerfile, creating default Dockerfile");

            let mut f = File::create(&docker_file).unwrap();
            f.write_all("FROM ubuntu:latest".as_bytes()).unwrap();
            f.sync_all().unwrap();

            info!("created default Dockerfile");
        }

        info!("building docker image");
        let image = match docker::Image::build("notebook-image", &docker_file) {
            Ok(image) => image,
            Err(err) => {
                error!("error building docker image: {}", err);
                return self.internal_error();
            },
        };
        
        info!("docker image built");

        info!("starting docker container");
        let container = docker::Container::start(image, self.notebook_dir.path());
        info!("docker container started");

        // wrap code blocks
        let mut insert_offset = 0;

        let mut insert_html = | events: &mut Vec<Event>, index: usize, html: String | {
            events.insert(index + insert_offset, Event::Html(Cow::from(html)));
            insert_offset += 1;
        };

        info!("wrapping code blocks");
        for (index, block) in blocks.into_iter().enumerate() {
            let block_wrapper_begin = format!(
                r#"<ul uk-accordion="multiple: true" id="block-{}">"#,
                index
            );
            let block_wrapper_end = String::from(r#"</ul>"#);

            // begin outer wrapper
            insert_html(
                &mut events,
                block.start_index,
                block_wrapper_begin.clone()
            );

            // wrap code
            insert_html(
                &mut events,
                block.start_index,
                self.collabsible_wrapper_begin(
                    "Input",
                    &block.options.name.clone().unwrap_or_default(),
                )
            );
            insert_html(
                &mut events,
                block.end_index + 1,
                self.collabsible_wrapper_end()
            );

            // end outer wrapper
            insert_html(
                &mut events,
                block.end_index + 1,
                block_wrapper_end.clone()
            );
        }
        info!("code blocks wrapped");

        // render html
        info!("rendering html");
        let mut html_buf = String::new();
        html::push_html(&mut html_buf, events.into_iter());
        info!("html rendered");

        html_buf
    }
}
