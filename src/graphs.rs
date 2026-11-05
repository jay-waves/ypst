use rquickjs::{CatchResultExt, Context, Function, Module, Object, Runtime};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use typst_syntax::{
    LinkedNode, Source, SyntaxNode,
    ast::{self, AstNode},
};

const CHARTIST: &str = include_str!("../vendor/renderer.js");
const CHARTIST_MODULE: &str = include_str!("../vendor/chartist-module.js");
const CHARTIST_SERVER: &str = include_str!("../vendor/chartist-server-module.js");
const ESSENTIALS: &str = include_str!("../vendor/chartist-essentials-module.js");
const ESSENTIALS_CSS: &str = include_str!("../vendor/chartist-essentials.css");
const CSS: &str = include_str!("../vendor/chartist.css");
const CHART_STYLE: &str = include_str!("../chartist.css");

fn range_of(root: &LinkedNode<'_>, wanted: &SyntaxNode) -> Option<std::ops::Range<usize>> {
    root.find(wanted.span()).map(|node| node.range())
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

fn render_chart(source: &str) -> Result<String, String> {
    let runtime = render_runtime().map_err(|e| e.to_string())?;
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        let result = (|| -> rquickjs::Result<String> {
            ctx.eval::<(), _>(CSS_NUMBER_FORMAT)?;
            ctx.eval::<(), _>(CHARTIST)?;
            ctx.globals().set("__chartistCss", CSS)?;
            ctx.globals().set("__chartistTheme", CHART_STYLE)?;
            ctx.globals().set("__essentialsCss", ESSENTIALS_CSS)?;
            ctx.eval::<(), _>("globalThis.__vizRenderSvg = scene => __vizChartistServer.renderToString(__vizChartistServer.applyStyles(scene, [__chartistCss, __essentialsCss, __chartistTheme]));")?;
            Module::declare(ctx.clone(), "chartist", CHARTIST_MODULE)?.eval()?.1.finish::<()>()?;
            Module::declare(ctx.clone(), "@chartist-js/essentials", ESSENTIALS)?.eval()?.1.finish::<()>()?;
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
        result.catch(&ctx).map_err(|e| e.to_string())
    })
}
fn render_mermaid(
    source: &str,
    index: usize,
    html: bool,
) -> Result<String, (String, Option<(usize, usize)>)> {
    let runtime = render_runtime().map_err(|e| (e.to_string(), None))?;
    let context = Context::full(&runtime).map_err(|e| (e.to_string(), None))?;
    context.with(|ctx| {
        let result = (|| -> rquickjs::Result<String> {
            ctx.eval::<(), _>(super::PELE)?;
            // Match the body font stack in lib.typ and provide a standalone SVG fallback.
            let font_family = "Georgia, Libertinus Serif, New Computer Modern, Source Han Serif, serif";
            let func: Function = ctx.eval("(source,id,fontFamily) => { try { return JSON.stringify({svg: pele.render(source,{idPrefix:id,fontFamily,autoDirection:false,maxWidth:900,images:false}).svg}); } catch (e) { return JSON.stringify({error: e.message || String(e), code: e.code, line: e.line, column: e.column}); } }")?;
            func.call((source, format!("graph-{index}"), font_family))
        })();
        let json = result.catch(&ctx).map_err(|e| (format!("mermaid: {e}"), None))?;
        let data: serde_json::Value = serde_json::from_str(&json).map_err(|e| (format!("mermaid: {e}"), None))?;
        if let Some(error) = data["error"].as_str() {
            let location = data["line"].as_u64().zip(data["column"].as_u64())
                .and_then(|(line, column)| Some((usize::try_from(line).ok()?, usize::try_from(column).ok()?)))
                .filter(|(line, column)| *line > 0 && *column > 0);
            let code = data["code"].as_str().filter(|code| matches!(*code, "syntax" | "semantic" | "unsupported-diagram" | "limit"));
            let prefix = code.map_or_else(|| "Pele: ".to_owned(), |code| format!("Pele {code}: "));
            let formatted = error.lines().enumerate().map(|(i, line)| if i == 0 { line.to_owned() } else { format!("  {line}") }).collect::<Vec<_>>().join("\n");
            return Err((format!("{prefix}{formatted}"), location));
        }
        let mut svg = data["svg"].as_str().ok_or_else(|| ("mermaid returned no SVG".into(), None))?.to_owned();
        if html { return Ok(svg); }
        Ok({
            for (name, value) in [("--_bg", "#ffffff"), ("--_fg", "#1f1f1f"), ("--_m", "#6e6e6e"), ("--_l", "#1f1f1f"), ("--_s", "#f3f3f3"), ("--_a", "#e6e6e6"), ("--_b", "#8a8a8a"), ("--_c", "#5b7bd5"), ("--_r", "4px"), ("--_fm", "monospace")] {
                svg = svg.replace(&format!("var({name})"), value);
            }
            let defaults = regex::Regex::new(r"var\(--pele-[a-z-]+,\s*([^()]*)\)").unwrap();
            defaults.replace_all(&svg, "$1").into_owned()
        })
    })
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

pub fn preprocess_html(
    source: &str,
    source_path: &Path,
    root: &Path,
) -> Result<(String, Option<tempfile::TempDir>), String> {
    preprocess_with_fetch_mode(source, source_path, root, &mut fetch_image, true)
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
    preprocess_with_fetch_mode(source, source_path, root, fetch, false)
}

fn preprocess_with_fetch_mode<F>(
    source: &str,
    source_path: &Path,
    root: &Path,
    fetch: &mut F,
    html: bool,
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
        let start = if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
            range.start - 1
        } else {
            range.start
        };
        let prefix = &source[..start];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        let mut error_location = None;
        let result = (|| -> Result<(), String> {
            let ast::Expr::Ident(name) = call.callee() else {
                unreachable!()
            };
            if name.as_str() == "image" {
                let Some(ast::Expr::Str(value)) = call.args().items().find_map(|arg| match arg {
                    ast::Arg::Pos(value) => Some(value),
                    _ => None,
                }) else {
                    return Ok(());
                };
                let remote = value.get();
                let url = url::Url::parse(remote.as_str())
                    .map_err(|e| format!("invalid image URL: {e}"))?;
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
                let span =
                    range_of(&linked, value.to_untyped()).ok_or("cannot locate image URL")?;
                edits.push((span, replacement));
                return Ok(());
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
                let script = fs::read_to_string(&file)
                    .map_err(|e| format!("cannot read chartist {relative:?}: {e}"))?;
                let markup =
                    render_chart(&script).map_err(|e| format!("chartist {relative:?}: {e}"))?;
                let start = markup.find("<svg").ok_or("chartist must render an SVG")?;
                let end = markup
                    .rfind("</svg>")
                    .ok_or("chartist returned invalid SVG")?
                    + 6;
                markup[start..end].to_owned()
            } else {
                let graph = match positional[0] {
                    ast::Expr::Str(value) => value.get().to_string(),
                    ast::Expr::ContentBlock(block) => raw_content(block.to_untyped())
                        .ok_or("#mermaid expects a raw code block")?,
                    ast::Expr::Raw(raw) => raw
                        .lines()
                        .map(|line| line.get().as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => return Err("#mermaid expects a raw code block or string".into()),
                };
                render_mermaid(&graph, index, html)
                    .map(|svg| {
                        if html {
                            svg.replace(
                                &format!("graph-{index}"),
                                &format!("{dir_name}-graph-{index}"),
                            )
                        } else {
                            svg
                        }
                    })
                    .map_err(|(message, location)| {
                        if let Some((line, column)) = location {
                            let mapped = source.get(range.clone()).and_then(|block| {
                                let found = block.find(&graph)?;
                                if block[found + graph.len()..].contains(&graph) {
                                    return None;
                                }
                                let part = graph.split('\n').nth(line.checked_sub(1)?)?;
                                let line_start = part.as_ptr() as usize - graph.as_ptr() as usize;
                                let mut units = 0;
                                let mut bytes = 0;
                                for ch in part.chars() {
                                    if units >= column - 1 {
                                        break;
                                    }
                                    units += ch.len_utf16();
                                    bytes += ch.len_utf8();
                                }
                                (units == column - 1)
                                    .then_some(range.start + found + line_start + bytes)
                            });
                            if let Some(offset) = mapped {
                                error_location = Some(offset);
                            } else {
                                return match message.split_once('\n') {
                                    Some((first, details)) => format!(
                                        "{first} (fragment line {line}, column {column})\n{details}"
                                    ),
                                    None => {
                                        format!("{message} (fragment line {line}, column {column})")
                                    }
                                };
                            }
                        }
                        message
                    })?
            };
            let filename = format!("{index}.svg");
            fs::write(dir.path().join(&filename), svg).map_err(|e| e.to_string())?;
            let file = serde_json::to_string(&format!("{dir_name}/{filename}"))
                .map_err(|e| e.to_string())?;
            let image = if html && name.as_str() == "mermaid" {
                format!(
                    "html.elem(\"div\", attrs: (\"data-ypst-pele\": {file}, \"style\": \"width:{}\"))",
                    width.unwrap_or("100%")
                )
            } else {
                let default_width = if name.as_str() == "mermaid" {
                    "auto"
                } else {
                    "100%"
                };
                format!("image({file}, width: {})", width.unwrap_or(default_width))
            };
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
            Ok(())
        })();
        result.map_err(|message| {
            let (line, column) = error_location
                .map(|at| {
                    let prefix = &source[..at];
                    (
                        prefix.bytes().filter(|b| *b == b'\n').count() + 1,
                        prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
                    )
                })
                .unwrap_or((line, column));
            format!(
                "{}:{line}:{column}: error: {message}",
                source_path.display()
            )
        })?;
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
    fn mermaid_error_points_into_raw_block() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let source =
            "Before\n\n#mermaid(```mermaid\nsequenceDiagram\nAlice ->> Bob: hi\nfoo\n```)\n";
        let error = preprocess(source, &root.join("note.typ"), &root).unwrap_err();
        assert!(error.contains(":6:4: error: Pele syntax: "), "{error}");
        assert!(
            error.lines().skip(1).all(|line| line.starts_with("  ")),
            "{error}"
        );
    }

    #[test]
    fn mermaid_accepts_literal_string_without_code_fence() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let dir = tempfile::tempdir().unwrap();
                let root = fs::canonicalize(dir.path()).unwrap();
                let (output, files) = preprocess(
                    "#mermaid(\"graph LR; A --> B\")",
                    &root.join("note.typ"),
                    &root,
                )
                .unwrap();
                assert!(output.contains("image("));
                assert!(
                    fs::read_to_string(files.unwrap().path().join("0.svg"))
                        .unwrap()
                        .contains("<svg")
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn named_muted_colors_apply_to_series_and_labels() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let source = r#"
import { barChartConfig, lineChartConfig } from 'chartist';
import { toScene, renderToString } from 'chartist/server';
import { trendLine, targetLine, valueLabels } from '@chartist-js/essentials';
export default c => {
  const data = {labels:['A','B'], series:[
    {className:'tol-muted-rose', data:[4,5]},
    {className:'tol-muted-indigo', data:[2,3]}
  ]};
  const bars = barChartConfig(data, {layers:{above:[valueLabels({barPosition:'inside-end'})]}});
  const lines = lineChartConfig(data, {layers:{above:[trendLine(),targetLine({value:2,label:'Target',className:'tol-muted-olive'})]}});
  c.innerHTML = [bars, lines].map(config => renderToString(toScene(config, {width:600,height:300}))).join('');
};
"#;
                let svg = render_chart(source).unwrap();
                let element = |class: &str, occurrence: usize| {
                    svg.split('<')
                        .filter(|part| part.contains(&format!("class=\"{class}\"")))
                        .nth(occurrence)
                        .unwrap()
                        .split('>')
                        .next()
                        .unwrap()
                };
                assert!(element("ct-bar", 0).contains("stroke=\"#cc6677\""));
                assert!(element("ct-bar", 2).contains("stroke=\"#332288\""));
                assert!(element("ct-trend-line", 0).contains("stroke=\"#cc6677\""));
                assert!(element("ct-trend-line", 1).contains("stroke=\"#332288\""));
                assert!(element("ct-target-line tol-muted-olive", 0).contains("stroke=\"#999933\""));
                assert!(element("ct-value-label tol-muted-rose ct-value-label-inside", 0).contains("fill=\"#202020\""));
                assert!(element("ct-value-label tol-muted-indigo ct-value-label-inside", 0).contains("fill=\"#ffffff\""));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn essentials_layers_use_the_chart_theme_in_static_svg() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let source = r#"
import { lineChartConfig } from 'chartist';
import { toScene, renderToString } from 'chartist/server';
import { trendLine, targetLine, valueLabels } from '@chartist-js/essentials';
export default c => {
  const config = lineChartConfig({labels:['A','B','C'],series:[[1,3,2],[2,4,3]]},
    {layers:{above:[trendLine(), targetLine({value:2,label:'Target'}), valueLabels()]}});
  c.innerHTML = renderToString(toScene(config, {width:600,height:300}));
};
"#;
                let svg = render_chart(source).unwrap();
                let element = |class: &str, occurrence: usize| {
                    svg.split('<')
                        .filter(|part| part.contains(&format!("class=\"{class}\"")))
                        .nth(occurrence)
                        .unwrap()
                        .split('>')
                        .next()
                        .unwrap()
                };
                assert!(element("ct-trend-line", 0).contains("stroke=\"#332288\""));
                assert!(element("ct-trend-line", 1).contains("stroke=\"#44aa99\""));
                assert!(element("ct-target-line", 0).contains("stroke=\"#666666\""));
                assert!(element("ct-target-label", 0).contains("fill=\"#666666\""));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn preprocesses_static_graph_calls_without_touching_raw_text() {
        std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(|| {
            let dir = tempfile::tempdir().unwrap();
            let root = fs::canonicalize(dir.path()).unwrap();
            let input = root.join("note.typ");
            fs::write(root.join("chart.js"), include_str!("../examples/chart.js")).unwrap();
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
            assert!(chart.contains("#cc6677"));
            assert!(chart.contains("ct-axis-title"));
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
    fn leaves_plain_source_untouched() {
        let source = "`#mermaid(\"example\")`\n// #chartist(\"example.js\")\n";
        let (output, dir) = preprocess(source, Path::new("note.typ"), Path::new(".")).unwrap();
        assert_eq!(source, output);
        assert!(dir.is_none());
    }
}
