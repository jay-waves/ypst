/// HTML-specific rendering helpers for the document theme.

/// Emit the note structure at compile time; details also works without scripts.
#let sidenote(body, aside) = {
    if aside == none {
        body
    } else {
        block(html.div(class: "sidenote", [
            #html.div(class: "sidenote-body", body)
            #html.elem("details", attrs: ("class": "sidenote-toggle"), [
                #html.elem("summary", "*")
                #html.elem("aside", attrs: (role: "note", class: "sidenote-panel"), [
                    #html.elem("a", attrs: (class: "sidenote-label",), "*")
                    #html.div(class: "note-content", aside)
                ])
            ])
        ]))
    }
}

/// Apply HTML-safe layout rules and emit browser resources.
#let template(theme: "light", body) = {
    // Heading and footnote formats are supplied by the public template.
    // HTML equations use native sequential numbering.
    set math.equation(numbering: "(1)")
    show heading: it => {
        show math.equation: math.bold
        it
    }

    // Typst's HTML exporter does not translate PDF layout rules. Keep the
    // semantic elements and let the browser styles arrange them.
    show align: it => {
        let horizontal = it.alignment.x
        let css-align = if horizontal == center {
            "center"
        } else if horizontal == right or horizontal == end {
            "right"
        } else {
            "left"
        }
        html.div(style: "text-align: " + css-align, it.body)
    }
    // Pad is also discarded by the exporter, including nested theorem text.
    show pad: it => it.body
    show grid.cell: it => it.body
    show grid: it => it.children.map(html.div).join()
    show stack: it => it.children.map(html.div).join()
    show place: it => it.body
    // During bundle document rendering the contextual target is HTML, so the
    // wrapper marks bundle mode with a private compiler input.
    let bundled = sys.inputs.at("ypst-template-bundle", default: "false") == "true"
    let assets = sys.inputs.at("ypst-template-assets-prefix", default: "assets/")
    if bundled {
        html.elem("link", attrs: (rel: "stylesheet", href: assets + "theme.css", "data-theme": theme))
        html.elem("link", attrs: (rel: "stylesheet", href: assets + "footnotes.css"))
        html.elem("link", attrs: (rel: "stylesheet", href: assets + "headings.css"))
    } else {
        html.elem("style", attrs: ("data-theme": theme), read("theme.css") + "\n" + read("footnotes.css") + "\n" + read("headings.css"))
    }
    block(html.div(class: "typst-document", body))
    if bundled {
        html.elem("script", attrs: (src: assets + "links.js"))
        html.elem("script", attrs: (src: assets + "footnotes.js"))
        html.elem("script", attrs: (src: assets + "headings.js"))
        html.elem("script", attrs: (src: assets + "line_navigation.js"))
    } else {
        html.elem("script", "(() => {\n" + read("links.js") + "\n" + read("footnotes.js") + "\n" + read("headings.js") + "\n" + read("line_navigation.js") + "\n})();")
    }
    context if query(raw.where(lang: "mermaid")).len() > 0 {
        if bundled {
            html.elem("link", attrs: (rel: "stylesheet", href: assets + "mermaid.css"))
            html.elem("script", attrs: (src: assets + "pele.global.js"))
            html.elem("script", attrs: (src: assets + "mermaid.js"))
        } else {
            html.elem("style", read("mermaid.css"))
            // HTML export rejects U+0085; preserve the JavaScript value with an escape.
            let pele = read("pele.global.js").replace("\u{85}", "\\u0085")
            html.elem("script", "/*\n" + read("LICENSE-Pele.txt").replace("*/", "* /") + "\n*/\n" + pele)
            html.elem("script", read("mermaid.js"))
        }
    }
}
