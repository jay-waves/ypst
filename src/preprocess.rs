use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use typst_syntax::{
    LinkedNode, Source,
    ast::{self, AstNode},
};

/// Sibling copies preserve caller-relative paths without editing original files.
#[derive(Default)]
pub struct Prepared {
    pub text: String,
    files: Vec<tempfile::NamedTempFile>,
    downloads: Vec<tempfile::TempDir>,
    copies: BTreeMap<PathBuf, PathBuf>,
    active: BTreeSet<PathBuf>,
}
impl Prepared {
    #[cfg(test)]
    pub fn new(entry: &Path, root: &Path, html: bool) -> Result<Self, String> {
        Self::with_site_images(entry, root, html, None)
    }
    pub fn with_site_images(
        entry: &Path,
        root: &Path,
        html: bool,
        site: Option<&super::site_images::SiteImages>,
    ) -> Result<Self, String> {
        let mut result = Self::default();
        result.text = result.process(entry, root, html, site)?.0;
        Ok(result)
    }
    fn process(
        &mut self,
        path: &Path,
        root: &Path,
        html: bool,
        site: Option<&super::site_images::SiteImages>,
    ) -> Result<(String, bool), String> {
        if !self.active.insert(path.to_owned()) {
            return Err(format!("cyclic include/import: {}", path.display()));
        }
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let parsed = Source::detached(source.clone());
        let mut references = Vec::new();
        fn visit(
            node: &LinkedNode<'_>,
            out: &mut Vec<(std::ops::Range<usize>, String, Option<String>)>,
        ) {
            let expr = node
                .get()
                .cast::<ast::ModuleInclude>()
                .map(|v| v.source())
                .or_else(|| node.get().cast::<ast::ModuleImport>().map(|v| v.source()));
            if let Some(ast::Expr::Str(value)) = expr
                && let Some(target) = node.find(value.span())
            {
                let bare = node
                    .get()
                    .cast::<ast::ModuleImport>()
                    .filter(|import| import.imports().is_none() && import.new_name().is_none())
                    .and_then(|import| import.bare_name().ok())
                    .map(|name| name.to_string());
                out.push((target.range(), value.get().to_string(), bare));
            }
            for child in node.children() {
                visit(&child, out);
            }
        }
        visit(&LinkedNode::new(parsed.root()), &mut references);
        let mut edits = if html && self.active.len() == 1 {
            line_anchors(&parsed)
        } else {
            Vec::new()
        };
        for (range, name, bare) in references {
            if name.starts_with('@') {
                if html && name.starts_with("@preview/cetz:") {
                    // Let Typst resolve bindings and aliases normally. Re-export the
                    // package through a shim instead of guessing which calls are CeTZ.
                    let package = serde_json::to_string(&name).map_err(|e| e.to_string())?;
                    let mut shim = tempfile::Builder::new()
                        .prefix(".ypst-cetz-")
                        .suffix(".typ")
                        .tempfile_in(path.parent().unwrap())
                        .map_err(|e| e.to_string())?;
                    write!(shim, "#import {package}: *\n#let _ypst-original-canvas = canvas\n#let canvas(..args) = context {{\n  if target() == \"html\" {{\n    html.frame(_ypst-original-canvas(..args))\n  }} else {{\n    _ypst-original-canvas(..args)\n  }}\n}}\n")
                        .map_err(|e| e.to_string())?;
                    let relative = shim
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let mut replacement = serde_json::to_string(&format!("/{relative}"))
                        .map_err(|e| e.to_string())?;
                    if let Some(name) = bare {
                        replacement.push_str(&format!(" as {name}"));
                    }
                    edits.push((range, replacement));
                    self.files.push(shim);
                }
                continue;
            }
            let target = if name.starts_with('/') {
                root.join(name.trim_start_matches('/'))
            } else {
                path.parent().unwrap().join(&name)
            };
            let target =
                fs::canonicalize(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            if !target.starts_with(root) {
                return Err(format!(
                    "include/import outside --root: {}",
                    target.display()
                ));
            }
            let copy = if let Some(copy) = self.copies.get(&target) {
                copy.clone()
            } else {
                let (text, changed) = self.process(&target, root, html, site)?;
                let copy = if !changed {
                    target.clone()
                } else {
                    let mut file = tempfile::Builder::new()
                        .prefix(".ypst-source-")
                        .suffix(".typ")
                        .tempfile_in(target.parent().unwrap())
                        .map_err(|e| e.to_string())?;
                    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
                    let copy = file.path().to_owned();
                    self.files.push(file);
                    copy
                };
                self.copies.insert(target.clone(), copy.clone());
                copy
            };
            if copy != target {
                let relative = copy
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let mut replacement =
                    serde_json::to_string(&format!("/{relative}")).map_err(|e| e.to_string())?;
                if let Some(name) = bare {
                    replacement.push_str(&format!(" as {name}"));
                }
                edits.push((range, replacement));
            }
        }
        let mut text = source;
        edits.sort_by_key(|(range, _)| range.start);
        for (range, value) in edits.into_iter().rev() {
            text.replace_range(range, &value);
        }
        let (text, downloads) = super::assets::preprocess(&text, path, root)?;
        if let Some(downloads) = downloads {
            self.downloads.push(downloads);
        }
        let text = if let Some(site) = site.filter(|_| html) {
            site.rewrite(&text, path, root)?
        } else {
            text
        };
        self.active.remove(path);
        let changed = text != parsed.text();
        Ok((text, changed))
    }
}

/// Sparse anchors refer only to the entry source, before any rewriting.
/// Deliberately stay in top-level markup: reusable content and included files
/// can be rendered multiple times and would produce duplicate HTML IDs.
fn line_anchors(source: &Source) -> Vec<(std::ops::Range<usize>, String)> {
    let root = LinkedNode::new(source.root());
    let mut reserved = BTreeSet::new();
    fn labels(node: &LinkedNode<'_>, out: &mut BTreeSet<String>) {
        if let Some(label) = node.get().cast::<ast::Label>() {
            out.insert(label.get().to_owned());
        }
        for child in node.children() {
            labels(&child, out);
        }
    }
    labels(&root, &mut reserved);
    let mut edits = Vec::new();
    let mut start = true;
    for expr in source.root().cast::<ast::Markup>().unwrap().exprs() {
        let span = match expr {
            ast::Expr::Parbreak(_) => {
                start = true;
                continue;
            }
            ast::Expr::Space(_) => continue,
            ast::Expr::Heading(heading) => {
                start = true;
                heading.body().span()
            }
            ast::Expr::Text(_)
            | ast::Expr::Strong(_)
            | ast::Expr::Emph(_)
            | ast::Expr::Link(_)
            | ast::Expr::Ref(_)
            | ast::Expr::Escape(_)
            | ast::Expr::Shorthand(_)
            | ast::Expr::SmartQuote(_)
                if start =>
            {
                start = false;
                expr.span()
            }
            // Code directives do not consume the first visible text position.
            // Raw, math and list internals are intentionally left alone.
            _ => continue,
        };
        let Some(node) = root.find(span) else {
            continue;
        };
        let offset = node.range().start;
        let line = source.lines().byte_to_line(offset).unwrap() + 1;
        let id = format!("L{line}");
        if !reserved.insert(id.clone()) {
            continue;
        }
        edits.push((offset..offset, format!(
            "#html.elem(\"span\", attrs: (id: \"{id}\", \"data-line\": \"{line}\", class: \"source-line-anchor\"))"
        )));
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entry_line_anchors_preserve_original_positions_and_compile() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let entry = root.join("main.typ");
        let source = concat!(
            "#let value = 1\r\n",
            "= 标题\r\n",
            "中文正文\r\n",
            "继续正文\r\n",
            "\r\n",
            "*强调* 开头\r\n",
            "\r\n",
            "// 注释\r\n",
            "末段\r\n",
            "\r\n",
            "#include \"body.typ\"\r\n",
            "#let reusable = [隐藏内容]\r\n",
            "\r\n",
            "保留标签 <L14>\r\n",
        );
        fs::write(&entry, source).unwrap();
        fs::write(root.join("body.typ"), "= Included\n\nIncluded paragraph").unwrap();
        let prepared = Prepared::new(&entry, &root, true).unwrap();
        for line in [2, 3, 6, 9] {
            assert!(prepared.text.contains(&format!("id: \"L{line}\"")));
        }
        assert_eq!(prepared.text.matches("source-line-anchor").count(), 4);
        assert!(
            prepared.files.is_empty(),
            "included source must stay unchanged"
        );
        assert!(prepared.text.contains("#include \"body.typ\""));
        assert_eq!(Prepared::new(&entry, &root, false).unwrap().text, source);
        let output = root.join("out.html");
        fs::write(root.join("prepared.typ"), &prepared.text).unwrap();
        let result = match std::process::Command::new("typst")
            .args(["compile", "--features", "html", "--root"])
            .arg(&root)
            .arg(root.join("prepared.typ"))
            .arg(&output)
            .output()
        {
            Ok(result) => result,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Release builders need only Rust. The preprocessing assertions
                // above still run; only the external HTML compilation is skipped.
                eprintln!("skipping HTML compilation: typst is not installed");
                return;
            }
            Err(error) => panic!("could not run typst for HTML anchor check: {error}"),
        };
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let html = fs::read_to_string(output).unwrap();
        for line in [2, 3, 6, 9] {
            assert!(html.contains(&format!("id=\"L{line}\"")));
        }
        assert_eq!(html.matches("source-line-anchor").count(), 4);
    }

