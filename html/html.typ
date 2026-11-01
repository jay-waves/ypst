/// HTML-specific rendering helpers for the document theme.

/// Emit the note structure at compile time; details also works without scripts.
#let sidenote(body, aside) = {
    if aside == none {
        body
    } else {
        block(html.div(class: "sidenote", [
            #html.div(class: "sidenote-body", body)
            #html.details(class: "sidenote-toggle", [
                #html.summary(html.sup("※"))
                #html.aside(role: "note", class: "sidenote-panel", [
                    #html.div(class: "note-content", aside)
                ])
            ])
        ]))
    }
}

/// Apply HTML-safe layout rules and emit browser resources.
#let template(theme: "light", body) = {
    // HTML follows the browser; the public theme parameter controls PDF only.
    let theme = "auto"
    // Keep display equations as SVG for consistent typography.
    show math.equation.where(block: true): it => context {
        set text(fill: rgb("#363733"))
        let formula = html.frame(math.equation(math.display(it.body), block: false, numbering: none))
        html.div(class: "display-equation", [
            #formula
            #if it.numbering != none {
                html.div(class: "equation-number", numbering(it.numbering, ..counter(math.equation).at(it.location())))
            }
        ])
    }
    show figure: it => {
        if type(it.kind) == str and it.kind in ("theorem", "lemma", "corollary", "definition", "proof") {
            it.body
        } else { it }
    }
    show heading: it => {
        show math.equation: math.bold
        it
    }

    // Typst's HTML exporter does not translate PDF layout rules. Keep the
    // semantic elements and let the browser styles arrange them.
    // Preserve text gaps in HTML; framed SVGs still need native spacing.
    show h: it => context {
        if target() == "html" { "\u{2002}" } else { it }
    }
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
    } else {
        html.elem("style", attrs: ("data-theme": theme), read("monaspace.css") + "\n" + read("theme.css") + "\n" + read("highlight.css") + "\n" + read("footnotes.css") + "\n" + read("headings.css"))
    }
    block(html.div(class: "typst-document", body))
    if bundled {
        html.script(src: assets + "theme.js")
    } else {
        html.script("(() => {\n" + read("links.js") + "\n" + read("footnotes.js") + "\n" + read("headings.js") + "\n" + read("line_navigation.js") + "\n})();")
    }
}
