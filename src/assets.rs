use std::{fs, path::Path, time::Duration};
use typst_syntax::{
    LinkedNode, Source, SyntaxNode,
    ast::{self, AstNode},
};

fn range_of(root: &LinkedNode<'_>, wanted: &SyntaxNode) -> Option<std::ops::Range<usize>> {
    root.find(wanted.span()).map(|node| node.range())
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
    fn leaves_mermaid_calls_to_typst() {
        let source = "#let mermaid(body) = body\n#mermaid(\"example\")\n`#mermaid(\"example\")`\n// #mermaid(\"example\")\n";
        let (output, dir) = preprocess(source, Path::new("note.typ"), Path::new(".")).unwrap();
        assert_eq!(source, output);
        assert!(dir.is_none());
    }
}
