import React from "react";
import ReactDOM from "react-dom";

import { FileTree } from './filetree';
import { Container, Spinner } from './uikit';
import "./style.css";

class Document extends React.Component {
	constructor(props) {
		super(props);
		this.state = {markdown: "Open a file"};
		this.props.socket.addEventListener("message", (msg) => this.new_message(msg));
	}

	new_message(msg) {
        try {
            let json_msg = JSON.parse(msg.data)
            if ("Markdown" in json_msg) {
                console.log("New Markdown message");
                this.setState({
                    markdown: json_msg.Markdown.markdown
                });
            }
        } catch (e) {}
    }

	render() {
		return <div 
			style={{marginLeft: 350}} 
			dangerouslySetInnerHTML={{__html: this.state.markdown}}>
		</div>
	} 
}

class App extends React.Component {
	constructor(props) {
		super(props);
		this.state = {
			socket: new WebSocket("ws://127.0.0.1:3012")
		};
		
		this.state.socket.addEventListener("open", () => {
			this.state.socket.send("\"GetTree\"");
		});
	}
	
	render() {
		return <Container>
			<FileTree socket={this.state.socket}/>
			<Document markdown="<h1>Hello react!</h1>" socket={this.state.socket}/>
		</Container>
	}
}

let mount_node = document.getElementById("app");
ReactDOM.render(<App />, mount_node);