    #[test]
    fn anchors_skip_raw_math_and_reusable_content() {
        let source = Source::detached(
            "#let body = [hidden]\n\n```typ\n= raw heading\n\nraw text\n```\n\n$ x $\n\n正文",
        );
        let edits = line_anchors(&source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].1.contains("id: \"L11\""));
    }

    #[test]
    fn cetz_imports_render_in_html_without_changing_pdf_sources() {
        if std::process::Command::new("typst")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        let dir = tempfile::tempdir_in(&root).unwrap();
        let entry = dir.path().join("note.typ");
        let imports = [
            (
                "#import \"@preview/cetz:0.5.2\"",
                "cetz.canvas",
                "cetz.draw",
            ),
            (
                "#import \"@preview/cetz:0.5.2\" as ct",
                "ct.canvas",
                "ct.draw",
            ),
            (
                "#import \"@preview/cetz:0.5.2\": canvas as paint, draw",
                "paint",
                "draw",
            ),
            ("#import \"@preview/cetz:0.5.2\": *", "canvas", "draw"),
        ];
        for (import, canvas, draw) in imports {
            let source = format!(
                "#import \"/lib.typ\": template\n#show: template\n{import}\n#let render = {canvas}\n#let drawing = render({{ import {draw}: *; circle((0, 0), radius: 1, fill: rgb(\"#12ab34\")); content((0, 0), [$x$]) }})\n#figure(drawing, caption: [CeTZ test])\n#html.frame(drawing)\n"
            );
            fs::write(&entry, &source).unwrap();
            for html in [true, false] {
                // The explicit frame is HTML-only; keep the PDF fixture native.
                let input = if html {
                    source.clone()
                } else {
                    source.replace("#html.frame(drawing)", "")
                };
                fs::write(&entry, &input).unwrap();
                if html {
                    fs::write(dir.path().join("body.typ"), &input).unwrap();
                    fs::write(&entry, "#include \"body.typ\"").unwrap();
                }
                let prepared = Prepared::new(&entry, &root, html).unwrap();
                if !html {
                    assert_eq!(prepared.text, input);
                }
                let compiled = dir.path().join("prepared.typ");
                fs::write(&compiled, &prepared.text).unwrap();
                let output = dir.path().join(if html { "out.html" } else { "out.pdf" });
                let mut command = std::process::Command::new("typst");
                command.arg("compile").arg("--root").arg(&root);
                if html {
                    command.args(["--features", "html"]);
                }
                let result = command.arg(&compiled).arg(&output).output().unwrap();
                let diagnostics = String::from_utf8_lossy(&result.stderr);
                assert!(result.status.success(), "{import}: {diagnostics}");
                assert!(!diagnostics.contains("curve was ignored"), "{diagnostics}");
                assert!(!diagnostics.contains("move was ignored"), "{diagnostics}");
                if html {
                    let document = fs::read_to_string(&output).unwrap();
                    assert_eq!(document.matches("fill=\"#12ab34\"").count(), 2, "{import}");
                } else {
                    assert!(fs::metadata(output).unwrap().len() > 0);
                }
            }
        }
    }
    #[test]
    fn preserves_native_raw_and_relative_assets_in_local_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir(root.join("chapter")).unwrap();
        fs::write(
            root.join("main.typ"),
            "#include \"chapter/body.typ\"\n#import \"chapter/body.typ\" as body",
        )
        .unwrap();
        fs::write(
            root.join("chapter/body.typ"),
            "```js\nconst x = 1;\n```\n#image(\"picture.svg\")",
        )
        .unwrap();
        let prepared = Prepared::new(&root.join("main.typ"), &root, true).unwrap();
        assert!(prepared.files.is_empty());
        assert_eq!(
            prepared.text,
            fs::read_to_string(root.join("main.typ")).unwrap()
        );
        assert!(
            fs::read_to_string(root.join("chapter/body.typ"))
                .unwrap()
                .starts_with("```js")
        );
    }
    #[test]
    fn rejects_cycles_and_skips_packages_and_dynamic_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let entry = root.join("main.typ");
        fs::write(&entry, "#include \"main.typ\"").unwrap();
        assert!(
            Prepared::new(&entry, &root, false)
                .err()
                .unwrap()
                .contains("cyclic")
        );
        let source = "#import \"@preview/example:0.1.0\": *\n#include chapter";
        fs::write(&entry, source).unwrap();
        assert_eq!(Prepared::new(&entry, &root, false).unwrap().text, source);
    }
}
