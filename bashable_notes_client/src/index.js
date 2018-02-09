import React from "react";
import ReactDOM from "react-dom";
import MediaQuery from 'react-responsive';
import hljs from 'highlight.js';

import { FileTree } from './filetree';
import { Container, Spinner } from './uikit';
import "./style.css";
import "highlight.js/styles/default.css";

export const FileTreeWidth = 350;

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
					path: json_msg.Markdown.path,
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
			
				// refresh images
				let images = document.querySelectorAll("img");
				for (let i = 0; i < images.length; i++) {
					let src = images[i].src;
					src = src.split("?")[0] + "?t=" + new Date().getTime();
					images[i].src = src;
				}
				
			} else if ("FileUpdate" in json_msg) {
				console.log("New FileUpdate message");	
				try {
					if (this.state.path == json_msg.FileUpdate.path){
						let req = {
							"OpenFile": {
								"path": this.state.path
							}
						}
						this.props.socket.send(JSON.stringify(req));
					}
				}  catch (e) {
					console.warn("Failed to send OpenFile message: {}", e);
				}
			}
        } catch (e) {}
	}
	
	componentDidUpdate() {
		hljs.initHighlighting.called = false;
		hljs.initHighlighting();
	}

	render() {
		return (
			<div>
				<MediaQuery minWidth={991}>
					<div 
						style={{marginLeft: FileTreeWidth}} 
						dangerouslySetInnerHTML={{__html: this.state.markdown}}>
					</div>
				</MediaQuery>
				<MediaQuery maxWidth={991}>
					<div 
						dangerouslySetInnerHTML={{__html: this.state.markdown}}>
					</div>
				</MediaQuery>
			</div>
		);
	} 
}

const NavBar = ({sideNavId}) => (
	<div className="uk-navbar-container uk-navbar-sticky" uk-navbar={""} uk-sticky={""}>
		<div className="uk-navbar-left">
			<a className="uk-navbar-item uk-logo">BashableNotes</a>
		</div>
		<MediaQuery maxWidth={991}>
			<div className="uk-navbar-right">
				<a className="uk-navbar-toggle" uk-navbar-toggle-icon={""} uk-toggle={""} href={"#"+sideNavId}></a>
			</div>
		</MediaQuery>
	</div>
);

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
		return (
			<div>
				<NavBar sideNavId="file-tree-nav" />
				<div style={{paddingTop: 20}}>
					<Container>
						<FileTree width={FileTreeWidth-40} sideNavId="file-tree-nav" socket={this.state.socket}/>
						<Document socket={this.state.socket}/>
					</Container>
				</div>
			</div>
		) 
	}
}

let mount_node = document.getElementById("app");
ReactDOM.render(<App />, mount_node);