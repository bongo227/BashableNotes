import React from "react";

const SubMenu = ({ children }) => (
    <ul className="uk-nav-sub uk-nav-parent-icon" uk-nav="multiple: true">
        {children}
    </ul>
);

const File = ({ name, path, onClick }) => (
    <li>
        <a href="#" onClick={onClick}><span uk-icon="icon: file" className="uk-margin-small-right"></span>{name}</a>
    </li>
);

const Folder = ({ name, children }) => (
    <li className="uk-parent">
        <a href="#"><span uk-icon="icon: folder" className="uk-margin-small-right"></span>{name}</a>
        <SubMenu>
            {children}
        </SubMenu>
    </li>
);

export class FileTree extends React.Component {
    constructor(props) {
        super(props);
        this.state = { tree: [] };

        this.props.socket.addEventListener("message", (msg) => this.new_message(msg));
    }

    new_message(msg) {
        try {
            let json_msg = JSON.parse(msg.data)
            if ("FileTree" in json_msg) {
                console.log("New FileTree message");
                this.setState({
                    tree: json_msg.FileTree.root
                });
            }
        } catch (e) {

        }
    }

    request_file(path) {
        try {
            let req = {
                "OpenFile": {
                    "path": path,
                }
            };
            this.props.socket.send(JSON.stringify(req));
        } catch (e) {
            console.warn("Failed to send OpenFile message: {}", e);
        }
    }

    render() {
        console.log("render");
        console.dir(this.state);
        let recurse_tree = (tree) => {
            return tree.map((item, index) => {
                if ('File' in item) {
                    return <File 
                        key={item.File.name + index} 
                        name={item.File.name} 
                        path={item.File.path} 
                        onClick={() => this.request_file(item.File.path)} />;
                } else {
                    return <Folder 
                        key={item.Folder.name + index} 
                        name={item.Folder.name}>{recurse_tree(item.Folder.subtree)}</Folder>;
                }
            });
        };

        return <div className="uk-width-1-2@s uk-width-2-5@m file-tree">
            <ul className="uk-nav-default uk-nav-parent-icon uk-width-medium uk-nav" uk-nav="multiple: true">
                {recurse_tree(this.state.tree)}
            </ul>
        </div>;
    }
}