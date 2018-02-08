import React from "react";
import UIkit from "uikit";
import MediaQuery from 'react-responsive';

const SubMenu = ({ children }) => (
    <ul className="uk-nav-sub uk-nav-parent-icon" uk-nav="multiple: true">
        {children}
    </ul>
);

const File = ({ name, path, onClick }) => (
    <li>
        <a href="#" onClick={onClick}>
            <span uk-icon="icon: file" className="uk-margin-small-right"></span>
            {name}
        </a>
    </li>
);

const Folder = ({ name, children }) => (
    <li className="uk-parent">
        <a href="#">
            <span uk-icon="icon: folder" className="uk-margin-small-right"></span>
            {name}
        </a>
        <SubMenu>
            {children}
        </SubMenu>
    </li>
);

class OnCanvas extends React.Component {
    componentDidMount() {
        console.log("hiding!");
        let element = UIkit.offcanvas("#"+this.props.sideNavId);
        console.dir(element);
        if (element !== undefined) element.hide();
    }
    
    render() {
        return (
            <div className="file-tree">
                <ul className="uk-nav-default uk-nav-parent-icon uk-nav" 
                    uk-nav="multiple: true" 
                    style={{width: this.props.width}}>
                    {this.props.children}
                </ul>
            </div>
        );
    }
}

class OffCanvas extends React.Component {

    componentDidMount() {
        UIkit.offcanvas("#"+this.props.sideNavId, {});
    }

    render() { 
        return (
            <div id={this.props.sideNavId} uk-offcanvas={""}>
                <div className="uk-offcanvas-bar">
                    <ul className="uk-nav-default uk-nav-parent-icon uk-nav"
                        uk-nav="multiple: true">
                        {this.props.children}
                    </ul>
                </div>
            </div>
        );
    }
}


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

        return (
            <div>
                <OffCanvas sideNavId={this.props.sideNavId}>{recurse_tree(this.state.tree)}</OffCanvas>
                <MediaQuery minWidth={991}>
                    <OnCanvas width={this.props.width} sideNavId={this.props.sideNavId}>{recurse_tree(this.state.tree)}</OnCanvas>
                </MediaQuery>
            </div>
        );

        return <DeviceTest />
    }
}