use std::str::from_utf8;
// use std::ops::{Generator, GeneratorState};
use std::thread;
use std::path::Path;
use ws::{CloseCode, Error, ErrorKind, Frame, Handler, Handshake, Message, OpCode, Result, Sender};
use ws::util::{Timeout, Token};
use time;
use serde_json;

use renderer::{Renderer, FileTree};

const PING: Token = Token(1);
const EXPIRE: Token = Token(2);

pub struct Server {
    pub out: Sender,
    pub ping_timeout: Option<Timeout>,
    pub expire_timeout: Option<Timeout>,
}

#[derive(Serialize, Deserialize)]
enum AppMessage {
    OpenFile { path: String },
    GetTree,
    Markdown { path: String, markdown: String },
    Output { id: String, stdout: String, stderr: String },
    Error { error: String },
    FileTree { root: Vec<FileTree> }
}

impl Handler for Server {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // schedule a timeout to send a ping every 5 seconds
        try!(self.out.timeout(5_000, PING));
        // schedule a timeout to close the connection if there is no activity for 15 minuets
        self.out.timeout(90_0000, EXPIRE)
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let out = self.out.clone();
        
        let thread_send = move |app_msg: AppMessage| {
            let out = out.clone();
            thread::spawn(move || {
                let text = serde_json::to_string(&app_msg).unwrap();
                out.send(Message::Text(text)).unwrap();
                debug!("message sent");
            });
        };

        debug!("message from client: {}", msg);

        let msg_text = match &msg.into_text() {
            &Ok(ref text) => text.clone(),
            &Err(ref err) => {
                warn!("unable to course message into text: {}", err);
                return Ok(())
            }
        };

        match serde_json::from_str(&msg_text) {
            Ok(msg) => match msg {
                AppMessage::OpenFile { path } => {
                    let mut renderer = Renderer::new();
                    thread_send(AppMessage::Markdown{
                        path: path.clone(), 
                        markdown: renderer.render(Path::new(&path))
                    });

                    thread::spawn(move || {
                        while !renderer.execution_finished() {
                            let exec_result = renderer.execute();
                            if let Some((id, (stdout, stderr))) = exec_result {
                                thread_send(AppMessage::Output{id, stdout, stderr});
                            }
                        }
                    });

                },
                AppMessage::GetTree => {
                    let renderer = Renderer::new();
                    thread_send(AppMessage::FileTree{
                        root: renderer.render_file_tree(),
                    })
                }
                _ => warn!("unexpected message")
            },
            Err(err) => warn!("unable to parse message: {}", err),
        }
        

        self.out.send(Message::text("Hi, I am the server!"))
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
    }

    fn on_error(&mut self, err: Error) {
        // Shutdown on any error
        warn!("shutting down server for error: {}", err);
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
            _ => Err(Error::new(ErrorKind::Internal, "Invalid timeout token encountered!")),
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
                // debug!("round trip time is {:.3}ms", (now - pong) as f64 / 1_000_000f64);
            } else {
                warn!("received bad pong");
            }
        }

        // Some activity has occured, so reset the expiration
        try!(self.out.timeout(30_000, EXPIRE));

        // Run default frame validation
        DefaultHandler.on_frame(frame)
    }
}

struct DefaultHandler;

impl Handler for DefaultHandler {}