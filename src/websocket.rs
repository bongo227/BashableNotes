use std::str::from_utf8;

use ws::{CloseCode, Error, ErrorKind, Frame, Handler, Handshake, Message, OpCode, Result, Sender};
use ws::util::{Timeout, Token};

use time;

use parser::MarkdownRenderer;
use regex;
use serde_json::to_string;

use std::ops::{Generator, GeneratorState};

use std::thread;
use std::path::Path;

const PING: Token = Token(1);
const EXPIRE: Token = Token(2);

// Server WebSocket handler
pub struct Server {
    pub out: Sender,
    pub ping_timeout: Option<Timeout>,
    pub expire_timeout: Option<Timeout>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonMsg {
    id: String,
    data: String,
}

struct OutputBlock {
    output_type: OutputType,
    index: usize,
    output: String,
}

pub enum OutputType {
    Stdout,
    Stderr,
}

impl Handler for Server {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // schedule a timeout to send a ping every 5 seconds
        try!(self.out.timeout(5_000, PING));
        // schedule a timeout to close the connection if there is no activity for 30 seconds
        self.out.timeout(30_000, EXPIRE)
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let out = self.out.clone();
        
        let thread_send = move |msg: Message| {
            let out = out.clone();
            thread::spawn(move || {
                out.send(msg).unwrap();
            });
        };

        thread::spawn(move || {
            println!("Server got message '{}'. ", msg);

            if let Ok(text) = msg.into_text() {
                if text == "init" {
                    let mut md_renderer = MarkdownRenderer::new();

                    // send file tree
                    let html = md_renderer.build_file_tree();
                    let json_msg = JsonMsg {
                        id: String::from("file-tree"),
                        data: html,
                    };
                    let json_str = to_string(&json_msg).unwrap();
                    let msg = Message::text(json_str);
                    thread_send(msg);

                } else if text.starts_with("get ") {
                    let path = &Path::new(&text[4..]);
                    let mut md_renderer = MarkdownRenderer::new();

                    // send file tree
                    let html = md_renderer.build_file_tree();
                    let json_msg = JsonMsg {
                        id: String::from("file-tree"),
                        data: html,
                    };
                    let json_str = to_string(&json_msg).unwrap();
                    let msg = Message::text(json_str);
                    thread_send(msg);

                    // send markdown
                    let html = md_renderer.parse_markdown(path);
                    let json_msg = JsonMsg {
                        id: String::from("document"),
                        data: html,
                    };
                    let json_str = to_string(&json_msg).unwrap();
                    let msg = Message::text(json_str);
                    thread_send(msg);

                    let mut output_generator = || {
                        for block in &md_renderer.blocks {
                            if let Some(ref cmd) = block.options.cmd {
                                let (stdout, stderr) = md_renderer.exec_cmd(
                                    &md_renderer.docker,
                                    &md_renderer.container_id,
                                    &cmd,
                                );

                                if stdout != "" {
                                    let output_html = format!(
                                        "{}<pre><code class=\"language-nohighlight hljs\">{}</code></pre>{}", 
                                        md_renderer.collabsible_wrapper_begin("Output", &block.options.cmd.clone().unwrap_or_default()), 
                                        stdout, 
                                        md_renderer.collabsible_wrapper_end());

                                    yield OutputBlock {
                                        output_type: OutputType::Stdout,
                                        index: block.index,
                                        output: output_html,
                                    };
                                }

                                if stderr != "" {
                                    let output_html = format!(
                                        "{}<pre><code class=\"language-nohighlight hljs\">{}</code></pre>{}", 
                                        md_renderer.collabsible_wrapper_begin("Error", &block.options.cmd.clone().unwrap_or_default()),
                                        stderr,
                                        md_renderer.collabsible_wrapper_end());

                                    yield OutputBlock {
                                        output_type: OutputType::Stdout,
                                        index: block.index,
                                        output: output_html,
                                    };
                                }
                            }
                        }

                        return ();
                    };

                    // send command outputs
                    loop {
                        match output_generator.resume() {
                            GeneratorState::Yielded(output) => {
                                let json_msg = JsonMsg {
                                    id: format!("{}", output.index),
                                    data: output.output,
                                };

                                let json_str = to_string(&json_msg).unwrap();
                                let msg = Message::text(json_str);
                                thread_send(msg);
                            }
                            GeneratorState::Complete(_) => break,
                        }
                    }

                    md_renderer.stop_docker_container();
                }
            }
        });

        self.out.send(Message::text("unknown message"))
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("WebSocket closing for ({:?}) {}", code, reason);

        // NOTE: This code demonstrates cleaning up timeouts
        if let Some(t) = self.ping_timeout.take() {
            self.out.cancel(t).unwrap();
        }
        if let Some(t) = self.expire_timeout.take() {
            self.out.cancel(t).unwrap();
        }

        // println!("Shutting down server after first connection closes.");
        // self.out.shutdown().unwrap();
    }

    fn on_error(&mut self, err: Error) {
        // Shutdown on any error
        println!("Shutting down server for error: {}", err);
        self.out.shutdown().unwrap();
    }

    fn on_timeout(&mut self, event: Token) -> Result<()> {
        match event {
            // PING timeout has occured, send a ping and reschedule
            PING => {
                try!(self.out.ping(time::precise_time_ns().to_string().into()));
                self.ping_timeout.take();
                self.out.timeout(5_000, PING)
            }
            // EXPIRE timeout has occured, this means that the connection is inactive, let's close
            EXPIRE => self.out.close(CloseCode::Away),
            // No other timeouts are possible
            _ => Err(Error::new(
                ErrorKind::Internal,
                "Invalid timeout token encountered!",
            )),
        }
    }

    fn on_new_timeout(&mut self, event: Token, timeout: Timeout) -> Result<()> {
        // Cancel the old timeout and replace.
        if event == EXPIRE {
            if let Some(t) = self.expire_timeout.take() {
                try!(self.out.cancel(t))
            }
            self.expire_timeout = Some(timeout)
        } else {
            // This ensures there is only one ping timeout at a time
            if let Some(t) = self.ping_timeout.take() {
                try!(self.out.cancel(t))
            }
            self.ping_timeout = Some(timeout)
        }

        Ok(())
    }

    fn on_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        // If the frame is a pong, print the round-trip time.
        // The pong should contain data from out ping, but it isn't guaranteed to.
        if frame.opcode() == OpCode::Pong {
            if let Ok(pong) = try!(from_utf8(frame.payload())).parse::<u64>() {
                let now = time::precise_time_ns();
                println!("RTT is {:.3}ms.", (now - pong) as f64 / 1_000_000f64);
            } else {
                println!("Received bad pong.");
            }
        }

        // Some activity has occured, so reset the expiration
        try!(self.out.timeout(30_000, EXPIRE));

        // Run default frame validation
        DefaultHandler.on_frame(frame)
    }
}

// For accessing the default handler implementation
struct DefaultHandler;

impl Handler for DefaultHandler {}
