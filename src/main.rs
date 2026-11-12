mod assets;
mod preprocess;
mod site_images;
mod site_preview;
mod svg_glyphs;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    sync::LazyLock,
};

const BUILD_COMMIT: &str = env!("YPST_BUILD_COMMIT");
const COMMIT_FILE: &str = ".ypst-commit";

const MONASPACE_WOFF: &[u8] = include_bytes!("../html/monaspace.woff");
const MONASPACE_CSS: &str = include_str!("../html/monaspace.css");
const THEME: &str = include_str!("../html/theme.css");
const HIGHLIGHT_CSS: &str = include_str!("../html/highlight.css");
const FOOTNOTES: &str = include_str!("../html/footnotes.css");
const HEADINGS: &str = include_str!("../html/headings.css");

const PACKAGE_FILES: &[(&str, &str)] = &[
    ("typst.toml", include_str!("../typst.toml")),
    ("lib.typ", include_str!("../lib.typ")),
    ("html/html.typ", include_str!("../html/html.typ")),
    ("html/theme.css", THEME),
    (
        "fonts/LICENSE-Monaspace.txt",
        include_str!("../fonts/LICENSE-Monaspace.txt"),
    ),
    ("html/highlight.css", HIGHLIGHT_CSS),
    (
        "html/highlight.tmTheme",
        include_str!("../html/highlight.tmTheme"),
    ),
    ("html/footnotes.css", FOOTNOTES),
    ("html/headings.css", HEADINGS),
    ("html/links.js", include_str!("../html/links.js")),
    ("html/footnotes.js", include_str!("../html/footnotes.js")),
    ("html/headings.js", include_str!("../html/headings.js")),
    (
        "html/line_navigation.js",
        include_str!("../html/line_navigation.js"),
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

fn default_package_directory() -> Result<PathBuf, String> {
    package_directory(
        env::consts::OS,
        env::var("APPDATA").ok().as_deref(),
        env::var("XDG_DATA_HOME").ok().as_deref(),
        env::var("HOME")
            .ok()
            .or_else(|| env::var("USERPROFILE").ok())
            .as_deref(),
    )
}

fn compile_package_directory(args: &[String]) -> Result<PathBuf, String> {
    let mut root = env::var_os("TYPST_PACKAGE_PATH").map(PathBuf::from);
    for (i, arg) in args.iter().enumerate() {
        if arg == "--package-path" {
            root = Some(PathBuf::from(
                args.get(i + 1).ok_or("missing --package-path value")?,
            ));
        } else if let Some(value) = arg.strip_prefix("--package-path=") {
            root = Some(PathBuf::from(value));
        }
    }
    match root {
        Some(root) => Ok(root.join("local/ypst-template/0.1.0")),
        None => default_package_directory(),
    }
}

fn ensure_package(target: &Path) -> Result<bool, String> {
    if fs::read_to_string(target.join(COMMIT_FILE)).ok().as_deref() == Some(BUILD_COMMIT) {
        return Ok(false);
    }
    install_package(target)?;
    Ok(true)
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
    fs::write(target.join(COMMIT_FILE), BUILD_COMMIT)
        .map_err(|e| format!("{}: {e}", target.join(COMMIT_FILE).display()))?;
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
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(.*):(\d+):(\d+): (error|warning|help):").unwrap());
    let Some(capture) = PATTERN.captures(line) else {
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

fn changed_entry(
    source: &Path,
    original: &str,
    processed: &str,
) -> Result<Option<tempfile::NamedTempFile>, String> {
    if processed == original {
        return Ok(None);
    }
    let mut file = tempfile::Builder::new()
        .prefix(".ypst-entry-")
        .suffix(".typ")
        .tempfile_in(source.parent().ok_or("source has no parent")?)
        .map_err(|e| e.to_string())?;
    file.write_all(processed.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(Some(file))
}

fn validate_output(source: &Path, output: &Path) -> Result<(), String> {
    if output == source || fs::canonicalize(output).ok().as_deref() == Some(source) {
        return Err("output must not overwrite source".into());
    }
    Ok(())
}

fn shared_assets(root: &Path) -> Result<PathBuf, String> {
    let css = [MONASPACE_CSS, THEME, HIGHLIGHT_CSS, FOOTNOTES, HEADINGS].join("\n");
    let js = format!(
        "(() => {{\n{}\n{}\n{}\n{}\n}})();\n",
        include_str!("../html/links.js"),
        include_str!("../html/footnotes.js"),
        include_str!("../html/headings.js"),
        include_str!("../html/line_navigation.js")
    );
    let mut hash = Sha256::new();
    for (name, data) in [
        ("theme.css", css.as_bytes()),
        ("theme.js", js.as_bytes()),
        ("monaspace.woff", MONASPACE_WOFF),
    ] {
        hash.update(name.as_bytes());
        hash.update(data);
    }
    let dir = root
        .join("_shared/typst")
        .join(&format!("{:x}", hash.finalize())[..16]);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, data) in [
        ("theme.css", css.as_bytes()),
        ("theme.js", js.as_bytes()),
        ("monaspace.woff", MONASPACE_WOFF),
    ] {
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

fn default_site_output(site: &Path, project_root: &Path, source: &Path) -> Result<PathBuf, String> {
    let relative = source
        .strip_prefix(project_root)
        .map_err(|_| "source must be inside --root when output is omitted")?;
    Ok(site.join(relative))
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
    let mut pdf = false;
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
        if flag == "--pdf" {
            if inline.is_some() {
                return Err("--pdf does not accept a value".into());
            }
            pdf = true;
        } else if matches!(flag, "--site-root" | "--assets-url" | "--output" | "-o") {
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
    if pdf {
        if result
            .pass
            .iter()
            .any(|arg| arg == "--format" || arg == "-f" || arg.starts_with("--format="))
        {
            return Err("--pdf conflicts with --format; specify only one".into());
        }
        if result.site.is_some() {
            return Err("--pdf conflicts with --site-root".into());
        }
        result.pass.extend(["--format".into(), "pdf".into()]);
    }
    if let Some(output) = output {
        if result.positional.len() != 1 {
            return Err("provide one input and one output".into());
        }
        result.positional.push(output);
    }
    if result.positional.get(1).is_some_and(|output| {
        Path::new(output)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    }) && !result
        .pass
        .iter()
        .any(|arg| arg == "--format" || arg == "-f" || arg.starts_with("--format="))
    {
        if result.site.is_some() {
            return Err("PDF output conflicts with --site-root".into());
        }
        result.pass.extend(["--format".into(), "pdf".into()]);
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
                "Usage: ypst serve INPUT [--root DIR] [--fragment FRAGMENT]\n       ypst serve [--site-root DIR] [--port PORT]"
            );
            return Ok(0);
        }
        Some("--help") | Some("-h") if args.len() == 1 => {
            println!(
                "Usage: ypst <compile|serve|stop|install> [OPTIONS]\n\ncompile INPUT [OUTPUT] [--pdf | --site-root DIR] [-- TYPST_OPTIONS]\nserve INPUT [--root DIR] [--fragment FRAGMENT] | --site-root DIR [--port PORT]\nstop\ninstall\n\nRepository and documentation: https://github.com/jay-waves/ypst"
            );
            return Ok(0);
        }
        Some("--version") | Some("-V") if args.len() == 1 => {
            println!(
                "ypst {} (commit {})",
                env!("CARGO_PKG_VERSION"),
                BUILD_COMMIT
            );
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
            "Usage: ypst compile [OPTIONS] INPUT [OUTPUT]\n\n  -o, --output PATH       Output path (.pdf selects PDF)\n      --site-root DIR     Write a site bundle\n      --assets-url URL    Public URL for shared site styles\n      --root DIR          Typst project root\n      --pdf               Output PDF (default: HTML)\n  -f, --format FORMAT     Explicit output format: html or pdf\n\nOther Typst compile options are forwarded; use -- to pass them through explicitly.\nRepository and documentation: https://github.com/jay-waves/ypst"
        );
        return Ok(0);
    }
    if args.first().is_some_and(|s| s == "install") {
        if args.len() != 1 {
            return Err("usage: ypst install".into());
        }
        let target = default_package_directory()?;
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
    let package = compile_package_directory(&pass)?;
    if ensure_package(&package)? {
        eprintln!(
            "Updated @local/ypst-template:0.1.0 to commit {BUILD_COMMIT} at {}",
            package.display()
        );
    }
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
    let pdf = format_opt.as_deref() == Some("pdf");
    if pdf && site.is_some() {
        return Err("--site-root is only supported for HTML".into());
    }
    if format_opt
        .as_deref()
        .is_some_and(|f| !matches!(f, "html" | "pdf"))
    {
        return Err("ypst HTML mode supports --format html or pdf".into());
    }
    let assets_url = assets_url.as_deref().map(asset_url).transpose()?;

    if !pdf
        && positional
            .get(1)
            .is_some_and(|s| s.to_ascii_lowercase().ends_with(".pdf"))
    {
        return Err(".pdf output conflicts with the explicit output format".into());
    }
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let site = site.map(|p| if p.is_absolute() { p } else { cwd.join(p) });
    let mut output = if let Some(out) = positional.get(1) {
        let p = PathBuf::from(out);
        if p.is_absolute() { p } else { cwd.join(p) }
    } else if let Some(root) = &site {
        default_site_output(root, &project_root, &source)?
    } else {
        source.with_extension(if pdf { "pdf" } else { "html" })
    };
    validate_output(&source, &output)?;
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
    let site_images = site.as_ref().map(|site| site_images::SiteImages {
        site: site.clone(),
        page: output.clone(),
    });
    let prepared =
        preprocess::Prepared::with_site_images(&source, &project_root, !pdf, site_images.as_ref())?;
    let processed = &prepared.text;
    if pdf {
        let entry = changed_entry(&source, &source_text, processed)?;
        let mut cmd = Command::new("typst");
        cmd.arg("compile")
            .arg(entry.as_ref().map(|e| e.path()).unwrap_or(&source));
        cmd.arg(&output);
        if !pass
            .iter()
            .any(|a| a == "--diagnostic-format" || a.starts_with("--diagnostic-format="))
        {
            cmd.args(["--diagnostic-format", "short"]);
        }
        cmd.args(&pass);
        let status = run_compiler(
            &mut cmd,
            entry.as_ref().map(|e| e.path()),
            &positional[0],
            source_lines,
            0,
            &pass,
        )?;
        return Ok(status.code().unwrap_or(1));
    }
    let target = if site.is_some() {
        output.join("index.html")
    } else {
        output.clone()
    };
    fs::create_dir_all(target.parent().ok_or("invalid output")?).map_err(|e| e.to_string())?;
    let mut compiler = Command::new("typst");
    compiler.arg("compile");
    let entry;
    if let Some(root) = &site {
        let assets = shared_assets(root)?;
        let prefix = if let Some(url) = assets_url {
            url
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
            "--input",
            &format!(
                "ypst-template-title={}",
                source
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Document")
            ),
        ]);
        entry = Some(temp);
    } else {
        entry = changed_entry(&source, &source_text, processed)?;
        compiler.args(["--format", "html", "--features", "html"]);
        compiler.args([
            "--input",
            &format!(
                "ypst-template-title={}",
                source
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Document")
            ),
        ]);
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
    svg_glyphs::optimize_file(&target)?;
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
    fn output_cannot_alias_source_and_unchanged_entries_need_no_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.typ");
        fs::write(&source, "hello").unwrap();
        let source = fs::canonicalize(source).unwrap();
        assert!(validate_output(&source, &source).is_err());
        assert!(validate_output(&source, &source.with_extension("pdf")).is_ok());
        assert!(changed_entry(&source, "hello", "hello").unwrap().is_none());
        let copy = changed_entry(&source, "hello", "changed").unwrap().unwrap();
        assert_eq!(copy.path().parent(), source.parent());
        assert_eq!(fs::read_to_string(copy.path()).unwrap(), "changed");
        assert_eq!(fs::read_to_string(&source).unwrap(), "hello");
        #[cfg(unix)]
        {
            let alias = dir.path().join("alias.pdf");
            std::os::unix::fs::symlink(&source, &alias).unwrap();
            assert!(validate_output(&source, &alias).is_err());
        }
    }

    #[test]
    fn shared_assets_separate_font_and_repair_corruption() {
        let root = tempfile::tempdir().unwrap();
        let dir = shared_assets(root.path()).unwrap();
        let css = fs::read_to_string(dir.join("theme.css")).unwrap();
        let font = fs::read(dir.join("monaspace.woff")).unwrap();
        assert!(css.contains("url(\"monaspace.woff\")"));
        assert!(!css.contains("data:font/woff;base64,"));
        assert!(font.starts_with(b"wOFF"));
        assert_eq!(font, MONASPACE_WOFF);
        let modified = fs::metadata(dir.join("monaspace.woff"))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(shared_assets(root.path()).unwrap(), dir);
        assert_eq!(
            fs::metadata(dir.join("monaspace.woff"))
                .unwrap()
                .modified()
                .unwrap(),
            modified
        );
        fs::write(dir.join("monaspace.woff"), b"damaged").unwrap();
        shared_assets(root.path()).unwrap();
        assert_eq!(fs::read(dir.join("monaspace.woff")).unwrap(), font);
    }

    #[test]
    fn standalone_html_uses_remote_font_and_local_fallback() {
        let template = include_str!("../html/html.typ");
        assert!(template.contains("https://cdn.jsdelivr.net/gh/githubnext/monaspace@v1.400/"));
        assert!(!template.contains("read(\"monaspace.css\")"));
        assert!(THEME.contains("\"Monaspace Neon\", Consolas, Menlo, ui-monospace, monospace"));
        assert!(template.contains("document.title = \" + repr(title)"));
    }

    #[test]
    fn pdf_is_selected_by_flag_or_output_extension() {
        let parse = |args: &[&str]| {
            parse_compile_options(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
        };
        assert!(parse(&["note.typ"]).unwrap().pass.is_empty());
        for args in [
            vec!["--pdf", "note.typ"],
            vec!["note.typ", "--pdf"],
            vec!["note.typ", "--pdf", "--", "--root", "."],
            vec!["note.typ", "out.pdf"],
            vec!["note.typ", "-o", "out.PDF"],
            vec!["--output=out.pdf", "note.typ"],
        ] {
            let result = parse(&args).unwrap();
            assert!(
                result
                    .pass
                    .windows(2)
                    .any(|pair| pair == ["--format", "pdf"])
            );
            assert!(!result.pass.contains(&"--pdf".into()));
        }
        assert!(parse(&["note.typ", "--pdf", "--site-root", "site"]).is_err());
        assert!(parse(&["note.typ", "--pdf", "--format", "html"]).is_err());
        assert!(parse(&["note.typ", "--pdf=true"]).is_err());
        assert!(parse(&["note.typ", "out.pdf", "--site-root", "site"]).is_err());
        let explicit = parse(&["note.typ", "out.pdf", "--format", "html"]).unwrap();
        assert_eq!(explicit.pass, ["--format", "html"]);
    }
    #[test]
    fn child_exit_codes_follow_public_contract() {
        assert_eq!(normalized_exit(0), ExitCode::SUCCESS);
        assert_eq!(normalized_exit(1), ExitCode::FAILURE);
        assert_eq!(normalized_exit(2), ExitCode::FAILURE);
        assert_eq!(normalized_exit(42), ExitCode::FAILURE);
    }
    #[test]
    fn package_commit_controls_automatic_refresh() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("package");
        assert!(ensure_package(&target).unwrap());
        assert_eq!(
            fs::read_to_string(target.join(COMMIT_FILE)).unwrap(),
            BUILD_COMMIT
        );
        fs::write(target.join("lib.typ"), "local edit").unwrap();
        assert!(!ensure_package(&target).unwrap());
        assert_eq!(
            fs::read_to_string(target.join("lib.typ")).unwrap(),
            "local edit"
        );
        fs::write(target.join(COMMIT_FILE), "older commit").unwrap();
        assert!(ensure_package(&target).unwrap());
        assert_eq!(
            fs::read_to_string(target.join("lib.typ")).unwrap(),
            include_str!("../lib.typ")
        );
        fs::remove_file(target.join(COMMIT_FILE)).unwrap();
        assert!(ensure_package(&target).unwrap());
    }

    #[test]
    fn compile_package_path_accepts_both_option_forms() {
        for args in [
            vec!["--package-path", "custom"],
            vec!["--package-path=custom"],
        ] {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            assert_eq!(
                compile_package_directory(&args).unwrap(),
                PathBuf::from("custom/local/ypst-template/0.1.0")
            );
        }
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
    #[cfg(not(windows))]
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
    fn package_path_respects_windows_appdata_and_fixed_version() {
        let appdata = std::env::temp_dir().join("ypst-test-appdata");
        let path = package_directory("windows", appdata.to_str(), None, None).unwrap();
        assert_eq!(
            path,
            appdata.join("typst/packages/local/ypst-template/0.1.0")
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
            site.join("nested/note.typ")
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
}
