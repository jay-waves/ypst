mod graphs;
mod site_preview;
use regex::Regex;
use rquickjs::{CatchResultExt, Context, Function, Runtime};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

const THEME: &str = include_str!("../html/theme.css");
const FOOTNOTES: &str = include_str!("../html/footnotes.css");
const HEADINGS: &str = include_str!("../html/headings.css");
const MERMAID_CSS: &str = include_str!("../html/mermaid.css");
const PELE: &str = include_str!("../html/pele.global.js");

const PACKAGE_FILES: &[(&str, &str)] = &[
    ("typst.toml", include_str!("../typst.toml")),
    ("lib.typ", include_str!("../lib.typ")),
    ("html/html.typ", include_str!("../html/html.typ")),
    ("html/theme.css", THEME),
    ("html/footnotes.css", FOOTNOTES),
    ("html/headings.css", HEADINGS),
    ("html/mermaid.css", MERMAID_CSS),
    ("html/links.js", include_str!("../html/links.js")),
    ("html/footnotes.js", include_str!("../html/footnotes.js")),
    ("html/headings.js", include_str!("../html/headings.js")),
    (
        "html/line_navigation.js",
        include_str!("../html/line_navigation.js"),
    ),
    ("html/pele.global.js", PELE),
    ("html/mermaid.js", include_str!("../html/mermaid.js")),
    (
        "html/LICENSE-Pele.txt",
        include_str!("../html/LICENSE-Pele.txt"),
    ),
    (
        "html/COMPATIBILITY.md",
        include_str!("../html/COMPATIBILITY.md"),
    ),
];

fn package_directory(
    platform: &str,
    appdata: Option<&str>,
    xdg: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, String> {
    let home = home.filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = match platform {
        "windows" => appdata
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|p| p.join("AppData/Roaming"))),
        "macos" => home.as_ref().map(|p| p.join("Library/Application Support")),
        _ => xdg
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.as_ref().map(|p| p.join(".local/share"))),
    }
    .ok_or("cannot locate the user package directory")?;
    Ok(base.join("typst/packages/local/ypst-template/0.1.0"))
}

