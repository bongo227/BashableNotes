use pulldown_cmark::{html, Event, Options, Parser, Tag};
use docker;
use std::path::Path;
use std::borrow::Cow;
use std::fs::File;
use std::fs;
use std::io::{Read, Write};
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use serde_json;

pub struct Renderer {
    notebook_dir: PathBuf,
    container: Option<docker::Container>,
    blocks: Vec<CodeBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeBlockOptions {
    hide: Option<bool>,
    name: Option<String>,
    cmd: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    id: String,
    options: CodeBlockOptions,
    start_index: usize,
    end_index: usize,
    code: String,
}

#[derive(Serialize, Deserialize)]
pub enum FileTree {
    File { name: String, path: String },
    Folder { name: String, subtree: Vec<FileTree> },
}

impl CodeBlock {
    fn new(index: usize) -> Self {
        CodeBlock {
            id: format!("block-{}", index),
            options: CodeBlockOptions::default(),
            start_index: index,
            end_index: 0,
            code: String::new(),
        }
    }

    fn push_code(&mut self, code: &str) {
        self.code.push_str(code);
    }
}

impl Renderer {
    pub fn new() -> Self {
        let notebook_dir = env::current_dir().unwrap();
        info!("created notebook directory");

        let env_exec_cmd = env::var("EXEC_CMD").unwrap_or_default() == "1";
        info!("enviroment variable EXEC_CMD = {}", env_exec_cmd);

        Renderer {
            blocks: Vec::new(),
            container: None,
            notebook_dir,
        }
    }

    pub fn clean_up(self) -> () {
        self.container.map(|c| c.kill());
    }

    fn parse<'a>(&self, markdown: &'a str) -> (Vec<CodeBlock>, Vec<Event<'a>>) {
        let mut blocks: Vec<CodeBlock> = Vec::new();
        let mut in_block = false;
        let mut first_line = false;

        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let events: Vec<Event> = parser
            .into_iter()
            .enumerate()
            .map(|(index, event)| {
                match event {
                    Event::Start(Tag::CodeBlock(_)) => {
                        let mut block = CodeBlock::new(index);
                        blocks.push(block);
                        in_block = true;
                        first_line = true;
                    }

                    Event::Text(ref text) => {
                        if in_block {
                            blocks
                                .last_mut()
                                .map(|block| if first_line {
                                    first_line = false;
                                    let result: serde_json::Result<CodeBlockOptions> = serde_json::from_str(text);
                                    if let Ok(options) = result {
                                        block.options = options;
                                    } else {
                                        block.push_code(&text);
                                    }
                                } else {
                                    block.push_code(&text);
                                });
                        }
                    }

                    Event::End(Tag::CodeBlock(_)) => {
                        if in_block {
                            blocks
                                .last_mut()
                                .map(|block| block.end_index = index);
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

    pub fn render_file_tree(&self) -> Vec<FileTree> {
        fn recurse_directorys(current_dir: PathBuf) -> Vec<FileTree> {
            let mut tree = Vec::new();
            
            for path in fs::read_dir(current_dir).unwrap() {
                let path = path.unwrap();
                let file_name: OsString = path.file_name();
                let file_name = file_name.to_str().unwrap();
                if path.path().is_dir() {
                    tree.push(FileTree::Folder{name:file_name.to_string(), subtree:recurse_directorys(path.path())});
                } else {
                    let path = path.path();
                    let full_path = path.to_str().unwrap();
                    tree.push(FileTree::File{name:file_name.to_string(), path: full_path.to_string()})
                }
            }

            tree
        }

        info!("building file tree");
        let current_dir = env::current_dir().unwrap();
        let file_tree = recurse_directorys(current_dir);
        info!("file tree build");

        file_tree
    }

    pub fn render(&mut self, markdown_path: &Path) -> String {
        info!("rendering started");

        // read markdown
        info!("reading markdown file");
        let mut f = File::open(markdown_path).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        info!("markdown file read");

        // parse markdown
        info!("parsing markdown");
        let (blocks, mut events) = self.parse(&contents);
        self.blocks = blocks.clone();
        info!("markdown parsed");

        // save files
        for block in blocks.clone() {
            if let Some(ref file_name) = block.options.name {
                let path = self.notebook_dir.join(file_name);
                let mut f = File::create(path).unwrap();
                f.write_all(block.code.as_bytes()).unwrap();
                f.sync_all().unwrap();

                info!(r#"saved file "{}""#, file_name);
            }
        }

        // wrap code blocks
        let mut insert_offset = 0;

        let mut insert_html = | events: &mut Vec<Event>, index: usize, html: String | {
            events.insert(index + insert_offset, Event::Html(Cow::from(html)));
            insert_offset += 1;
        };

        info!("wrapping code blocks");
        for block in blocks {
            let block_wrapper_begin = format!(
                r#"<ul uk-accordion="multiple: true" id="{}">"#,
                block.id
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

    pub fn execution_finished(&self) -> bool {
        self.blocks.len() == 0
    }

    pub fn execute(&mut self) -> Option<(String, (String, String))> {
        if self.container.is_none() {
            // create docker container
            let docker_file = self.notebook_dir.join("Dockerfile");
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
                    let ie = self.internal_error();
                    return Some((ie.clone(), (ie.clone(), ie.clone())));
                },
            };
            info!("docker image built");        

            info!("starting docker container");
            let container = docker::Container::start(image, &self.notebook_dir);
            self.container = container.ok();
            let container = self.container.clone().unwrap();
            info!("docker container {} started", container.id());
            // thread::sleep_ms(5000);
        }

        if self.container.is_none() {
            return None;
        } else {
            let container = self.container.clone().unwrap();
            let block = self.blocks.pop();

            match block {
                Some(block) => {
                    let result = match block.options.cmd {
                        Some(ref cmd) => {
                            info!("executing command: {}", cmd);
                            container.exec(&cmd, &block.code)
                        },
                        None => return None,
                    };
                    
                    Some((block.id, result.unwrap()))
                },
                None => {
                    debug!("block {:?} doesnt have a command", block);
                    None
                },
            }
        }

    }
}
