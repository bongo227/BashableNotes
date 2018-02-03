import React from "react";
import ReactDOM from "react-dom";
import "./style.css";

const SubMenu = ({children}) => (
	<ul className="uk-nav-sub uk-nav-parent-icon" uk-nav="multiple: true">
		{children}
	</ul>
);

const File = ({name, path}) => (
	<li>
		<a href="#" onClick={() => {
			console.log(`Clicked on ${name} with path ${path}`);
		}}><span uk-icon="icon: file" className="uk-margin-small-right"></span>{name}</a>
	</li>
);

const Folder = ({name, children}) => (
	<li className="uk-parent">
		<a href="#"><span uk-icon="icon: folder" className="uk-margin-small-right"></span>{name}</a>
		<SubMenu>
			{children}
		</SubMenu>
	</li>
);

const FileTree = ({tree}) => {
	let recurse_tree = (tree) => {
		return tree.map((item, index) => {
			if ('path' in item) {
				return <File key={item.name+index} name={item.name} path={item.path} />;
			} else {
				return <Folder key={item.name+index} name={item.name}>{recurse_tree(item.subtree)}</Folder>;
			}
		});
	};

	return <div className="uk-width-1-2@s uk-width-2-5@m file-tree">
		<ul className="uk-nav-default uk-nav-parent-icon uk-width-medium uk-nav" uk-nav="multiple: true">
			{recurse_tree(tree)}
		</ul>
	</div>;
};

const Container = ({children}) => (
	<div className="uk-container">{children}</div>
);

const Spinner = ({}) => (
	<div uk-spinner=""></div>
);

const Document = ({markdown}) => {
	return <div style={{marginLeft: 350}} dangerouslySetInnerHTML={{__html: markdown}}></div>
}

class HelloMessage extends React.Component {
	render() {
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
	}
}

var mountNode = document.getElementById("app");
ReactDOM.render(<HelloMessage name="Jane" />, mountNode);