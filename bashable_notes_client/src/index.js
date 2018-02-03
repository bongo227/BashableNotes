import React from "react";
import ReactDOM from "react-dom";

import { FileTree } from './filetree';
import { Container, Spinner } from './uikit';
import "./style.css";

const Document = ({markdown}) => {
	return <div style={{marginLeft: 350}} dangerouslySetInnerHTML={{__html: markdown}}></div>
}

const App = () => {
	let tree = [{
		name: "folder1",
		subtree: [
			{
				name: "file1",
				path: "/a/b/c"
			}, 
			{
				name: "file2",
				path: "/a/b/c"
			}
		],
	},
	{
		name: "file3",
		path: "/a/b/c"	
	}];
		
	return <Container>
		<FileTree tree={tree}/>
		<Document markdown="<h1>Hello react!</h1>"/>
	</Container>
};

let mount_node = document.getElementById("app");
ReactDOM.render(<App />, mount_node);