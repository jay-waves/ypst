//! Share self-contained Typst equation glyphs across SVG frames.
// TODO: Remove this postprocessing when Typst shares SVG definitions natively.
use regex::{Captures, Regex};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
    sync::OnceLock,
};

fn regex(pattern: &'static str) -> &'static Regex {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<&'static str, &'static Regex>>> =
        OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(pattern)
        .or_insert_with(|| Box::leak(Box::new(Regex::new(pattern).unwrap())))
}

fn valid_svg(svg: &str) -> bool {
    let mut reader = quick_xml::Reader::from_str(svg);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => return true,
            Err(_) => return false,
            _ => {}
        }
    }
}

fn optimize(html: &str) -> String {
    if html.contains("data-equation-glyphs=\"\"") {
        return html.to_owned();
    }
    let equation = regex(r"(?s)<svg\b.*?</svg>");
    let defs = regex(r"(?s)<defs>(.*?)</defs>");
    let symbol = regex(r#"(?s)<symbol\b[^>]*\bid="(g[0-9A-Fa-f]{32})"[^>]*>.*?</symbol>"#);
    let ids = regex(r#"\bid="([^"]+)"#);
    let refs = regex(r##"((?:xlink:)?href)="#([^"]+)""##);
    let mut used: HashSet<String> = ids.captures_iter(html).map(|c| c[1].to_owned()).collect();
    let mut symbols: HashMap<String, String> = HashMap::new();
    let mut pooled = Vec::new();
    let mut serial = 0usize;
    let has_theme = html.contains("class=\"typst-document\"");
    let result = equation.replace_all(html, |eq: &Captures<'_>| {
        let svg = &eq[0];
        let is_typst_frame = svg.contains(" xmlns:h5=\"http://www.w3.org/1999/xhtml\"");
        let is_display =
            html[..eq.get(0).unwrap().start()].ends_with("<div class=\"display-equation\">");
        if (!is_typst_frame && !is_display) || !symbol.is_match(svg) || !valid_svg(svg) {
            return eq[0].to_owned();
        }
        let mut names = HashMap::new();
        let svg = defs.replace_all(svg, |block: &Captures<'_>| {
            let remaining = symbol.replace_all(&block[1], |item: &Captures<'_>| {
                let original = &item[0];
                if original.contains("href=")
                    || original.contains("url(")
                    || original.contains("<style")
                {
                    return original.to_owned();
                }
                let id = format!(" id=\"{}\"", &item[1]);
                let key = original.replacen(&id, "", 1);
                let name = symbols.entry(key).or_insert_with(|| {
                    let name = loop {
                        let name = format!("eqg{serial:x}");
                        serial += 1;
                        if used.insert(name.clone()) {
                            break name;
                        }
                    };
                    pooled.push(original.replacen(&id, &format!(" id=\"{name}\""), 1));
                    name
                });
                names.insert(item[1].to_owned(), name.clone());
                String::new()
            });
            if remaining.trim().is_empty() {
                String::new()
            } else {
                format!("<defs>{remaining}</defs>")
            }
        });
        let svg = refs.replace_all(&svg, |reference: &Captures<'_>| {
            match names.get(&reference[2]) {
                Some(name) => format!("{}=\"#{name}\"", &reference[1]),
                None => reference[0].to_owned(),
            }
        });
        // Only compact the exact frame style emitted by Typst; leave custom styles alone.
        let mut svg = svg.into_owned();
        if let Some(end) = svg.find('>') {
            let head = &svg[..end];
            if has_theme
                && head.contains("style=\"overflow: visible; width=")
                && head.contains("; display: block\"")
                && !head.contains(" class=")
            {
                let compact = head
                    .replacen("<svg ", "<svg class=\"ypst-math-svg\" ", 1)
                    .replacen("overflow: visible; ", "", 1)
                    .replacen("; display: block\"", "\"", 1);
                svg = format!("{compact}{}", &svg[end..]);
            }
        }
        if !svg.contains("h5:") {
            svg = svg.replace(" xmlns:h5=\"http://www.w3.org/1999/xhtml\"", "");
        }
        svg
    });
    if pooled.is_empty() {
        return html.to_owned();
    }
    let Some(body) = regex(r"<body\b[^>]*>").find(&result) else {
        return html.to_owned();
    };
    let pool = format!(
        "<svg data-equation-glyphs=\"\" xmlns=\"http://www.w3.org/2000/svg\" aria-hidden=\"true\" width=\"0\" height=\"0\" style=\"position:absolute;overflow:hidden\"><defs>{}</defs></svg>",
        pooled.join("")
    );
    let output = format!("{}{pool}{}", &result[..body.end()], &result[body.end()..]);
    if output.len() < html.len() {
        output
    } else {
        html.to_owned()
    }
}

pub fn optimize_file(path: &Path) -> Result<(), String> {
    let html = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let output = optimize(&html);
    if output != html {
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| e.to_string())?;
        temp.write_all(output.as_bytes())
            .map_err(|e| e.to_string())?;
        temp.persist(path)
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pools_identical_glyphs_without_touching_other_graphics() {
        let glyph = format!("g{}", "A".repeat(32));
        let svg = format!(
            "<svg><use href=\"#{glyph}\"/><defs><symbol id=\"{glyph}\"><path d=\"{}\"/></symbol></defs></svg>",
            "M123 456".repeat(30)
        );
        let html = format!(
            "<body><span id=\"eqg0\"></span><div class=\"display-equation\">{svg}</div><div class=\"display-equation\">{svg}</div>{svg}</body>"
        );
        let result = optimize(&html);
        assert_eq!(result.matches("id=\"eqg1\"").count(), 1);
        assert_eq!(result.matches("href=\"#eqg1\"").count(), 2);
        assert!(result.contains(&svg));
        assert_eq!(optimize(&result), result);
    }
    #[test]
    fn leaves_invalid_or_external_definitions_untouched() {
        let id = format!("g{}", "B".repeat(32));
        let html = format!(
            "<body><div class=\"display-equation\"><svg><defs><symbol id=\"{id}\"><path fill=\"url(#paint)\"/></symbol></defs></svg></div><div class=\"display-equation\"><svg><path></svg></div></body>"
        );
        assert_eq!(optimize(&html), html);
    }

    #[test]
    fn shares_inline_and_display_frames_and_removes_unused_namespace() {
        let id = format!("g{}", "C".repeat(32));
        let svg = format!(
            "<svg xmlns:h5=\"http://www.w3.org/1999/xhtml\"><use href=\"#{id}\"/><defs><symbol id=\"{id}\"><path d=\"{}\"/></symbol></defs></svg>",
            "M123 456".repeat(30)
        );
        let html = format!("<body><p>{svg}</p><div class=\"display-equation\">{svg}</div></body>");
        let result = optimize(&html);
        assert_eq!(result.matches("<symbol ").count(), 1);
        assert_eq!(result.matches("href=\"#eqg0\"").count(), 2);
        assert!(!result.contains("xmlns:h5"));
    }
}
