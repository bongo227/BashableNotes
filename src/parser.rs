use typed_arena::Arena;
use comrak::{format_html, parse_document, ComrakOptions};
use comrak::nodes::{AstNode, NodeValue, make_block};
use std::mem;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::cell::RefCell;

pub fn parse_markdown() -> String {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let mut f = File::open(Path::new("res/test.md")).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let mut options = ComrakOptions::default();
    options.ext_strikethrough = true;
    options.ext_table = true;
    options.ext_tasklist = true;
    options.ext_superscript = true;
    let root = parse_document(&arena, &contents, &options);

    fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
    where
        F: Fn(&'a AstNode<'a>),
    {
        f(node);
        for c in node.children() {
            iter_nodes(c, f);
        }
    }

    let mut html = vec![];

    {
        iter_nodes(root, &|node| match &mut node.data.borrow_mut().value {
            &mut NodeValue::CodeBlock(ref mut block) => {
                let info = String::from_utf8(block.info.clone()).unwrap();
                let code = String::from_utf8(block.literal.clone()).unwrap();

                if info.contains("hide") {
                    println!("info: {}, code: {}", info, code);

                    let sparkle_heart = vec![240, 159, 146, 150];
                    let node = NodeValue::Text(sparkle_heart);
                    let ast_node = AstNode::new(RefCell::new(make_block(node, 0, 0)));
                    let alloc_ast_node = arena.alloc(ast_node);

                    root.insert_after(alloc_ast_node);
                }
            },
            _ => (),
        });

        format_html(root, &ComrakOptions::default(), &mut html).unwrap();
    }


    let code_highlighting = r#"
        <link rel="stylesheet" href="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/styles/atom-one-dark.min.css">
        <script src="//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/highlight.min.js"></script>
        <script>hljs.initHighlightingOnLoad();</script>"#;
    let style = "<link rel=\"stylesheet\" type=\"text/css\" href=\"res/style.css\">";
    let html_string = String::from_utf8(html).unwrap();

    format!("{}\n\n{}\n\n{}", code_highlighting, style, html_string)
}
