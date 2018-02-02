use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use std::borrow::Cow;
use serde_json;
use std::io::Write;
use std::fs;
use std::process::Command;
use shiplift::{Docker, ExecContainerOptions};
use std::env;
use tempdir::TempDir;
use walkdir::WalkDir;
use std::borrow::Borrow;
use std::path::PathBuf;
use std::ffi::OsString;

pub struct MarkdownRenderer {
    notebook_dir: TempDir,
    pub blocks: Vec<CodeBlock>,
    pub docker: Docker,
    pub container_id: String,
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
    pub options: CodeBlockOptions,
    start_index: usize,
    end_index: usize,
    code: String,
    pub index: usize,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        // TODO: move back to a temp folder
        let notebook_dir = TempDir::new("notebook").unwrap();
        info!("created notebook directory");

        let env_exec_cmd = env::var("EXEC_CMD").unwrap_or_default() == "1";

        MarkdownRenderer {
            notebook_dir,
            blocks: Vec::new(),
            docker: Docker::new(),
            container_id: String::new(),
            env_exec_cmd,
        }
    }

    fn build_docker_container(&self) {
        info!("building docker container");

        let output = Command::new("docker")
            .current_dir(self.notebook_dir.path())
            .arg("build")
            // .arg("--no-cache")
            .arg("--network=host") // share the network with host
            .arg(".")
            .arg("-t")
            .arg("notebook-container")
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.contains("Successfully tagged notebook-container:latest") {
            panic!(
                "Failed to build docker image:\n==== stdout ====\n{}\n==== stderr ====\n{}",
                stdout, stderr
            );
        }

        info!("docker container built");
    }

    fn start_docker_container(&self) -> String {
        info!("starting docker container");

        // debug!("docker folder link: {}", format!("{}:/home/notebook", notebook_path.to_str().unwrap()));
        // debug!("nbpath: {:?}", );

        let output = Command::new("docker")
        .current_dir(self.notebook_dir.path())
        .arg("run")
        .arg("-i") // keep container alive even though we are not attached
        .arg("-d") // run in the background
        .arg("-v") // link notebook folder
        .arg(format!("{}:/home", self.notebook_dir.path().canonicalize().unwrap().to_str().unwrap()))
        .arg("--net=host") // share the network with host
        .arg("notebook-container")
        .output()
        .unwrap();

        let container_id = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr != "" {
            panic!(
                "Failed to start container!\n--stdout:\n{}\n--stderr:\n{}",
                container_id, stderr
            );
        }

        info!("docker container {} started", container_id);
        container_id.to_string()
    }

    pub fn stop_docker_container(&self) {
        // Stop the container
        if self.env_exec_cmd {
            info!("stopping container");
            let containers = self.docker.containers();
            let container = containers.get(&self.container_id);
            if let Err(err) = container.stop(None) {
                error!("error stopping container: {}", err);
            } else {
                info!("container stopped");
            }
        }
    }

    fn parse_code_block(&self, settings: String, index: usize) -> CodeBlock {
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
            start_index: index,
            end_index: 0,
            code: String::new(),
            index: index,
        }
    }

    fn extract_blocks<'b>(&self, parser: Parser<'b>) -> (Vec<Event<'b>>, Vec<CodeBlock>) {
        let mut in_block = false;
        let mut blocks = Vec::new();
        let events: Vec<Event> = parser
            .into_iter()
            .enumerate()
            .map(|(index, event)| {
                match event {
                    Event::Start(Tag::CodeBlock(ref settings)) => {
                        let settings = settings.clone().into_owned();
                        blocks.push(self.parse_code_block(settings, index));
                        in_block = true;
                    }

                    Event::Text(ref text) => {
                        if in_block {
                            blocks.last_mut().map(|block| block.code.push_str(&text));
                        }
                    }

                    Event::End(Tag::CodeBlock(_)) => {
                        if in_block {
                            blocks.last_mut().map(|block| block.end_index = index);
                            in_block = false;
                        }
                    }

                    _ => {}
                }
                event
            })
            .collect();

        (events, blocks)
    }

    pub fn exec_cmd(&self, docker: &Docker, container_id: &str, cmd: &String) -> (String, String) {
        info!("building command");
        let options = ExecContainerOptions::builder()
            .cmd(vec![
                "bash",
                "-c",
                // "echo -n \"echo VAR=$VAR on stdout\"; echo -n \"echo VAR=$VAR on stderr\" >&2",
                format!("cd home && {}", cmd).as_str(),
            ])
            .env(vec!["VAR=value"])
            .attach_stdout(true)
            .attach_stderr(true)
            .build();
        info!("command build");

        info!("executing command: {}", cmd);
        let containers = docker.containers();
        let container = containers.get(&container_id);
        let result = container.exec(&options);
        match result {
            Ok(result) => {
                info!("command executed successfully");
                (result.stdout, result.stderr)
            }
            Err(err) => {
                error!("error executing command: {}", err);
                (String::new(), String::from("BashableNotes internal error"))
            }
        }
    }

    pub fn collabsible_wrapper_begin(&self, title: &str, subtext: &str) -> String {
        format!(r##"
            <li class="uk-open">
                <a class="uk-accordion-title uk-text-small" href="#"><span class="uk-text-bold">{}</span> <span class="uk-text-muted">{}</span></a>
                <div class="uk-accordion-content">"##,
            title, subtext
        )
    }

    pub fn collabsible_wrapper_end(&self) -> String {
        String::from(
            r#"
                </div>
            </li>"#,
        )
    }

    pub fn build_file_tree(&self) -> String {
        fn recurse_directorys(current_dir: PathBuf) -> String {
            let mut menu_html = String::new();

            let inner_menu_begin =
            r#"<ul class="uk-nav-sub uk-nav-parent-icon" uk-nav="multiple: true">"#;
            let inner_menu_end = "</ul>";

            fn file_item(file: &str, name: &str) -> String {
                format!(
                    r##"
                    <li>
                        <a href="#" onclick="goto('{}')"><span uk-icon="icon: file" class="uk-margin-small-right"></span>{}</a>
                    </li>"##,
                    file,
                    name
                )
            };

            fn folder_item_begin(name: &str) -> String {
                format!(
                    r##"
                <li class="uk-parent">
                    <a href="#">
                        <span uk-icon="icon: folder" class="uk-margin-small-right"></span>{}</a>
                "##,
                    name
                )
            };

            let folder_item_end = "</li>";

            for path in fs::read_dir(current_dir).unwrap() {
                let path = path.unwrap();
                let file_name: OsString = path.file_name();
                let file_name = file_name.to_str().unwrap();
                if path.path().is_dir() {
                    menu_html.push_str(&folder_item_begin(file_name));
                    menu_html.push_str(inner_menu_begin);
                    menu_html.push_str(&recurse_directorys(path.path()));
                    menu_html.push_str(inner_menu_end);
                    menu_html.push_str(folder_item_end);
                } else {
                    let path = path.path();
                    let full_path = path.to_str().unwrap();
                    menu_html.push_str(&file_item(full_path, file_name));
                }

                // debug!("entry: {}", path.unwrap().path().display());
            }

            menu_html
        }
        info!("building file tree");
        let current_dir = env::current_dir().unwrap();
        let menu_html = recurse_directorys(current_dir);
        info!("file tree build");

        menu_html
    }

    pub fn parse_markdown(&mut self, path: &Path) -> String {
        info!("variable EXEC_CMD = {}", self.env_exec_cmd);
        
        // Read markdown file
        info!("Rendering: {}", path.display());
        let mut f = File::open(path).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        info!("read markdown file");

        // Create parser
        let options = Options::all();
        let parser = Parser::new_ext(&contents, options);

        // extract code blocks from markdown
        let (mut events, blocks) = self.extract_blocks(parser);
        self.blocks = blocks;
        info!("extracted code blocks");

        // Save code blocks to files
        if self.env_exec_cmd {
            for block in &self.blocks {
                if let Some(ref file_name) = block.options.name {
                    let path = self.notebook_dir.path().join(file_name);
                    let mut f = File::create(path).unwrap();
                    f.write_all(block.code.as_bytes()).unwrap();
                    f.sync_all().unwrap();

                    info!("saved file: {}", file_name);
                }
            }
        }

        // Setup docker enviroment
        if self.env_exec_cmd {
            let docker_file = self.notebook_dir.path().join("Dockerfile");
            debug!("docker file location: {}", docker_file.display());
            if !docker_file.as_path().exists() {
                info!("no Dockerfile, creating default");

                let mut f = File::create(docker_file).unwrap();
                f.write_all("FROM ubuntu:latest".as_bytes()).unwrap();
                f.sync_all().unwrap();

                info!("created docker file");
            }

            self.build_docker_container();
            self.container_id = self.start_docker_container();
            assert_ne!(self.container_id, "")
        }

        let mut insert_offset = 0;

        let insert_html =
            |index: usize, insert_offset: &mut usize, html: String, events: &mut Vec<Event>| {
                events.insert(index + *insert_offset, Event::Html(Cow::from(html)));
                *insert_offset += 1;
            };

        for block in &self.blocks {
            let block_wrapper_begin = format!(
                r#"<ul uk-accordion="multiple: true" id="block-{}">"#,
                block.index
            );
            let block_wrapper_end = String::from(r#"</ul>"#);

            // begin outer wrapper
            insert_html(
                block.start_index,
                &mut insert_offset,
                block_wrapper_begin.clone(),
                &mut events,
            );

            // wrap code
            insert_html(
                block.start_index,
                &mut insert_offset,
                self.collabsible_wrapper_begin(
                    "Input",
                    &block.options.name.clone().unwrap_or_default(),
                ),
                &mut events,
            );
            insert_html(
                block.end_index + 1,
                &mut insert_offset,
                self.collabsible_wrapper_end(),
                &mut events,
            );

            // end outer wrapper
            insert_html(
                block.end_index + 1,
                &mut insert_offset,
                block_wrapper_end.clone(),
                &mut events,
            );
        }

        info!("building html");
        let mut html_buf = String::new();
        html::push_html(&mut html_buf, events.into_iter());
        info!("html built");

        html_buf
    }
}