fn install_package(target: &Path) -> Result<(), String> {
    for (name, contents) in PACKAGE_FILES {
        let path = target.join(name);
        fs::create_dir_all(path.parent().unwrap())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if fs::read(&path).ok().as_deref() != Some(contents.as_bytes()) {
            let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())
                .map_err(|e| e.to_string())?;
            temp.write_all(contents.as_bytes())
                .map_err(|e| e.to_string())?;
            temp.persist(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn short_diagnostics(args: &[String]) -> bool {
    let mut format = "short";
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--diagnostic-format" {
            if let Some(value) = args.get(i + 1) {
                format = value;
                i += 1;
            }
        } else if let Some(value) = args[i].strip_prefix("--diagnostic-format=") {
            format = value;
        }
        i += 1;
    }
    format == "short"
}

fn remap_diagnostic(
    line: &str,
    entry: &Path,
    source_name: &str,
    source_lines: usize,
    offset: usize,
) -> String {
    let pattern = Regex::new(r"^(.*):(\d+):(\d+): (error|warning|help):").unwrap();
    let Some(capture) = pattern.captures(line) else {
        return line.into();
    };
    let name = capture[1].strip_prefix(r"\\?\").unwrap_or(&capture[1]);
    let same = fs::canonicalize(name)
        .ok()
        .zip(fs::canonicalize(entry).ok())
        .is_some_and(|(a, b)| a == b);
    let Some(row) = capture[2]
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(offset))
    else {
        return line.into();
    };
    if !same || row == 0 || row > source_lines {
        return line.into();
    }
    format!(
        "{}:{}:{}: {}:{}",
        source_name,
        row,
        &capture[3],
        &capture[4],
        &line[capture.get(0).unwrap().end()..]
    )
}

fn run_compiler(
    compiler: &mut Command,
    entry: Option<&Path>,
    original: &str,
    lines: usize,
    offset: usize,
    options: &[String],
) -> Result<std::process::ExitStatus, String> {
    if let Some(entry) = entry.filter(|_| short_diagnostics(options)) {
        let mut child = compiler
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or("cannot capture Typst diagnostics")?;
        for line in BufReader::new(stderr).lines() {
            let line = line.map_err(|e| e.to_string())?;
            eprintln!(
                "{}",
                remap_diagnostic(&line, entry, original, lines, offset)
            );
        }
        child.wait().map_err(|e| e.to_string())
    } else {
        compiler.status().map_err(|e| e.to_string())
    }
}

fn shared_assets(root: &Path) -> Result<PathBuf, String> {
    let css = [THEME, FOOTNOTES, HEADINGS, MERMAID_CSS].join("\n");
    let js = format!(
        "(() => {{\n{}\n{}\n{}\n{}\n}})();\n",
        include_str!("../html/links.js"),
        include_str!("../html/footnotes.js"),
        include_str!("../html/headings.js"),
        include_str!("../html/line_navigation.js")
    );
    let mut hash = Sha256::new();
    for (name, data) in [("theme.css", css.as_bytes()), ("theme.js", js.as_bytes())] {
        hash.update(name.as_bytes());
        hash.update(data);
    }
    let dir = root
        .join("_shared/typst")
        .join(&format!("{:x}", hash.finalize())[..16]);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, data) in [("theme.css", css.as_bytes()), ("theme.js", js.as_bytes())] {
        let path = dir.join(name);
        if fs::read(&path).ok().as_deref() != Some(data) {
            let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
            file.write_all(data).map_err(|e| e.to_string())?;
            file.persist(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(dir)
}

fn relative_path(from: &Path, to: &Path) -> Result<String, String> {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return Err(
            "site assets and output must be on the same filesystem root; use --assets-url".into(),
        );
    }
    let mut result = PathBuf::new();
    for _ in common..from.len() {
        result.push("..");
    }
    for part in &to[common..] {
        result.push(part.as_os_str());
    }
    Ok(result.to_string_lossy().replace('\\', "/"))
}

fn asset_url(value: &str) -> Result<String, String> {
    if value.is_empty() || value.contains('?') || value.contains('#') {
        return Err(
            "--assets-url must be a URL path or HTTP(S) URL without a query or fragment".into(),
        );
    }
    if value.contains("://") {
        let parsed = url::Url::parse(value).map_err(|e| format!("invalid --assets-url: {e}"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("--assets-url must use HTTP(S)".into());
        }
    } else if value
        .split('/')
        .next()
        .is_some_and(|part| part.contains(':'))
    {
        return Err("--assets-url must be a URL path or HTTP(S) URL".into());
    }
    Ok(format!("{}/", value.trim_end_matches('/')))
}

fn escape_relative_url(path: &str) -> String {
    path.split('/')
        .map(|part| {
            url::form_urlencoded::byte_serialize(part.as_bytes())
                .collect::<String>()
                .replace('+', "%20")
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn externalize_site_images(html: &str, site: &Path, page: &Path) -> Result<String, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let pattern = Regex::new(r#"(?is)(<img\b[^>]*?\bsrc\s*=\s*")data:(image/(?:png|jpeg|gif|webp|svg\+xml));base64,([A-Za-z0-9+/=]+)(")"#).unwrap();
    let excluded =
        Regex::new(r"(?is)<!--.*?-->|<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>")
            .unwrap();
    let ignored: Vec<_> = excluded.find_iter(html).map(|item| item.range()).collect();
    let matches: Vec<_> = pattern
        .captures_iter(html)
        .filter(|cap| {
            let start = cap.get(0).unwrap().start();
            !ignored.iter().any(|range| range.contains(&start))
        })
        .collect();
    if matches.is_empty() {
        return Ok(html.to_owned());
    }
    let shared = site.join("_shared/typst/images");
    fs::create_dir_all(&shared).map_err(|e| e.to_string())?;
    let shared = fs::canonicalize(shared).map_err(|e| e.to_string())?;
    let page_dir =
        fs::canonicalize(page.parent().ok_or("invalid page path")?).map_err(|e| e.to_string())?;
    let mut result = String::new();
    let mut cursor = 0;
    for cap in matches {
        let whole = cap.get(0).unwrap();
        result.push_str(&html[cursor..whole.start()]);
        let bytes = STANDARD
            .decode(&cap[3])
            .map_err(|e| format!("invalid embedded image: {e}"))?;
        let extension = match &cap[2].to_ascii_lowercase()[..] {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            _ => unreachable!(),
        };
        let name = format!("{:x}.{extension}", Sha256::digest(&bytes));
        let destination = shared.join(&name);
        if fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
            let mut temp = tempfile::NamedTempFile::new_in(&shared).map_err(|e| e.to_string())?;
            temp.write_all(&bytes).map_err(|e| e.to_string())?;
            if let Err(error) = temp.persist(&destination)
                && fs::read(&destination).ok().as_deref() != Some(bytes.as_slice())
            {
                return Err(error.to_string());
            }
        }
        let href = escape_relative_url(&relative_path(&page_dir, &destination)?);
        result.push_str(&cap[1]);
        result.push_str(&href);
        result.push_str(&cap[4]);
        cursor = whole.end();
    }
    result.push_str(&html[cursor..]);
    Ok(result)
}

fn render_mermaid(html: &str) -> Result<String, String> {
    let pattern = Regex::new(
        r#"(?s)<pre(?:\s[^>]*)?>\s*<code\b[^>]*\bdata-lang="mermaid"[^>]*>(.*?)</code>\s*</pre>"#,
    )
    .unwrap();
    if !pattern.is_match(html) {
        return Ok(html.into());
    }
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(256 * 1024 * 1024);
    runtime.set_max_stack_size(4 * 1024 * 1024);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    runtime.set_interrupt_handler(Some(Box::new(move || {
        std::time::Instant::now() >= deadline
    })));
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| -> Result<String, String> {
        ctx.eval::<(), _>(PELE).catch(&ctx).map_err(|e| format!("Pele initialization: {e}"))?;
        let function: Function = ctx.eval("(source, id) => pele.render(source, {idPrefix: id, autoDirection: false, maxWidth: 900, images: false}).svg").catch(&ctx).map_err(|e| e.to_string())?;
        let mut result = String::new(); let mut last = 0;
        for (index, capture) in pattern.captures_iter(html).enumerate() {
            let whole = capture.get(0).unwrap(); result.push_str(&html[last..whole.start()]);
            let source = quick_xml::escape::unescape(capture.get(1).unwrap().as_str()).map_err(|e| format!("invalid Mermaid source: {e}"))?;
            let svg: String = function.call((source.as_ref(), format!("diagram-{index}"))).catch(&ctx).map_err(|e| format!("Mermaid diagram {}: {e}", index + 1))?;
            result.push_str("<pre class=\"mermaid\">"); result.push_str(&svg); result.push_str("</pre>"); last = whole.end();
        }
        result.push_str(&html[last..]); Ok(result)
    })
}

fn default_site_output(site: &Path, project_root: &Path, source: &Path) -> Result<PathBuf, String> {
    let relative = source
        .strip_prefix(project_root)
        .map_err(|_| "source must be inside --root when output is omitted")?;
    Ok(site.join(relative).with_extension(""))
}

struct CompileOptions {
    pass: Vec<String>,
    positional: Vec<String>,
    site: Option<PathBuf>,
    assets_url: Option<String>,
}

fn parse_compile_options(args: &[String]) -> Result<CompileOptions, String> {
    let mut result = CompileOptions {
        pass: Vec::new(),
        positional: Vec::new(),
        site: None,
        assets_url: None,
    };
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            result.pass.extend_from_slice(&args[i + 1..]);
            break;
        }
        let (flag, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(name, value)| (name, Some(value)));
        if matches!(flag, "--site-root" | "--assets-url" | "--output" | "-o") {
            let value = if let Some(value) = inline {
                value.to_owned()
            } else {
                i += 1;
                args.get(i)
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone()
            };
            if value.is_empty() {
                return Err(format!("{flag} requires a value"));
            }
            match flag {
                "--site-root" => result.site = Some(value.into()),
                "--assets-url" => result.assets_url = Some(value),
                _ => output = Some(value),
            }
        } else if arg.starts_with('-') {
            result.pass.push(arg.clone());
            if inline.is_none()
                && matches!(
                    flag,
                    "--root"
                        | "--input"
                        | "--font-path"
                        | "--package-path"
                        | "--package-cache-path"
                        | "--creation-timestamp"
                        | "--pages"
                        | "--pdf-standard"
                        | "--ppi"
                        | "--deps"
                        | "--deps-format"
                        | "--jobs"
                        | "-j"
                        | "--features"
                        | "--diagnostic-format"
                        | "--timings"
                        | "--format"
                        | "-f"
                )
            {
                i += 1;
                result.pass.push(
                    args.get(i)
                        .ok_or_else(|| format!("{flag} requires a value"))?
                        .clone(),
                );
            }
        } else {
            result.positional.push(arg.clone());
        }
        i += 1;
    }
    if let Some(output) = output {
        if result.positional.len() != 1 {
            return Err("provide one input and one output".into());
        }
        result.positional.push(output);
    }
    Ok(result)
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("preview") => return Err("unknown command preview; use serve".into()),
        Some("serve")
            if args.get(1).is_some_and(|arg| !arg.starts_with('-'))
                || args
                    .iter()
                    .skip(1)
                    .any(|arg| arg == "--root" || arg.starts_with("--root=")) =>
        {
            site_preview::compile(args[1..].to_vec())?;
            return Ok(0);
        }
        Some("serve")
            if args
                .get(1)
                .is_some_and(|arg| arg == "--help" || arg == "-h") =>
        {
            println!(
                "Usage: ypst serve INPUT [--root DIR]\n       ypst serve [--site-root DIR] [--port PORT]"
            );
            return Ok(0);
        }
        Some("--help") | Some("-h") if args.len() == 1 => {
            println!(
                "Usage: ypst <compile|serve|stop|install> [OPTIONS]\n\ncompile INPUT [OUTPUT] [--site-root DIR] [-- TYPST_OPTIONS]\nserve INPUT [--root DIR] | --site-root DIR [--port PORT]\nstop\ninstall"
            );
            return Ok(0);
        }
        Some("--version") | Some("-V") if args.len() == 1 => {
            println!("ypst {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }
        Some("stop") if args.len() == 1 => {
            site_preview::stop()?;
            return Ok(0);
        }
        Some("serve") | Some("__site_preview_server") => {
            let mut dir = PathBuf::from(".");
            let mut port = 8765;
            if args[0] == "__site_preview_server" && args.len() == 3 {
                dir = args[1].clone().into();
                port = args[2].parse::<u16>().map_err(|e| e.to_string())?;
            } else {
                let mut i = 1;
                while i < args.len() {
                    let flag = &args[i];
                    i += 1;
                    let value = args.get(i).ok_or("missing serve option value")?;
                    match flag.as_str() {
                        "--site-root" | "--dir" => dir = value.into(),
                        "--port" => port = value.parse::<u16>().map_err(|e| e.to_string())?,
                        _ => return Err(format!("unknown option {flag}")),
                    };
                    i += 1;
                }
            }
            site_preview::serve(&dir, port)?;
            return Ok(0);
        }
        _ => {}
    }
    if args.first().is_some_and(|s| s == "compile")
        && args.len() == 2
        && matches!(args[1].as_str(), "--help" | "-h")
    {
        println!(
            "Usage: ypst compile [OPTIONS] INPUT [OUTPUT]\n\n  -o, --output PATH       Output path\n      --site-root DIR     Write a site bundle\n      --assets-url URL    Public URL for shared site styles\n      --root DIR          Typst project root\n  -f, --format FORMAT     Output format: html or pdf\n\nOther Typst compile options are forwarded; use -- to pass them through explicitly."
        );
        return Ok(0);
    }
    if args.first().is_some_and(|s| s == "install") {
        if args.len() != 1 {
            return Err("usage: ypst install".into());
        }
        let target = package_directory(
            env::consts::OS,
            env::var("APPDATA").ok().as_deref(),
            env::var("XDG_DATA_HOME").ok().as_deref(),
            env::var("HOME")
                .ok()
                .or_else(|| env::var("USERPROFILE").ok())
                .as_deref(),
        )?;
        install_package(&target)?;
        println!(
            "Installed @local/ypst-template:0.1.0 to {}",
            target.display()
        );
        return Ok(0);
    }
    if args.is_empty() || args[0] != "compile" {
        return Command::new("typst")
            .args(&args)
            .status()
            .map(|s| s.code().unwrap_or(1))
            .map_err(|e| e.to_string());
    }
    let CompileOptions {
        pass,
        positional,
        site,
        assets_url,
    } = parse_compile_options(&args[1..])?;
    if positional.is_empty() {
        return Command::new("typst")
            .args(&args)
            .status()
            .map(|s| s.code().unwrap_or(1))
            .map_err(|e| e.to_string());
    }
    if positional.len() > 2 {
        return Err("expected source and optional output".into());
    }
    if assets_url.is_some() && site.is_none() {
        return Err("--assets-url requires --site-root".into());
    }
    if positional.iter().any(|arg| arg == "-") {
        return Err("stdin and stdout are not supported".into());
    }
    let source = fs::canonicalize(&positional[0]).map_err(|e| e.to_string())?;
    let format_opt = pass
        .iter()
        .position(|x| x == "--format" || x == "-f")
        .and_then(|i| pass.get(i + 1))
        .cloned()
        .or_else(|| {
            pass.iter()
                .find_map(|x| x.strip_prefix("--format=").map(str::to_owned))
        });
    let source_text = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    let source_lines = source_text.lines().count();
    let project_root = pass
        .iter()
        .enumerate()
        .find_map(|(index, arg)| {
            if arg == "--root" {
                pass.get(index + 1).cloned()
            } else {
                arg.strip_prefix("--root=").map(str::to_owned)
            }
        })
        .unwrap_or_else(|| ".".into());
    let project_root = fs::canonicalize(project_root).map_err(|e| e.to_string())?;
    if !source.starts_with(&project_root) {
        return Err("source is outside --root".into());
    }
    let (processed, _graph_files) = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("typst-graphs".into())
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || {
                graphs::preprocess(&source_text, &source, &project_root)
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "graph renderer thread failed".to_string())?
    })?;
    let pdf = format_opt.as_deref() == Some("pdf")
        || (format_opt.is_none()
            && positional
                .get(1)
                .is_some_and(|s| s.to_ascii_lowercase().ends_with(".pdf")));
    if pdf {
        if site.is_some() {
            return Err("--site-root is only supported for HTML".into());
        }
        let mut entry = None;
        if processed != source_text {
            let mut temp = tempfile::Builder::new()
                .prefix(".typst-graphs-entry-")
                .suffix(".typ")
                .tempfile_in(source.parent().unwrap())
                .map_err(|e| e.to_string())?;
            temp.write_all(processed.as_bytes())
                .map_err(|e| e.to_string())?;
            entry = Some(temp);
        }
        let mut cmd = Command::new("typst");
        cmd.arg("compile")
            .arg(entry.as_ref().map(|e| e.path()).unwrap_or(&source));
        let default_output = source.with_extension("pdf");
        cmd.arg(positional.get(1).map(Path::new).unwrap_or(&default_output));
        if !pass
            .iter()
            .any(|a| a == "--diagnostic-format" || a.starts_with("--diagnostic-format="))
        {
            cmd.args(["--diagnostic-format", "short"]);
        }
        cmd.args(&pass);
        return run_compiler(
            &mut cmd,
            entry.as_ref().map(|e| e.path()),
            &positional[0],
            source_lines,
            0,
            &pass,
        )
        .map(|s| s.code().unwrap_or(1));
    }
    if format_opt.as_deref().is_some_and(|f| f != "html") {
        return Err("ypst HTML mode supports --format html or pdf".into());
    }
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let site = site.map(|p| if p.is_absolute() { p } else { cwd.join(p) });
    let mut output = if let Some(out) = positional.get(1) {
        let p = PathBuf::from(out);
        if p.is_absolute() { p } else { cwd.join(p) }
    } else if let Some(root) = &site {
        default_site_output(root, &project_root, &source)?
    } else {
        source.with_extension("html")
    };
    if output == source || fs::canonicalize(&output).ok().as_ref() == Some(&source) {
        return Err("output must not overwrite source".into());
    }
    if site.is_some()
        && output
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("html"))
    {
        output.set_extension("bundle");
    }
    if let Some(root) = &site {
        let absolute_output = if output.exists() {
            fs::canonicalize(&output).map_err(|e| e.to_string())?
        } else {
            output.clone()
        };
        let shared = root.join("_shared");
        if shared.starts_with(&absolute_output) {
            return Err("bundle output must not contain the shared assets directory".into());
        }
    }
    if pass.iter().any(|x| {
        x == "--deps"
            || x == "--deps-format"
            || x.starts_with("--deps=")
            || x.starts_with("--deps-format=")
    }) {
        return Err("dependency output is managed by the wrapper".into());
    }
    let target = if site.is_some() {
        output.join("index.html")
    } else {
        output.clone()
    };
    fs::create_dir_all(target.parent().ok_or("invalid output")?).map_err(|e| e.to_string())?;
    let mut compiler = Command::new("typst");
    compiler.arg("compile");
    let mut entry = None;
    if let Some(root) = &site {
        let assets = shared_assets(root)?;
        let prefix = if let Some(url) = assets_url {
            asset_url(&url)?
        } else {
            format!(
                "{}/",
                escape_relative_url(&relative_path(&output, &assets)?)
            )
        };
        let text = &processed;
        let mut temp = tempfile::Builder::new()
            .prefix(".typst-bundle-")
            .suffix(".typ")
            .tempfile_in(source.parent().unwrap())
            .map_err(|e| e.to_string())?;
        write!(temp, "#document(\"index.html\")[\n{text}\n]\n").map_err(|e| e.to_string())?;
        compiler.args([
            "--format",
            "bundle",
            "--features",
            "bundle,html",
            "--input",
            "ypst-template-bundle=true",
            "--input",
            &format!("ypst-template-assets-prefix={prefix}"),
        ]);
        entry = Some(temp);
    } else {
        if processed != source_text {
            let mut temp = tempfile::Builder::new()
                .prefix(".typst-graphs-entry-")
                .suffix(".typ")
                .tempfile_in(source.parent().unwrap())
                .map_err(|e| e.to_string())?;
            temp.write_all(processed.as_bytes())
                .map_err(|e| e.to_string())?;
            entry = Some(temp);
        }
        compiler.args(["--format", "html", "--features", "html"]);
    }
    if !pass
        .iter()
        .any(|x| x == "--root" || x.starts_with("--root="))
    {
        compiler.args(["--root", "."]);
    }
    if !pass
        .iter()
        .any(|x| x == "--diagnostic-format" || x.starts_with("--diagnostic-format="))
    {
        compiler.args(["--diagnostic-format", "short"]);
    }
    // The wrapper owns the HTML output format.
    let mut skip = false;
    for arg in &pass {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--format" || arg == "-f" {
            skip = true;
            continue;
        }
        if arg.starts_with("--format=") {
            continue;
        }
        compiler.arg(arg);
    }
    compiler
        .arg(entry.as_ref().map(|t| t.path()).unwrap_or(&source))
        .arg(&output);
    let status = run_compiler(
        &mut compiler,
        entry.as_ref().map(|e| e.path()),
        &positional[0],
        source_lines,
        usize::from(site.is_some()),
        &pass,
    )?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }
    let html = fs::read_to_string(&target).map_err(|e| e.to_string())?;
    let rendered = render_mermaid(&html)?;
    let rendered = if let Some(root) = &site {
        externalize_site_images(&rendered, root, &target)?
    } else {
        rendered
    };
    if rendered != html {
        let mut temp = tempfile::NamedTempFile::new_in(target.parent().ok_or("invalid output")?)
            .map_err(|e| e.to_string())?;
        temp.write_all(rendered.as_bytes())
            .map_err(|e| e.to_string())?;
        temp.persist(&target).map_err(|e| e.to_string())?;
    }
    Ok(0)
}
fn is_source_diagnostic(message: &str) -> bool {
    regex::Regex::new(r"^.+:[0-9]+:[0-9]+: (?:error|warning):")
        .unwrap()
        .is_match(message)
}

