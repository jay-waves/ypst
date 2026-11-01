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
    graphs: Vec<tempfile::TempDir>,
    copies: BTreeMap<PathBuf, PathBuf>,
    active: BTreeSet<PathBuf>,
}
impl Prepared {
    pub fn new(entry: &Path, root: &Path, html: bool) -> Result<Self, String> {
        let mut result = Self::default();
        result.text = result.process(entry, root, html)?;
        Ok(result)
    }
    fn process(&mut self, path: &Path, root: &Path, html: bool) -> Result<String, String> {
        if !self.active.insert(path.to_owned()) {
            return Err(format!("cyclic include/import: {}", path.display()));
        }
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let parsed = Source::detached(source.clone());
        let mut references = Vec::new();
        fn visit(node: &LinkedNode<'_>, out: &mut Vec<(std::ops::Range<usize>, String)>) {
            let expr = node
                .get()
                .cast::<ast::ModuleInclude>()
                .map(|v| v.source())
                .or_else(|| node.get().cast::<ast::ModuleImport>().map(|v| v.source()));
            if let Some(ast::Expr::Str(value)) = expr
                && !value.get().starts_with('@')
                && let Some(target) = node.find(value.span())
            {
                out.push((target.range(), value.get().to_string()));
            }
            for child in node.children() {
                visit(&child, out);
            }
        }
        visit(&LinkedNode::new(parsed.root()), &mut references);
        let mut edits = Vec::new();
        for (range, name) in references {
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
                let text = self.process(&target, root, html)?;
                let original = fs::read_to_string(&target).map_err(|e| e.to_string())?;
                let copy = if text == original {
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
                edits.push((
                    range,
                    serde_json::to_string(&format!("/{relative}")).map_err(|e| e.to_string())?,
                ));
            }
        }
        let mut text = source;
        for (range, value) in edits.into_iter().rev() {
            text.replace_range(range, &value);
        }
        let (text, graphs) = super::graphs::preprocess(&text, path, root)?;
        if let Some(graphs) = graphs {
            self.graphs.push(graphs);
        }
        let text = if html {
            super::graphs::highlight_code(&text)?
        } else {
            text
        };
        self.active.remove(path);
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recursively_rewrites_local_files_and_preserves_relative_assets() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
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
                assert_eq!(prepared.files.len(), 1);
                let file = &prepared.files[0];
                assert_eq!(file.path().parent(), Some(root.join("chapter").as_path()));
                let text = fs::read_to_string(file.path()).unwrap();
                assert!(text.contains("hljs-keyword"));
                assert!(text.contains("#image(\"picture.svg\")"));
                assert_eq!(prepared.text.matches(".ypst-source-").count(), 2);
                assert!(
                    fs::read_to_string(root.join("chapter/body.typ"))
                        .unwrap()
                        .starts_with("```js")
                );
            })
            .unwrap()
            .join()
            .unwrap();
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
