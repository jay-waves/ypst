//! Externalize literal images after graph/download preprocessing, before Typst.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use typst_syntax::{
    LinkedNode, Source,
    ast::{self, AstNode},
};

pub struct SiteImages {
    pub site: PathBuf,
    pub page: PathBuf,
}

impl SiteImages {
    pub fn rewrite(&self, source: &str, input: &Path, root: &Path) -> Result<String, String> {
        let parsed = Source::detached(source);
        let linked = LinkedNode::new(parsed.root());
        let mut edits = Vec::new();
        fn visit(
            node: &LinkedNode<'_>,
            source: &str,
            input: &Path,
            root: &Path,
            config: &SiteImages,
            edits: &mut Vec<(std::ops::Range<usize>, String)>,
        ) -> Result<(), String> {
            if let Some(call) = node.get().cast::<ast::FuncCall>()
                && matches!(call.callee(), ast::Expr::Ident(name) if name.as_str() == "image")
            {
                let args: Vec<_> = call.args().items().collect();
                if let Some(ast::Arg::Pos(ast::Expr::Str(path))) = args.first() {
                    let mut attrs = Vec::new();
                    let mut supported = true;
                    for arg in args.iter().skip(1) {
                        if let ast::Arg::Named(named) = arg {
                            let name = named.name();
                            if matches!(name.as_str(), "width" | "height") {
                                let range = node
                                    .find(named.expr().span())
                                    .ok_or("missing image argument")?
                                    .range();
                                attrs.push(format!("{}: repr({})", name.as_str(), &source[range]));
                            } else if name.as_str() == "alt" {
                                let range = node
                                    .find(named.expr().span())
                                    .ok_or("missing image argument")?
                                    .range();
                                attrs.push(format!("alt: {}", &source[range]));
                            } else {
                                supported = false;
                            }
                        } else {
                            supported = false;
                        }
                    }
                    if supported {
                        let name = path.get();
                        let path = if name.starts_with('/') {
                            root.join(name.trim_start_matches('/'))
                        } else {
                            input
                                .parent()
                                .ok_or("source has no parent")?
                                .join(name.as_str())
                        };
                        let path = fs::canonicalize(&path)
                            .map_err(|e| format!("image {}: {e}", path.display()))?;
                        if !path.starts_with(root) {
                            return Err(format!("image {} is outside --root", path.display()));
                        }
                        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
                        let extension = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if matches!(
                            extension.as_str(),
                            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
                        ) {
                            let dir = config.site.join("_shared/typst/images");
                            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                            let target =
                                dir.join(format!("{:x}.{extension}", Sha256::digest(&bytes)));
                            if fs::read(&target).ok().as_deref() != Some(bytes.as_slice()) {
                                let mut temp = tempfile::NamedTempFile::new_in(&dir)
                                    .map_err(|e| e.to_string())?;
                                temp.write_all(&bytes).map_err(|e| e.to_string())?;
                                temp.persist(&target).map_err(|e| e.to_string())?;
                            }
                            let href = super::escape_relative_url(&super::relative_path(
                                &config.page,
                                &target,
                            )?);
                            attrs.insert(
                                0,
                                format!(
                                    "src: {}",
                                    serde_json::to_string(&href).map_err(|e| e.to_string())?
                                ),
                            );
                            // Dimensions belong in CSS: HTML width/height attributes do not accept pt/%.
                            let dimensions: Vec<_> = attrs
                                .iter()
                                .filter(|s| s.starts_with("width:") || s.starts_with("height:"))
                                .cloned()
                                .collect();
                            attrs.retain(|s| !s.starts_with("width:") && !s.starts_with("height:"));
                            if !dimensions.is_empty() {
                                let parts = dimensions
                                    .iter()
                                    .map(|s| {
                                        let (key, value) = s.split_once(": ").unwrap();
                                        format!(
                                            "{} + {value}",
                                            serde_json::to_string(&format!("{key}:")).unwrap()
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                attrs.push(format!("style: ({},).join(\";\")", parts.join(",")));
                            }
                            let mut replacement =
                                format!("html.elem(\"img\", attrs: ({},))", attrs.join(", "));
                            replacement.push_str(&"\n".repeat(
                                source[node.range()].bytes().filter(|b| *b == b'\n').count(),
                            ));
                            edits.push((node.range(), replacement));
                            return Ok(());
                        }
                    }
                }
            }
            for child in node.children() {
                visit(&child, source, input, root, config, edits)?;
            }
            Ok(())
        }
        visit(&linked, source, input, root, self, &mut edits)?;
        let mut output = source.to_owned();
        for (range, text) in edits.into_iter().rev() {
            output.replace_range(range, &text);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_images_externalize_once_and_keep_dimensions() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        let bytes = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>";
        fs::write(root.join("pic.svg"), bytes).unwrap();
        let site = SiteImages {
            site: root.join("site"),
            page: root.join("site/note.typ"),
        };
        let source = "#image(\"../pic.svg\", width: 75%, alt: \"Example\")\n#image(\"/pic.svg\")\n#image(path)\n#image(\"../pic.svg\", fit: \"contain\")\n";
        let result = site
            .rewrite(source, &root.join("nested/note.typ"), &root)
            .unwrap();
        assert!(
            Source::detached(result.clone())
                .root()
                .errors_and_warnings()
                .0
                .is_empty()
        );
        assert!(result.contains("html.elem(\"img\""));
        assert!(result.contains("repr(75%)"));
        assert!(result.contains("alt: \"Example\""));
        assert!(result.contains("../_shared/typst/images/"));
        assert!(result.contains("#image(path)"));
        assert!(result.contains("fit: \"contain\""));
        assert!(!result.contains("base64"));
        let files = fs::read_dir(site.site.join("_shared/typst/images"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(fs::read(files[0].path()).unwrap(), bytes);
    }

    #[test]
    fn rejects_outside_root() {
        let temp = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(temp.path()).unwrap();
        let root = base.join("root");
        fs::create_dir(&root).unwrap();
        fs::write(base.join("outside.svg"), "<svg/>").unwrap();
        let site = SiteImages {
            site: root.join("site"),
            page: root.join("site/note.typ"),
        };
        assert!(
            site.rewrite("#image(\"../outside.svg\")", &root.join("note.typ"), &root)
                .unwrap_err()
                .contains("outside --root")
        );
    }
}