fn normalized_exit(code: i32) -> ExitCode {
    if code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => normalized_exit(code),
        Err(error) => {
            if is_source_diagnostic(&error) {
                eprintln!("{error}");
            } else {
                eprintln!("ypst: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_exit_codes_follow_public_contract() {
        assert_eq!(normalized_exit(0), ExitCode::SUCCESS);
        assert_eq!(normalized_exit(1), ExitCode::FAILURE);
        assert_eq!(normalized_exit(2), ExitCode::FAILURE);
        assert_eq!(normalized_exit(42), ExitCode::FAILURE);
    }
    #[test]
    fn renders_mermaid_to_svg_without_browser_javascript() {
        let input =
            "<div><pre><code data-lang=\"mermaid\">graph TD\n A --&gt; B</code></pre></div>";
        let result = render_mermaid(input).unwrap();
        assert!(result.contains("<svg"));
        assert!(result.contains("class=\"mermaid\""));
        assert!(!result.contains("data-lang=\"mermaid\""));
    }
    #[test]
    fn install_uses_fixed_version_and_preserves_other_files() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("ypst-template/0.1.0");
        fs::create_dir_all(target.join("fonts")).unwrap();
        fs::write(target.join("fonts/custom.ttf"), b"font").unwrap();
        install_package(&target).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("lib.typ")).unwrap(),
            include_str!("../lib.typ")
        );
        assert_eq!(fs::read(target.join("fonts/custom.ttf")).unwrap(), b"font");
        install_package(&target).unwrap();
    }
    #[test]
    fn package_path_respects_xdg_and_fixed_version() {
        let path = package_directory("linux", None, Some("relative"), Some("/home/test")).unwrap();
        assert_eq!(
            path,
            PathBuf::from("/home/test/.local/share/typst/packages/local/ypst-template/0.1.0")
        );
        assert_eq!(
            package_directory("linux", None, Some("/data"), Some("/home/test")).unwrap(),
            PathBuf::from("/data/typst/packages/local/ypst-template/0.1.0")
        );
    }
    #[test]
    fn maps_only_diagnostics_from_wrapped_source() {
        let entry = tempfile::NamedTempFile::new().unwrap();
        let line = format!("{}:4:7: error: bad value", entry.path().display());
        assert_eq!(
            remap_diagnostic(&line, entry.path(), "note.typ", 5, 1),
            "note.typ:3:7: error: bad value"
        );
        let unrelated = "import.typ:4:7: error: bad value";
        assert_eq!(
            remap_diagnostic(unrelated, entry.path(), "note.typ", 5, 1),
            unrelated
        );
        let outside = format!("{}:1:1: error: wrapper", entry.path().display());
        assert_eq!(
            remap_diagnostic(&outside, entry.path(), "note.typ", 5, 1),
            outside
        );
        assert!(!short_diagnostics(&["--diagnostic-format=human".into()]));
        assert!(short_diagnostics(&[
            "--diagnostic-format=human".into(),
            "--diagnostic-format".into(),
            "short".into()
        ]));
    }
    #[test]
    fn validates_and_escapes_asset_paths() {
        assert_eq!(
            asset_url("https://example.org/static").unwrap(),
            "https://example.org/static/"
        );
        assert!(asset_url("file://host/path").is_err());
        assert!(asset_url("/static?x=1").is_err());
        assert_eq!(escape_relative_url("../a b/图"), "../a%20b/%E5%9B%BE");
    }
    #[test]
    fn site_output_is_relative_to_project_root() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let source = project.join("nested/note.typ");
        let site = tmp.path().join("site");
        assert_eq!(
            default_site_output(&site, &project, &source).unwrap(),
            site.join("nested/note")
        );
        assert!(default_site_output(&site, &project, &tmp.path().join("elsewhere.typ")).is_err());
    }
    #[test]
    fn source_diagnostics_remain_parseable() {
        assert!(super::is_source_diagnostic(
            "C:\\notes\\a.typ:12:3: error: invalid"
        ));
        assert!(!super::is_source_diagnostic("cannot start server"));
    }
    #[test]
    fn compile_options_accept_typst_and_site_flags_before_or_after_input() {
        for args in [
            vec!["--root", "/project", "note.typ", "--site-root", "_site"],
            vec!["note.typ", "--site-root=_site", "--root=/project"],
        ] {
            let parsed =
                parse_compile_options(&args.into_iter().map(str::to_owned).collect::<Vec<_>>())
                    .unwrap();
            assert_eq!(parsed.positional, ["note.typ"]);
            assert_eq!(parsed.site, Some(PathBuf::from("_site")));
            assert!(parsed.pass.iter().any(|arg| arg.starts_with("--root")));
        }
        let args = ["-o", "out.pdf", "--format", "pdf", "note.typ"].map(str::to_owned);
        let parsed = parse_compile_options(&args).unwrap();
        assert_eq!(parsed.positional, ["note.typ", "out.pdf"]);
        assert_eq!(parsed.pass, ["--format", "pdf"]);
    }
    #[test]
    fn site_images_are_shared_and_script_text_is_preserved() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site space");
        let page = site.join("a/b/index.html");
        fs::create_dir_all(page.parent().unwrap()).unwrap();
        let html = r#"<script>const sample = '<img src="data:image/png;base64,aGVsbG8=">';</script><img alt="x" src="data:image/png;base64,aGVsbG8="><img src="data:image/png;base64,aGVsbG8=">"#;
        let output = externalize_site_images(html, &site, &page).unwrap();
        assert!(output.contains("const sample = '<img src=\"data:image/png;base64,aGVsbG8=\">'"));
        assert_eq!(output.matches("../../_shared/typst/images/").count(), 2);
        let files = fs::read_dir(site.join("_shared/typst/images"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(fs::read(files[0].path()).unwrap(), b"hello");
        let second_page = site.join("other/index.html");
        fs::create_dir_all(second_page.parent().unwrap()).unwrap();
        let second = externalize_site_images(html, &site, &second_page).unwrap();
        assert_eq!(second.matches("../_shared/typst/images/").count(), 2);
        assert_eq!(
            fs::read_dir(site.join("_shared/typst/images"))
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            externalize_site_images(&output, &site, &page).unwrap(),
            output
        );
    }
    #[test]
    fn preserves_html_without_mermaid() {
        let input = "<p>Hello</p>";
        assert_eq!(render_mermaid(input).unwrap(), input);
    }
}
