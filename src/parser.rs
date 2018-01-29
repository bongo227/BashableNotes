use typed_arena::Arena;
use comrak::{format_html, parse_document, ComrakOptions};
use comrak::nodes::{AstNode, NodeValue};
use std::mem;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn parse_markdown() -> String {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let mut f = File::open(Path::new("res/test.md")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let root = parse_document(&arena, &contents, &ComrakOptions::default());

    fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
    where
        F: Fn(&'a AstNode<'a>),
    {
        f(node);
        for c in node.children() {
            iter_nodes(c, f);
        }
    }

    iter_nodes(root, &|node| match &mut node.data.borrow_mut().value {
        &mut NodeValue::Text(ref mut text) => {
            let orig = mem::replace(text, vec![]);
            *text = String::from_utf8(orig)
                .unwrap()
                .replace("my", "your")
                .as_bytes()
                .to_vec();
        }
        _ => (),
    });

    let mut html = vec![];
    format_html(root, &ComrakOptions::default(), &mut html).unwrap();

    let html_string = String::from_utf8(html).unwrap();
    let style = "<link rel=\"stylesheet\" type=\"text/css\" href=\"res/markdown7.css\">";

    format!("{}\n\n{}", style, html_string)
}
