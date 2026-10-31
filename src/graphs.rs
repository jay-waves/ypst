use rquickjs::{CatchResultExt, Context, Function, Module, Object, Runtime};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use typst_syntax::{
    LinkedNode, SyntaxNode,
    ast::{self, AstNode},
};

const CHARTIST: &str = include_str!("../vendor/renderer.js");
const CHARTIST_MODULE: &str = include_str!("../vendor/chartist-module.js");
const CHARTIST_SERVER: &str = include_str!("../vendor/chartist-server-module.js");
const CSS: &str = include_str!("../vendor/chartist.css");
const CHART_STYLE: &str = include_str!("../chartist.css");

fn range_of(root: &LinkedNode<'_>, wanted: &SyntaxNode) -> Option<std::ops::Range<usize>> {
    if std::ptr::eq(root.get(), wanted) {
        return Some(root.range());
    }
    root.children().find_map(|child| range_of(&child, wanted))
}
fn raw_content(node: &SyntaxNode) -> Option<String> {
    if let Some(raw) = node.cast::<ast::Raw>() {
        return Some(
            raw.lines()
                .map(|line| line.get().as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    node.children().find_map(raw_content)
}
fn render_chart(source: &str) -> Result<String, String> {
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(256 * 1024 * 1024);
    runtime.set_max_stack_size(4 * 1024 * 1024);
    let deadline = Instant::now() + Duration::from_secs(10);
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        let result = (|| -> rquickjs::Result<String> {
            ctx.eval::<(), _>(CHARTIST)?;
            ctx.globals().set("__chartistCss", CSS)?;
            ctx.globals().set("__chartistTheme", CHART_STYLE)?;
            ctx.eval::<(), _>("globalThis.__vizRenderSvg = scene => __vizChartistServer.renderToString(__vizChartistServer.applyStyles(scene, [__chartistCss, __chartistTheme]));")?;
            Module::declare(ctx.clone(), "chartist", CHARTIST_MODULE)?.eval()?.1.finish::<()>()?;
            let server = CHARTIST_SERVER.replace("export const renderToString=globalThis.__vizChartistServer.renderToString;", "export const renderToString=globalThis.__vizRenderSvg;");
            Module::declare(ctx.clone(), "chartist/server", server)?.eval()?.1.finish::<()>()?;
            let prepare: Function = ctx.globals().get("prepareViz")?;
            let container: Object = prepare.call(())?;
            let (module, promise) = Module::declare(ctx.clone(), "chart", source)?.eval()?;
            promise.finish::<()>()?;
            let function: Function = module.get("default")?;
            let run: Function = ctx.globals().get("runViz")?;
            run.call((function, container))
        })();
        result.catch(&ctx).map_err(|e| format!("chartist: {e}"))
    })
}
fn render_mermaid(source: &str, index: usize) -> Result<String, String> {
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(256 * 1024 * 1024);
    runtime.set_max_stack_size(4 * 1024 * 1024);
    let deadline = Instant::now() + Duration::from_secs(10);
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        let result = (|| -> rquickjs::Result<String> {
            ctx.eval::<(), _>(super::PELE)?;
            let func: Function = ctx.eval("(source,id) => pele.render(source,{idPrefix:id,autoDirection:false,maxWidth:900,images:false}).svg")?;
            func.call((source, format!("graph-{index}")))
        })();
        result.catch(&ctx).map(|mut svg| {
            for (name, value) in [("--_bg", "#ffffff"), ("--_fg", "#1f1f1f"), ("--_m", "#6e6e6e"), ("--_l", "#1f1f1f"), ("--_s", "#f3f3f3"), ("--_a", "#e6e6e6"), ("--_b", "#8a8a8a"), ("--_c", "#5b7bd5"), ("--_r", "4px"), ("--_fm", "monospace")] {
                svg = svg.replace(&format!("var({name})"), value);
            }
            let defaults = regex::Regex::new(r"var\(--pele-[a-z-]+,\s*([^()]*)\)").unwrap();
            defaults.replace_all(&svg, "$1").into_owned()
        }).map_err(|e| format!("mermaid: {e}"))
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
    root: &Path,
    fetch: &mut F,
) -> Result<(String, Option<tempfile::TempDir>), String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let tree = typst_syntax::parse(source);
    let linked = LinkedNode::new(&tree);
    let mut calls = Vec::new();
    fn visit<'a>(
        node: &LinkedNode<'a>,
        calls: &mut Vec<(std::ops::Range<usize>, ast::FuncCall<'a>)>,
    ) {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && let ast::Expr::Ident(name) = call.callee()
            && (matches!(name.as_str(), "chartist" | "mermaid") || (name.as_str() == "image" && call.args().items().find_map(|arg| match arg { ast::Arg::Pos(value) => Some(value), _ => None }).is_some_and(|value| matches!(value, ast::Expr::Str(text) if { let lower = text.get().to_ascii_lowercase(); lower.starts_with("https://") || lower.starts_with("http://") }))))
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
        .prefix(".typst-graphs-")
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
        let ast::Expr::Ident(name) = call.callee() else {
            unreachable!()
        };
        if name.as_str() == "image" {
            let Some(ast::Expr::Str(value)) = call.args().items().find_map(|arg| match arg {
                ast::Arg::Pos(value) => Some(value),
                _ => None,
            }) else {
                continue;
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
            continue;
        }
        let mut positional = Vec::new();
        let mut width = None;
        let mut caption = None;
        for arg in call.args().items() {
            match arg {
                ast::Arg::Pos(value) => positional.push(value),
                ast::Arg::Named(named) => {
                    let r = range_of(&linked, named.expr().to_untyped())
                        .ok_or("cannot locate graph argument")?;
                    let value = &source[r];
                    match named.name().as_str() {
                        "width" if width.is_none() => width = Some(value),
                        "caption" if caption.is_none() => caption = Some(value),
                        other => {
                            return Err(format!(
                                "unsupported or duplicate graph argument: {other}"
                            ));
                        }
                    }
                }
                _ => return Err("graph arguments cannot use spreads".into()),
            }
        }
        if positional.len() != 1 {
            return Err(format!("#{} expects one input", name.as_str()));
        }
        let svg = if name.as_str() == "chartist" {
            let ast::Expr::Str(path) = positional[0] else {
                return Err("#chartist expects a literal JS file path".into());
            };
            let relative = path.get();
            let file = fs::canonicalize(source_path.parent().unwrap().join(relative.as_str()))
                .map_err(|e| format!("chartist {relative}: {e}"))?;
            if !file.starts_with(root) {
                return Err("chartist file is outside --root".into());
            }
            let script = fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let markup = render_chart(&script)?;
            let start = markup.find("<svg").ok_or("chartist must render an SVG")?;
            let end = markup
                .rfind("</svg>")
                .ok_or("chartist returned invalid SVG")?
                + 6;
            markup[start..end].to_owned()
        } else {
            let graph = match positional[0] {
                ast::Expr::Str(value) => value.get().to_string(),
                ast::Expr::ContentBlock(block) => {
                    raw_content(block.to_untyped()).ok_or("#mermaid expects a raw code block")?
                }
                ast::Expr::Raw(raw) => raw
                    .lines()
                    .map(|line| line.get().as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => return Err("#mermaid expects a raw code block or string".into()),
            };
            render_mermaid(&graph, index)?
        };
        let filename = format!("{index}.svg");
        fs::write(dir.path().join(&filename), svg).map_err(|e| e.to_string())?;
        let file =
            serde_json::to_string(&format!("{dir_name}/{filename}")).map_err(|e| e.to_string())?;
        let image = format!("image({file}, width: {})", width.unwrap_or("100%"));
        let mut replacement = caption.map_or(image.clone(), |value| {
            format!("figure({image}, caption: {value})")
        });
        let before = source[range.clone()]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        let after = replacement.bytes().filter(|b| *b == b'\n').count();
        replacement.push_str(&"\n".repeat(before.saturating_sub(after)));
        edits.push((range, replacement));
    }
    let mut output = source.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    Ok((output, Some(dir)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preprocesses_static_graph_calls_without_touching_raw_text() {
        std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(|| {
            let dir = tempfile::tempdir().unwrap();
            let root = fs::canonicalize(dir.path()).unwrap();
            let input = root.join("note.typ");
            fs::write(root.join("chart.js"), "import { barChartConfig } from 'chartist'; import { toScene, renderToString } from 'chartist/server'; export default c => { c.innerHTML = renderToString(toScene(barChartConfig({labels:['A'],series:[[1]]}),{width:400,height:200})); };").unwrap();
            let source = "// #chartist(\"missing.js\")\n#chartist(\"chart.js\", caption: [Chart], width: 70%)\n#mermaid(caption: [Flow])[\n```mermaid\ngraph LR\n A --> B\n```\n]\n`#chartist(\"no.js\")`\n";
            let (out, graphs) = preprocess(source, &input, &root).unwrap();
            assert!(out.contains("figure(image("));
            assert!(out.contains("width: 70%"));
            assert!(out.contains("caption: [Flow]"));
            assert!(out.contains("`#chartist(\"no.js\")`"));
            assert_eq!(source.lines().count(), out.lines().count());
            let graph = graphs.unwrap();
            let chart = fs::read_to_string(graph.path().join("0.svg")).unwrap();
            let mermaid = fs::read_to_string(graph.path().join("1.svg")).unwrap();
            assert!(chart.contains("ct-bar"));
            assert!(chart.contains("#618ba2"));
            assert!(chart.contains("Georgia"));
            assert!(mermaid.contains("<svg"));
            assert!(!mermaid.contains("var(--"));
        }).unwrap().join().unwrap();
    }
    #[test]
    fn url_images_use_temporary_files_and_reuse_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let input = root.join("note.typ");
        let source = "#image(\"https://example.test/a?x=1\", width: 50%, alt: \"A\")\n#image(\"https://example.test/a?x=1\")\n`#image(\"https://ignored.test\")`";
        let mut calls = 0;
        let mut fetch = |url: &str| {
            calls += 1;
            assert_eq!(url, "https://example.test/a?x=1");
            Ok(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec())
        };
        let (output, temp) = preprocess_with_fetch(source, &input, &root, &mut fetch).unwrap();
        assert_eq!(calls, 1);
        assert!(output.contains("width: 50%, alt: \"A\""));
        assert!(output.contains("`#image(\"https://ignored.test\")`"));
        assert_eq!(output.matches("0.svg").count(), 2);
        assert!(temp.unwrap().path().join("0.svg").is_file());
    }
    #[test]
    fn unsupported_image_data_fails_before_compilation() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let mut fetch = |_: &str| Ok(b"not an image".to_vec());
        let result = preprocess_with_fetch(
            "#image(\"https://example.test/a\")",
            &root.join("n.typ"),
            &root,
            &mut fetch,
        );
        assert!(result.unwrap_err().contains("unsupported image format"));
    }
    #[test]
    fn leaves_plain_source_untouched() {
        let source = "`#mermaid(\"example\")`\n// #chartist(\"example.js\")\n";
        let (output, dir) = preprocess(source, Path::new("note.typ"), Path::new(".")).unwrap();
        assert_eq!(source, output);
        assert!(dir.is_none());
    }
}
