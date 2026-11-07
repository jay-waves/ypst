use std::sync::LazyLock;
use syntect::{
    easy::ScopeRegionIterator,
    parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use typst_syntax::{LinkedNode, Source, ast};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(|| {
    let mut builder = two_face::syntax::extra_newlines().into_builder();
    for source in [
        include_str!("../syntaxes/HTTP.sublime-syntax"),
        include_str!("../syntaxes/Excel.sublime-syntax"),
        include_str!("../syntaxes/PowerShell.sublime-syntax"),
        include_str!("../syntaxes/VisualBasic.sublime-syntax"),
        include_str!("../syntaxes/WebAssembly.sublime-syntax"),
    ] {
        builder.add(
            SyntaxDefinition::load_from_str(source, true, None)
                .expect("bundled syntax must be valid"),
        );
    }
    builder.build()
});

fn syntax(name: &str) -> Option<&'static SyntaxReference> {
    let name = name.rsplit('.').next().unwrap_or(name).to_ascii_lowercase();
    let name = match name.as_str() {
        "typ" => "typst",
        "md" | "mkdown" | "mkd" => "markdown",
        "bib" => "bibtex",
        "pwsh" | "ps" | "ps1" => "powershell",
        "tex" => "latex",
        "docker" => "dockerfile",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "jsx" => "tsx",
        "jsonc" | "json5" => "json",
        "sh" | "zsh" | "shell" | "console" | "shellsession" => "bash",
        "cs" | "csharp" => "c#",
        "cpp" | "cc" | "cxx" | "h++" | "hpp" | "hh" | "hxx" => "c++",
        "h" => "c",
        "py" | "gyp" | "ipython" | "python-repl" | "pycon" => "python",
        "rb" | "gemspec" | "podspec" | "thor" | "irb" => "ruby",
        "rs" => "rust",
        "kt" | "kts" | "ktm" | "ktx" => "kotlin",
        "https" => "http",
        "objectivec" | "mm" | "objc" | "obj-c" | "obj-c++" | "objective-c++" => "objective-c",
        "php-template" => "php",
        "nginxconf" => "nginx",
        "golang" => "go",
        "pluto" => "lua",
        "mk" | "mak" | "make" => "makefile",
        "pl" | "pm" => "perl",
        "xhtml" | "rss" | "atom" | "xjb" | "xsd" | "xsl" | "plist" | "wsf" | "svg" => "xml",
        "plaintext" | "text" | "txt" | "" => return None,
        other => other,
    };
    SYNTAXES.find_syntax_by_token(name)
}

// Map grammar scopes to a small shared palette; inner scopes take precedence.
fn class(scope: &str) -> Option<&'static str> {
    let mut parts = scope.split('.');
    let first = parts.next()?;
    let second = parts.next().unwrap_or("");
    Some(match (first, second) {
        ("comment", _) => "comment",
        ("string", _) => "string",
        ("keyword" | "storage", _) => "keyword",
        ("constant", _) | ("variable", _) => "constant",
        ("entity", "name") => "entity",
        ("entity", "other") => "constant",
        ("support", _) => "builtin",
        ("markup", "heading") => "heading",
        ("markup", "bold") => "strong",
        ("markup", "italic") => "emphasis",
        ("markup", "inserted") => "addition",
        ("markup", "deleted") => "deletion",
        ("markup", "raw") => "string",
        _ => return None,
    })
}

fn text(output: &mut String, value: &str, class: Option<&str>) {
    if value.is_empty() {
        return;
    }
    if let Some(class) = class {
        output.push_str(&format!(
            "#html.elem(\"span\", attrs: (class: \"syntax-{class}\",), ["
        ));
    }
    // Typst uses \u{...}, unlike JSON's \uXXXX escapes for control characters.
    output.push_str("#text(\"");
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push_str("\")");
    if class.is_some() {
        output.push_str("])");
    }
}

fn highlight(source: &str, syntax: &SyntaxReference, output: &mut String) -> Result<(), String> {
    let mut parser = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    for line in LinesWithEndings::from(source) {
        let ops = parser
            .parse_line(line, &SYNTAXES)
            .map_err(|e| e.to_string())?;
        // Merge adjacent regions with the same color to avoid redundant spans.
        let mut start = 0;
        let mut end = 0;
        let mut previous = None;
        for (region, op) in ScopeRegionIterator::new(&ops, line) {
            stack.apply(op).map_err(|e| e.to_string())?;
            if region.is_empty() {
                continue;
            }
            let current = stack
                .scopes
                .iter()
                .rev()
                .find_map(|scope| class(&scope.to_string()));
            if current != previous {
                text(output, &line[start..end], previous);
                start = end;
                previous = current;
            }
            end += region.len();
        }
        text(output, &line[start..end], previous);
    }
    Ok(())
}

