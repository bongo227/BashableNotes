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
            } else if ("Output" in json_msg) {
				console.log("New Output message");
					
				let insert_output = (title, output) => {
					// remove old input nodes	
					let nodes = document.querySelectorAll(`#${json_msg.Output.id} > li`);
					if (nodes.length > 0) {
						document.getElementById(json_msg.Output.id).innerHTML = nodes[0].outerHTML;
					}

					document.getElementById(json_msg.Output.id).innerHTML += `
					<li class="uk-open">
						<a class="uk-accordion-title uk-text-small" href="#"><span class="uk-text-bold">${title}</span> <span class="uk-text-muted">command</span></a>
						<div class="uk-accordion-content">
							<pre><code class="language-nohighlight hljs">${output}</code></pre>
						</div>
					</li>`;
				}
				
				if (json_msg.Output.stdout != "") insert_output("Output", json_msg.Output.stdout);
				if (json_msg.Output.stderr != "") insert_output("Error", json_msg.Output.stderr);
			}
        } catch (e) {}
    }

	render() {
		return <div 
			style={{marginLeft: 220}} 
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

		this.state.socket.addEventListener("message", (msg) => {
			console.log("Message from server: {}", msg);
		});
	}
	
	render() {
		return <Container>
			<FileTree socket={this.state.socket}/>
			<Document socket={this.state.socket}/>
		</Container>
	}
}

let mount_node = document.getElementById("app");
ReactDOM.render(<App />, mount_node);