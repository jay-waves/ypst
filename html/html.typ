/// HTML-specific rendering helpers for the document theme.

/// Keep the body visible and reveal its note with a native disclosure.
#let sidenote(body, aside) = {
    if aside == none {
        body
    } else {
        block(html.div(class: "sidenote", [
            #body
            #html.elem("details", attrs: ("class": "sidenote-toggle"), [
                #html.elem("summary", "*")
                #html.elem("aside", attrs: (role: "note"), [
                    #html.elem("span", attrs: (class: "sidenote-label", "aria-hidden": "true"), "*")
                    #aside
                ])
            ])
        ]))
    }
}

/// Apply the HTML-safe layout rules and inject the package assets.
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
    html.elem("style", attrs: ("data-theme": theme), read("theme.css"))
    block(html.div(class: "typst-document", body))
    html.elem("script", "(() => {\n" + read("links.js") + "\n" + read("footnotes.js") + "\n" + read("headings.js") + "\n})();")
}
