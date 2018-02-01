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

pub struct MarkdownRenderer<'a> {
    notebook_path: &'a Path,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    code: String,
}

impl<'a> MarkdownRenderer<'a> {
    pub fn new() -> Self {
        // TODO: move back to a temp folder
        let notebook_path = Path::new("notebook/");

        fs::create_dir_all(notebook_path).unwrap();
        info!("created notebook directory");

        MarkdownRenderer { notebook_path }
    }

    fn build_docker_container(&self) {
        info!("building docker container");

        let output = Command::new("docker")
            .current_dir(self.notebook_path)
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
        .current_dir(self.notebook_path)
        .arg("run")
        .arg("-i") // keep container alive even though we are not attached
        .arg("-d") // run in the background
        .arg("-v") // link notebook folder
        .arg(format!("{}:/home", self.notebook_path.canonicalize().unwrap().to_str().unwrap()))
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

    fn exec_cmd(&self, docker: &Docker, container_id: &str, cmd: &String) -> (String, String) {
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

    pub fn parse_markdown(&self) -> String {
        let env_exec_cmd = env::var("EXEC_CMD").unwrap_or_default() == "1";
        info!("variable EXEC_CMD = {}", env_exec_cmd);

        // Read markdown file
        let mut f = File::open(Path::new("res/test.md")).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        info!("read markdown file");

        // Create parser
        let options = Options::all();
        let parser = Parser::new_ext(&contents, options);

        // extract code blocks from markdown
        let (mut events, blocks) = self.extract_blocks(parser);
        info!("extracted code blocks");

        // Save code blocks to files
        if env_exec_cmd {
            for block in &blocks {
                if let Some(ref file_name) = block.options.name {
                    let path = self.notebook_path.join(file_name);
                    let mut f = File::create(path).unwrap();
                    f.write_all(block.code.as_bytes()).unwrap();
                    f.sync_all().unwrap();

                    info!("saved file: {}", file_name);
                }
            }
        }

        // Setup docker enviroment
        let mut container_id = String::new();
        let docker = Docker::new();
        if env_exec_cmd {
            self.build_docker_container();
            container_id = self.start_docker_container();
            assert_ne!(container_id, "")
        }

        let mut insert_offset = 0;

        let block_wrapper_begin = String::from(r#"<div class="block-wrapper">"#);
        let block_wrapper_end = String::from(r#"</div>"#);

        let collabsible_wrapper_begin = |index: usize, id: &str, box_name: &str| {
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

        let collabsible_wrapper_end = || {
            String::from(
                r#"
                    </div>
                </div>
            </div>"#,
            )
        };

        let insert_html =
            |index: usize, insert_offset: &mut usize, html: String, events: &mut Vec<Event>| {
                events.insert(index + *insert_offset, Event::Html(Cow::from(html)));
                *insert_offset += 1;
            };

        for (index, block) in blocks.iter().enumerate() {
            // begin outer wrapper
            insert_html(
                block.start_index,
                &mut insert_offset,
                block_wrapper_begin.clone(),
                &mut events,
            );

            // wrap code
            let input_name = match block.options.name {
                Some(ref name) => format!("INPUT: {}", name),
                None => String::from("INPUT"),
            };
            insert_html(
                block.start_index,
                &mut insert_offset,
                collabsible_wrapper_begin(index, "in", &input_name),
                &mut events,
            );
            insert_html(
                block.end_index + 1,
                &mut insert_offset,
                collabsible_wrapper_end(),
                &mut events,
            );

            if env_exec_cmd {
                if let Some(ref cmd) = block.options.cmd {
                    let (stdout, stderr) = self.exec_cmd(&docker, &container_id, cmd);

                    // insert output
                    if stdout != "" {
                        let output_name = match block.options.cmd {
                            Some(ref cmd) => format!("OUTPUT: {}", cmd),
                            None => String::from("OUTPUT"),
                        };

                        let output_html = format!(
                            "{}<pre><code class=\"language-nohighlight hljs\">{}</code></pre>{}",
                            collabsible_wrapper_begin(index, "out", &output_name),
                            stdout,
                            collabsible_wrapper_end()
                        );
                        insert_html(
                            block.end_index + 1,
                            &mut insert_offset,
                            output_html,
                            &mut events,
                        );
                    }

                    // insert error output
                    if stderr != "" {
                        let error_name = match block.options.cmd {
                            Some(ref cmd) => format!("ERROR: {}", cmd),
                            None => String::from("ERROR"),
                        };

                        let output_html = format!(
                            "{}<pre><code class=\"language-nohighlight hljs\">{}</code></pre>{}",
                            collabsible_wrapper_begin(index, "error", &error_name),
                            stderr,
                            collabsible_wrapper_end()
                        );
                        insert_html(
                            block.end_index + 1,
                            &mut insert_offset,
                            output_html,
                            &mut events,
                        );
                    }
                }
            }

            // end outer wrapper
            insert_html(
                block.end_index + 1,
                &mut insert_offset,
                block_wrapper_end.clone(),
                &mut events,
            );
        }

        // Stop the container
        if env_exec_cmd {
            info!("stopping container");
            let containers = docker.containers();
            let container = containers.get(&container_id);
            if let Err(err) = container.stop(None) {
                error!("error stopping container: {}", err);
            } else {
                info!("container stopped");
            }
        }

        info!("building html");
        let mut html_buf = String::new();
        html::push_html(&mut html_buf, events.into_iter());
        info!("html built");

        
        html_buf
    }
}