fn block(source: &str, name: &str) -> Result<String, String> {
    let mut output =
        String::from("#html.elem(\"pre\", html.elem(\"code\", attrs: (class: \"syntax\",), [");
    if let Some(syntax) = syntax(name) {
        // HTTP bodies use JSON only when the Content-Type declares it.
        let body = if syntax.name == "HTTP" {
            source
                .find("\r\n\r\n")
                .map(|at| at + 4)
                .or_else(|| source.find("\n\n").map(|at| at + 2))
                .filter(|&end| {
                    source[..end].lines().any(|line| {
                        line.split_once(':').is_some_and(|(key, value)| {
                            let mime = value
                                .split(';')
                                .next()
                                .unwrap_or("")
                                .trim()
                                .to_ascii_lowercase();
                            key.eq_ignore_ascii_case("content-type")
                                && mime.starts_with("application/")
                                && (mime == "application/json" || mime.ends_with("+json"))
                        })
                    })
                })
        } else {
            None
        };
        if let Some(body) = body {
            highlight(&source[..body], syntax, &mut output)?;
            highlight(
                &source[body..],
                SYNTAXES.find_syntax_by_token("json").unwrap(),
                &mut output,
            )?;
        } else {
            highlight(source, syntax, &mut output)?;
        }
    } else {
        text(&mut output, source, None);
    }
    output.push_str("]))");
    Ok(output)
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
    let mut output = source.to_owned();
    for (range, text, language) in blocks.into_iter().rev() {
        let mut replacement = block(&text, &language)?;
        // Preserve source line numbers for compiler diagnostics.
        replacement.push_str(
            &"\n".repeat(
                source[range.clone()]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count(),
            ),
        );
        output.replace_range(range, &replacement);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_aliases() {
        for name in "bash c cpp csharp css diff go graphql ini java javascript json kotlin less lua
            makefile markdown objectivec perl php php-template python python-repl r ruby rust scss
            shell sql swift typescript vbnet wasm xml yaml pwsh excel http https nginx cmake tex docker bib typ
            patch gql jsx jsonc json5 tsx yml xlsx xls vb sh zsh h cc c++ h++ hpp hh hxx cxx cs c#
            md mkdown mkd rb gemspec podspec thor irb golang toml jsp js mjs cjs kt kts ktm ktx
            pluto mk mak make pl pm mm objc obj-c obj-c++ objective-c++ py gyp ipython pycon rs
            console shellsession ts mts cts ps ps1 nginxconf html xhtml rss atom xjb xsd xsl plist wsf svg"
            .split_whitespace() {
            assert!(syntax(name).is_some(), "missing syntax: {name}");
            block("let value = 42;\n", name).unwrap();
        }
        for (alias, canonical) in [
            ("example.JS", "javascript"),
            ("pwsh", "powershell"),
            ("bib", "bibtex"),
            ("tex", "latex"),
            ("typ", "typst"),
            ("docker", "dockerfile"),
        ] {
            assert_eq!(syntax(alias).unwrap().name, syntax(canonical).unwrap().name);
        }
    }

    #[test]
    fn preserves_source_and_only_rewrites_literal_block_raw() {
        let source = "```js\nconst x = \"<tag>\";\n```\n\n```unknown\na < b & c\n```\n`inline`\n#let code = ```js\nconst y = 2;\n```";
        let output = highlight_code(source).unwrap();
        assert!(output.contains("syntax-keyword"));
        assert!(output.contains("a < b & c"));
        assert!(output.contains("`inline`"));
        assert!(output.contains("#let code = ```js\nconst y = 2;\n```"));
        assert_eq!(source.lines().count(), output.lines().count());
        assert!(
            Source::detached(output)
                .root()
                .errors_and_warnings()
                .0
                .is_empty()
        );
        assert!(
            !block("let x = 1", "unknown")
                .unwrap()
                .contains("syntax-keyword")
        );
        assert!(!block("let x = 1", "").unwrap().contains("syntax-keyword"));
    }

    #[test]
    fn multiline_grammars_and_http_json() {
        let output = block("/* first\nsecond */\nconst s = `hello`;", "js").unwrap();
        assert!(output.contains("syntax-comment\",), [#text(\"second */\")"));
        for newline in ["\n", "\r\n"] {
            let http = format!(
                "HTTP/1.1 200 OK{newline}Content-Type: application/problem+json; charset=utf-8{newline}{newline}{{\"message\": true}}"
            );
            let output = block(&http, "http").unwrap();
            assert!(
                output.contains("syntax-constant\",), [#text(\"true\")"),
                "{output}"
            );
        }
        for (lang, source, expected) in [
            ("typ", "#let x = 1", "keyword"),
            ("bib", "@article{key, title={Title}}", "entity"),
            ("pwsh", "if ($true) { Write-Host \"Hello\" }", "keyword"),
            ("excel", "=SUM(A1:A4)", "builtin"),
            ("docker", "FROM alpine:latest", "keyword"),
            ("nginx", "server { listen 80; }", "keyword"),
            ("cmake", "set(NAME example)", "builtin"),
        ] {
            let output = block(source, lang).unwrap();
            assert!(
                output.contains(&format!("syntax-{expected}")),
                "{lang}: {output}"
            );
        }
    }

    #[test]
    fn typst_export_preserves_code_text() {
        use std::{fs, process::Command};
        if Command::new("typst").arg("--version").output().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let source = "const 文本 = \"<&>\\\"\";\n\t// #] $ @ \\u{41}\n\nend\n";
        let tags = regex::Regex::new("<[^>]*>").unwrap();
        for language in ["js", "unknown"] {
            let input = dir.path().join("test.typ");
            let output = dir.path().join("test.html");
            fs::write(&input, block(source, language).unwrap()).unwrap();
            let result = Command::new("typst")
                .args(["compile", "--features", "html"])
                .arg(&input)
                .arg(&output)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let html = fs::read_to_string(output).unwrap();
            let code = html
                .split("<code class=\"syntax\">")
                .nth(1)
                .unwrap()
                .split("</code>")
                .next()
                .unwrap();
            let stripped = tags.replace_all(code, "");
            let decoded = quick_xml::escape::unescape_with(
                &stripped,
                quick_xml::escape::resolve_html5_entity,
            )
            .unwrap();
            assert_eq!(decoded, source);
        }
        // Escaping must also produce valid Typst for non-JSON control escapes.
        assert!(
            Source::detached(block("a\u{000c}b", "unknown").unwrap())
                .root()
                .errors_and_warnings()
                .0
                .is_empty()
        );
    }
}
