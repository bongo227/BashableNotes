import React from "react";
import ReactDOM from "react-dom";

import { FileTree } from './filetree';
import { Container, Spinner } from './uikit';
import "./style.css";

const Document = ({markdown, socket}) => {
	return <div style={{marginLeft: 350}} dangerouslySetInnerHTML={{__html: markdown}}></div>
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