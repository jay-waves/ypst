use rquickjs::{CatchResultExt, Context, Function, Runtime};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use typst_syntax::{
    LinkedNode, Source, SyntaxNode,
    ast::{self, AstNode},
};

fn range_of(root: &LinkedNode<'_>, wanted: &SyntaxNode) -> Option<std::ops::Range<usize>> {
    root.find(wanted.span()).map(|node| node.range())
}
const CSS_NUMBER_FORMAT: &str = include_str!("../vendor/intl.js");

// Keep each invocation isolated while sharing the resource-limit policy.
fn render_runtime() -> rquickjs::Result<Runtime> {
    let runtime = Runtime::new()?;
    runtime.set_memory_limit(256 * 1024 * 1024);
    runtime.set_max_stack_size(4 * 1024 * 1024);
    let deadline = Instant::now() + Duration::from_secs(10);
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    Ok(runtime)
}

/// Highlight literal block raw nodes in the main source before HTML compilation.
pub fn highlight_code(source: &str) -> Result<String, String> {
    let parsed = Source::detached(source);
    let linked = LinkedNode::new(parsed.root());
    let mut blocks = Vec::new();
    fn visit(node: &LinkedNode<'_>, blocks: &mut Vec<(std::ops::Range<usize>, String, String)>) {
        if let Some(raw) = node.get().cast::<ast::Raw>() {
            if raw.block()
                && node
                    .parent()
                    .is_some_and(|p| p.kind() == typst_syntax::SyntaxKind::Markup)
            {
                blocks.push((
                    node.range(),
                    raw.lines()
                        .map(|line| line.get().as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    raw.lang().map_or("", |lang| lang.get().as_str()).to_owned(),
                ));
            }
            return;
        }
        for child in node.children() {
            visit(&child, blocks);
        }
    }
    visit(&linked, &mut blocks);
    if blocks.is_empty() {
        return Ok(source.to_owned());
    }
    let runtime = render_runtime().map_err(|e| e.to_string())?;
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        let result = (|| -> rquickjs::Result<String> {
            ctx.eval::<(), _>(CSS_NUMBER_FORMAT)?;
            ctx.eval::<(), _>(include_str!("../vendor/highlight.js"))?;
            let render: Function = ctx.globals().get("highlightTypst")?;
            let mut output = source.to_owned();
            for (range, text, language) in blocks.into_iter().rev() {
                let mut replacement: String = render.call((text, language))?;
                let before = source[range.clone()]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count();
                replacement.push_str(&"\n".repeat(before));
                output.replace_range(range, &replacement);
            }
            Ok(output)
        })();
        result.catch(&ctx).map_err(|e| format!("highlight.js: {e}"))
    })
}

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else if bytes.starts_with(b"%PDF-") {
        Some("pdf")
    } else if std::str::from_utf8(bytes).ok().is_some_and(|s| {
        let s = s.trim_start_matches('\u{feff}').trim_start();
        s.starts_with("<svg") || (s.starts_with("<?xml") && s.contains("<svg"))
    }) {
        Some("svg")
    } else {
        None
    }
}

fn fetch_image(remote: &str) -> Result<Vec<u8>, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut response = agent
        .get(remote)
        .call()
        .map_err(|e| format!("image {remote}: {e}"))?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("image {remote}: {e}"))
}

pub fn preprocess(
    source: &str,
    source_path: &Path,
    root: &Path,
) -> Result<(String, Option<tempfile::TempDir>), String> {
    preprocess_with_fetch(source, source_path, root, &mut fetch_image)
}

