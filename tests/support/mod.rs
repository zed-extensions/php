//! Test helpers that exercise the language `.scm` runnable queries the same way
//! Zed does: the native `tree-sitter` engine, the pinned PHP grammar, and Zed's
//! query-cursor match limit of 64 (`QueryCursorHandle::new` in
//! `crates/language/src/syntax_map.rs`).

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

/// A single `@run` capture produced by a runnables query — i.e. one gutter icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The tag set on the matched pattern via `(#set! tag <tag>)`.
    pub tag: Option<String>,
    /// Source text of the captured node (what `$ZED_SYMBOL` resolves near).
    pub text: String,
    /// 1-indexed row the gutter icon lands on.
    pub row: usize,
}

/// Run a runnables `.scm` file over PHP `source` and collect every `@run`
/// capture, mirroring how Zed extracts runnables.
pub fn run_query(scm_path: &str, source: &str) -> Vec<Run> {
    let language: tree_sitter::Language = tree_sitter_php::LANGUAGE_PHP.into();

    let mut parser = Parser::new();
    parser.set_language(&language).expect("load PHP grammar");
    let tree = parser.parse(source, None).expect("parse source");

    let scm = std::fs::read_to_string(scm_path).unwrap_or_else(|e| panic!("read {scm_path}: {e}"));
    let query = Query::new(&language, &scm).expect("compile runnables query");
    let run_idx = query
        .capture_index_for_name("run")
        .expect("query defines a @run capture");

    let mut cursor = QueryCursor::new();
    // Mirror Zed: QueryCursorHandle::new() caps in-progress matches at 64. This
    // is what makes `program`-rooted correlation patterns drop runnables on big
    // files, so tests must honour the same limit to catch that regression class.
    cursor.set_match_limit(64);

    let mut runs = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        let tag = query
            .property_settings(m.pattern_index)
            .iter()
            .find(|p| &*p.key == "tag")
            .and_then(|p| p.value.as_deref().map(str::to_owned));
        for c in m.captures.iter().filter(|c| c.index == run_idx) {
            runs.push(Run {
                tag: tag.clone(),
                text: c
                    .node
                    .utf8_text(source.as_bytes())
                    .expect("utf8 text")
                    .to_string(),
                row: c.node.start_position().row + 1,
            });
        }
    }
    runs
}
