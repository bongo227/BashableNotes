use std::thread;
use std::path::Path;
use ws::{CloseCode, Error, Handler, Message, Result, Sender};
use serde_json;
use renderer::{Renderer, FileTree};

pub struct Server {
    pub out: Sender,
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
                        renderer.clean_up();
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
    }

    fn on_error(&mut self, err: Error) {
        // Shutdown on any error
        warn!("shutting down server for error: {}", err);
        self.out.shutdown().unwrap();
    }
}