fn preprocess_with_fetch<F>(
    source: &str,
    source_path: &Path,
    _root: &Path,
    fetch: &mut F,
) -> Result<(String, Option<tempfile::TempDir>), String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let parsed = Source::detached(source);
    let linked = LinkedNode::new(parsed.root());
    let mut calls = Vec::new();
    fn visit<'a>(
        node: &LinkedNode<'a>,
        calls: &mut Vec<(std::ops::Range<usize>, ast::FuncCall<'a>)>,
    ) {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && let ast::Expr::Ident(name) = call.callee()
            && name.as_str() == "imagit"
            && call
                .args()
                .items()
                .find_map(|arg| match arg {
                    ast::Arg::Pos(value) => Some(value),
                    _ => None,
                })
                .is_some_and(|value| {
                    matches!(value, ast::Expr::Str(text) if {
                        let lower = text.get().to_ascii_lowercase();
                        lower.starts_with("https://") || lower.starts_with("http://")
                    })
                })
        {
            calls.push((node.range(), call));
            return;
        }
        for child in node.children() {
            visit(&child, calls);
        }
    }
    visit(&linked, &mut calls);
    if calls.is_empty() {
        return Ok((source.to_owned(), None));
    }
    let dir = tempfile::Builder::new()
        .prefix(".ypst-images-")
        .tempdir_in(source_path.parent().ok_or("source has no parent")?)
        .map_err(|e| e.to_string())?;
    let dir_name = dir
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid temporary directory")?;
    let mut edits = Vec::new();
    let mut downloaded = std::collections::HashMap::<String, String>::new();
    for (index, (range, call)) in calls.into_iter().enumerate() {
        let start = if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
            range.start - 1
        } else {
            range.start
        };
        let prefix = &source[..start];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        let result = (|| -> Result<(), String> {
            let ast::Expr::Ident(name) = call.callee() else {
                unreachable!()
            };
            let Some(ast::Expr::Str(value)) = call.args().items().find_map(|arg| match arg {
                ast::Arg::Pos(value) => Some(value),
                _ => None,
            }) else {
                return Ok(());
            };
            let remote = value.get();
            let url =
                url::Url::parse(remote.as_str()).map_err(|e| format!("invalid image URL: {e}"))?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err("image URL must use HTTP or HTTPS".into());
            }
            let filename = if let Some(filename) = downloaded.get(url.as_str()) {
                filename.clone()
            } else {
                let bytes = fetch(url.as_str())?;
                let ext = image_extension(&bytes)
                    .ok_or_else(|| format!("image {remote}: unsupported image format"))?;
                let filename = format!("{index}.{ext}");
                fs::write(dir.path().join(&filename), bytes).map_err(|e| e.to_string())?;
                downloaded.insert(url.as_str().to_owned(), filename.clone());
                filename
            };
            let replacement = serde_json::to_string(&format!("{dir_name}/{filename}"))
                .map_err(|e| e.to_string())?;
            let span = range_of(&linked, value.to_untyped()).ok_or("cannot locate image URL")?;
            edits.push((span, replacement));
            let callee =
                range_of(&linked, name.to_untyped()).ok_or("cannot locate imagit function")?;
            edits.push((callee, "image".to_owned()));
            Ok(())
        })();
        result.map_err(|message| {
            format!(
                "{}:{line}:{column}: error: {message}",
                source_path.display()
            )
        })?;
    }
    let mut output = source.to_owned();
    edits.sort_by_key(|(range, _)| range.start);
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    Ok((output, Some(dir)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intl_number_format_handles_grouping_and_rounding() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            ctx.eval::<(), _>(CSS_NUMBER_FORMAT).unwrap();
            let result: String = ctx.eval("new Intl.NumberFormat('en-US', {useGrouping: true, maximumFractionDigits: 2}).format(1234.567)").unwrap();
            assert_eq!(result, "1,234.57");
        });
    }
    #[test]
    fn url_images_use_temporary_files_and_reuse_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let input = root.join("note.typ");
        let source = "#imagit(\"https://example.test/a?x=1\", width: 50%, alt: \"A\")\n#imagit(\"https://example.test/a?x=1\")\n`#imagit(\"https://ignored.test\")`";
        let mut calls = 0;
        let mut fetch = |url: &str| {
            calls += 1;
            assert_eq!(url, "https://example.test/a?x=1");
            Ok(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec())
        };
        let (output, temp) = preprocess_with_fetch(source, &input, &root, &mut fetch).unwrap();
        assert_eq!(calls, 1);
        assert!(output.contains("width: 50%, alt: \"A\""));
        assert!(output.contains("`#imagit(\"https://ignored.test\")`"));
        assert!(
            Source::detached(output.clone())
                .root()
                .errors_and_warnings()
                .0
                .is_empty()
        );
        assert_eq!(output.matches("0.svg").count(), 2);
        assert!(temp.unwrap().path().join("0.svg").is_file());
    }
    #[test]
    fn imagit_downloads_remote_images_and_preserves_other_calls() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let source = r#"#imagit("https://example.test/a", width: 50%, alt: "A")
#image("https://example.test/a")
#imagit("local.svg")
#imagit(path)
`#imagit("https://ignored.test")`
#let pic = imagit("https://example.test/a")"#;
        let mut calls = 0;
        let mut fetch = |_: &str| {
            calls += 1;
            Ok(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec())
        };
        let (output, temp) =
            preprocess_with_fetch(source, &root.join("note.typ"), &root, &mut fetch).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(output.matches("0.svg").count(), 2);
        assert!(output.starts_with("#image("));
        assert!(output.contains("width: 50%, alt: \"A\""));
        assert!(output.contains("#let pic = image("));
        assert!(output.contains("#imagit(\"local.svg\")"));
        assert!(output.contains("#imagit(path)"));
        assert!(output.contains("#image(\"https://example.test/a\")"));
        assert!(output.contains("`#imagit(\"https://ignored.test\")`"));
        assert!(temp.unwrap().path().join("0.svg").is_file());
    }
    #[test]
    fn native_url_images_are_not_preprocessed() {
        let dir = tempfile::tempdir().unwrap();
        let source = r#"#image("https://example.test/a")"#;
        let mut fetch = |_: &str| -> Result<Vec<u8>, String> {
            panic!("native image must not fetch remote URLs")
        };
        let (output, temp) =
            preprocess_with_fetch(source, &dir.path().join("note.typ"), dir.path(), &mut fetch)
                .unwrap();
        assert_eq!(output, source);
        assert!(temp.is_none());
    }
    #[test]
    fn unsupported_image_data_fails_before_compilation() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let mut fetch = |_: &str| Ok(b"not an image".to_vec());
        let result = preprocess_with_fetch(
            "#imagit(\"https://example.test/a\")",
            &root.join("n.typ"),
            &root,
            &mut fetch,
        );
        let error = result.unwrap_err();
        assert!(error.contains("unsupported image format"));
        assert!(error.starts_with(&format!("{}:1:1: error:", root.join("n.typ").display())));
    }
    #[test]
    fn highlights_static_blocks_and_preserves_plain_text() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let source =
                    "```js\nconst x = \"<tag>\";\n```\n\n```unknown\na < b & c\n```\n`inline`";
                let output = highlight_code(source).unwrap();
                assert!(output.contains("hljs-keyword"));
                assert!(output.contains(r#"#text("<")#text("tag")#text(">")"#));
                assert!(output.contains("a < b & c"));
                assert!(output.contains("`inline`"));
                assert_eq!(source.lines().count(), output.lines().count());
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn leaves_mermaid_calls_to_typst() {
        let source = "#let mermaid(body) = body\n#mermaid(\"example\")\n`#mermaid(\"example\")`\n// #mermaid(\"example\")\n";
        let (output, dir) = preprocess(source, Path::new("note.typ"), Path::new(".")).unwrap();
        assert_eq!(source, output);
        assert!(dir.is_none());
    }
}
