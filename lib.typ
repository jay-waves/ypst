/// Shared document theme and typesetting helpers.
///
/// Common imports and usage (adjust the path relative to your document):
/// ```typst
/// #import "../appx/theme.typ": template, sidenote, theorem, lemma, corollary,
///   definition, proof
/// #set document(title: "Notes", keywords: ("robotics",))
/// #show: template.with(theme: "light", layout: "portrait")
/// ```
///
/// Feature index:
/// - `template(theme: "light", layout: "portrait", body)`: page layout and styling.
/// - `front-matter(body)`: introductory content with unnumbered headings.
/// - `sidenote(body, aside, side-image: none)`: body with a right-hand sidenote and optional image.
/// - `footnote(body)`: a native PDF footnote and an inline HTML disclosure.
/// - `theorem` / `lemma` / `corollary`: theorem, lemma, and corollary environments.
/// - `definition` / `proof`: definition and proof environments.
///
/// HTML export: `typst2 compile path/to/note.typ note.html` from the checkout.
/// The recommended interfaces are listed above; other top-level names remain importable.

#import "@preview/headcount:0.1.1": dependent-numbering, reset-counter
#import "html/html.typ" as _html

#assert(
    sys.version >= version(0, 15, 0),
    message: "This theme requires Typst 0.15.0 or newer.",
)

#let _code_fonts = (
    "Iosevka",
    "Source Han Sans",
)

// Missing glyphs fall back to other fonts registered with the compiler.
#let _math_fonts = ("New Computer Modern Math",)
#let _serif_fonts = ("Georgia", "Libertinus Serif", "New Computer Modern")
#let _cjk_serif_fonts = ("Source Han Serif",)
#let _cjk_sans_fonts = ("Source Han Sans",)

// Each role owns both script paths; weights are (regular, strong).
#let _typography = (
    body: (
        latin: (font: _serif_fonts + _cjk_serif_fonts, weights: (400, 700)),
        cjk: (font: _cjk_serif_fonts, weights: (500, 800)),
    ),
    heading: (
        latin: (font: _cjk_sans_fonts, weights: (600, 800)),
        cjk: (font: _cjk_sans_fonts, weights: (600, 800)),
    ),
    code: (
        latin: (font: _code_fonts, weights: (400, 700)),
        cjk: (font: _cjk_sans_fonts, weights: (400, 700)),
    ),
    table: (
        latin: (font: _serif_fonts + _cjk_serif_fonts, weights: (450, 700)),
        cjk: (font: _cjk_serif_fonts, weights: (500, 800)),
    ),
)

// Chinese typography, including full-width punctuation. This is deliberately
// not a claim to cover Japanese kana or Korean hangul.
// Single curly quotes also serve as English apostrophes; keep them in the Latin font.
#let _cjk_text = regex("[\\p{Han}、，。！？；：“”《》「」『』（）【】〔〕〈〉…—]")

#let _script_text(profile, bold, body) = text(
    font: profile.font,
    weight: profile.weights.at(int(bold)),
    body,
)

#let _font_size = (
  tiny: 6.5pt,
  code: 6.75pt,
  body: 8pt,
  // Match the HTML heading scale: 1.4x / 1.2x / 1.1x of body text.
  h1: 11.2pt,
  h2: 9.6pt,
  h3: 8.8pt,
)

#let _palette(theme) = {
    assert(
        theme in ("light", "dark"),
        message: "The theme must be either \"light\" or \"dark\".",
    )
    let dark = theme == "dark"
    (
        fg: if dark { rgb("#c9cdd2") } else { rgb("#26282b") },
        bg: if dark { rgb("#1e1e1e") } else { white },
        muted: if dark { rgb("#a0a5ad") } else { rgb("#62666b") },
        border: if dark { rgb("#50555d") } else { rgb("#cbd0d5") },
        border-muted: if dark { rgb("#50555db3") } else { rgb("#cbd0d5b3") },
        accent: if dark { rgb("#dde0e4") } else { rgb("#202326") },
        accent-2: if dark { rgb("#bfc5cd") } else { rgb("#4b5157") },
        code-fg: if dark { rgb("#c3c9d1") } else { rgb("#33373b") },
        pre-bg: if dark { rgb("#2d3035") } else { rgb("#f6f7f8") },
        code-bg: if dark { rgb("#2d3035cc") } else { rgb("#f6f7f8cc") },
    )
}

// Components are evaluated contextually so one template configuration also
// controls diagrams, sidenotes, and theorem environments.
#let _theme = state("ypst-template.theme", "light")

// Keep opening punctuation with the following Han glyph and closing
// punctuation with the preceding one. Each match remains small enough for
// normal CJK line breaking while avoiding punctuation stranded at line edges.
#let _cjk_emph_chunk = regex(
    "[‘“《「『（【〔〈]*\\p{Han}[、，。！？；：’”）》」』】〕〉…—]*"
)

// Semantic modifiers re-enter the same role, preserving bold + emphasis in
// either nesting order. Only CJK emphasis needs a geometric transformation.
#let _typeset(role, bold: false, italic: false, heading-cjk-size: 0.94em, body) = {
    let profile = _typography.at(role)
    set text(style: ("normal", "italic").at(int(italic)))
    show strong: it => _typeset(role, bold: true, italic: italic, heading-cjk-size: heading-cjk-size, it.body)
    show emph: it => _typeset(role, bold: bold, italic: true, heading-cjk-size: heading-cjk-size, it.body)
    show _cjk_text: set text(
        font: profile.cjk.font,
        weight: profile.cjk.weights.at(int(bold)),
        size: if role == "heading" { heading-cjk-size } else { 1em },
    )
    if italic {
        show _cjk_emph_chunk: chunk => {
            set text(style: "normal")
            box(skew(ax: -12deg, reflow: false,
                _script_text(profile.cjk, bold, chunk)))
        }
        _script_text(profile.latin, bold, body)
    } else {
        _script_text(profile.latin, bold, body)
    }
}

#let _sidenote_content(content) = {
    // Unnumbered equations do not advance the equation counter. Avoid restoring
    // a contextual snapshot, which chains layout dependencies across sidenotes.
    show math.equation: set math.equation(numbering: none)
    content
}

/// Body with a right-hand sidenote; `side-image` accepts a string, `path`, or image content.
#let sidenote(body, aside, side-image: none) = context {
    let palette = _palette(_theme.get())
    let aside-text = if aside == [] {
        none
    } else {
        block(
            width: 100%,
            text(size: _font_size.tiny, aside),
        )
    }
    let aside-content = if side-image == none {
        aside-text
    } else {
        let pinned-image = if type(side-image) in (str, path) {
            image(side-image, width: 100%)
        } else {
            block(width: 100%, {
                show image: set image(width: 100%)
                side-image
            })
        }

        if aside-text == none {
            block(width: 100%, pinned-image)
        } else {
            stack(
                dir: ttb,
                spacing: 0.6em,
                block(width: 100%, pinned-image),
                aside-text,
            )
        }
    }

    let rendered-aside = _sidenote_content(aside-content)
    if target() == "html" {
        _html.sidenote(body, rendered-aside)
    } else {
        block(
            width: 100%,
            breakable: true,
            grid(
                columns: (3fr, 1fr), // body-ratio : aside-ratio
                column-gutter: 4%,
                align: top + left,
                block(width: 100%, body),
                block(
                    width: 100%,
                    inset: (left: 0.75em),
                    stroke: (left: 0.5pt + palette.border),
                    rendered-aside,
                ),
            ),
        )
    }
}

// Native figure counters provide labels/references; headcount adds the chapter.
#let _statement_kinds = ("theorem", "lemma", "corollary", "definition", "proof")
#let _statement_numbering = dependent-numbering("1.1")
#let _figure_numbering = dependent-numbering("1.1a")
#let _equation_numbering = dependent-numbering("(1.1)")

// A small renderer shared by all statement environments. Each kind has its
// own native counter. Optional positional names retain theorem([Name])[...].
#let _statement(kind, title, supplement, boxed, ..args, body,
    numbering: _statement_numbering, name: none) = {
    let names = args.pos()
    assert(names.len() <= 1, message: "Expected at most one statement name.")
    let name = if names.len() > 0 { names.first() } else { name }
    let rendered = context {
        let palette = _palette(_theme.get())
        let content = [
            #strong[
                #title#if numbering != none [~#context counter(figure.where(kind: kind)).display(numbering)]
                #if name != none [ (#name)]
            ]: #body#if kind == "proof" [ #if target() == "html" { html.span(class: "proof-qed", [□]) } else { h(1fr); $square$ }]
        ]
        if target() == "html" {
            html.div(class: "theorem-environment" + if boxed { " theorem-box" } else { "" }, content)
        } else {
            block.with(
                width: 100%, breakable: true,
                inset: if boxed { (x: 1em, y: .5em) } else { 0pt },
                fill: if boxed { palette.pre-bg } else { none },
                stroke: if boxed { .4pt + palette.border } else { none },
                radius: if boxed { 3pt } else { 0pt },
            )(..args.named(), content)
        }
    }
    figure(rendered, kind: kind, supplement: supplement,
        numbering: numbering, outlined: false)
}

/// Statements have independent chapter-based counters and native references.
#let theorem = _statement.with("theorem", "Theorem", [Thm.], true)
#let lemma = _statement.with("lemma", "Lemma", [Lemma], true)
#let corollary = _statement.with("corollary", "Corollary", [Cor.], false)
#let definition = _statement.with("definition", "Definition", [Def.], true)
/// Unnumbered proof with an automatic end symbol.
#let proof = _statement.with("proof", "Proof", [Proof], false, numbering: none)

// Only numbered headings reset chapter-dependent counters. Apply this after
// each target's heading design so it does not get discarded by that design.
#let _reset_numbered(keys, body) = {
    if keys.len() == 0 { body } else {
        show heading: it => {
            if it.numbering == none { it } else { reset-counter(counter(keys.first()))(it) }
        }
        _reset_numbered(keys.slice(1), body)
    }
}
#let _chapter_counters(body) = _reset_numbered(
    (math.equation, figure.where(kind: image), figure.where(kind: table))
        + _statement_kinds.map(kind => figure.where(kind: kind)), body,
)

// Native refs retain parentheses returned by a numbering function. Equations
// display them, but references use the plain number at the target location.
#let _equation_ref(it) = context {
    let equation = it.element
    if it.form != "normal" or equation == none or equation.func() != math.equation or equation.numbering != _equation_numbering {
        return it
    }
    let loc = equation.location()
    let supplement = if it.supplement == auto { equation.supplement } else { it.supplement }
    if type(supplement) == function { supplement = supplement(equation) }
    link(loc, [#if supplement not in (none, []) [#supplement~]#numbering(
        "1.1", counter(heading).at(loc).first(), ..counter(math.equation).at(loc),
    )])
}

// Paged output is the core renderer. HTML-specific compatibility rules live in
// html/html.typ, which embeds HTML assets at compilation.
#let _paged_template(theme, layout, body) = context {
    let palette = _palette(theme)
    let landscape = layout == "landscape"
    let page-columns = if landscape { 2 } else { 1 }

    set heading(supplement: [Sec.])
    show math.equation: set text(font: _math_fonts)

    set page(
        paper: "a5",
        flipped: landscape,
        columns: page-columns,
        fill: palette.bg,
        margin: (y: 2.25em, x: 1.8em),
        header: counter(footnote).update(0),
        foreground: if landscape {
            place(
                center + horizon,
                rect(
                    width: 0.55pt,
                    height: 100% - 6em,
                    fill: palette.border-muted,
                ),
            )
        },
    )
    set columns(gutter: 4%)

    show link: set text(fill: palette.accent-2)
    show link: underline

    // Set the body fallback without styling heading contents before their own show rule.
    set text(font: _typography.body.latin.font)

    let paragraph-leading = 0.8em
    // Match html/theme.css's 1rem block margin more closely at the compact body
    // size used by the paged template.
    let block-spacing = 1.4em
    set par(
        leading: paragraph-leading,
        spacing: block-spacing,
    )
    show heading: it => context {
        let level = calc.min(it.level, 3)
        let size = if it.level >= 4 {
            _font_size.body
        } else { (
            _font_size.h1,
            _font_size.h2,
            _font_size.h3,
        ).at(level - 1) }

        show: _typeset.with("heading", heading-cjk-size: size * 94%)
        // Use mathematical bold glyphs while preserving italic variables.
        show math.equation: math.bold

        set text(
            size: size,
            fill: palette.accent,
        )

        v((24pt, 18pt, 16pt).at(level - 1), weak: true)

        if it.numbering != none and it.level <= 3 {
            counter(heading).display(it.numbering)
            h(7pt, weak: true)
        }

        it.body

        if level <= 2 {
            v(5pt, weak: true)
            line(length: 100%, stroke: 0.35pt + palette.border-muted)
        }

        v(12pt, weak: true)
    }

    set text(
        fill: palette.fg,
        size: _font_size.body,
        number-type: "old-style",
        number-width: "tabular",
    )
    show figure: fig => {
        show figure.caption: caption => context [
            *#caption.supplement~#numbering(
                caption.numbering,
                ..caption.counter.at(fig.location()),
            )#h(1em)*#caption.body
        ]
        block(
            above: block-spacing,
            below: block-spacing,
            fig,
        )
    }
    show figure.caption: set text(size: _font_size.tiny)

    show figure: it => {
        if it.kind in _statement_kinds { align(start, it.body) } else { it }
    }

    // code block
    show raw.where(block: true): it => {
        show: _typeset.with("code")
        let language = it.lang
        set text(
            size: _font_size.code,
            fill: palette.code-fg,
            // Raw lines use a baseline bottom edge; reserve 0.8em above it.
            top-edge: 0.8em,
            bottom-edge: 0pt,
        )
        // Use 1.7x code line spacing; keep outer spacing tied to surrounding prose.
        set par(leading: 0.9em, spacing: 0.6em)
        let code-spacing = _font_size.body

        block(
            width: 100%,
            above: code-spacing,
            below: code-spacing,
            inset: (x: 1em, y: 0.9em),
            radius: 2pt,
            fill: palette.code-bg,
            [
                #if language != none {
                    place(
                        top + right,
                        dx: -1em,
                        // Keep the label on the first code-line baseline.
                        dy: 0.05em,
                        text(
                            size: _font_size.code,
                            weight: 500,
                            fill: palette.muted,
                        )[ #language ],
                    )
                }
                // Raw blocks preserve long source lines. Do not wrap, scale,
                // or otherwise reshape them; Typst can report width overflow.
                #it
            ],
        )
    }

    // inline code
    show raw.where(block: false): it => _typeset("code", box(
        inset: (x: 0.42em, y: 0.18em),
        radius: 2.5pt,
        fill: palette.code-bg,
        text(
            size: _font_size.code,
            fill: palette.code-fg,
            it,
        ),
    ))

    // Keep quotes visually quiet and separate from surrounding paragraphs.
    show quote: it => block(
      width: 100%,
      above: block-spacing,
      below: block-spacing,
      inset: (left: 0.9em, right: 0pt, top: 0.2em, bottom: 0.2em),
      stroke: (left: 1pt + palette.border-muted),
    )[
        #text(fill: palette.muted)[
            #it.body

            #if it.attribution != none [
                #v(0.45em)
                #align(
                    right,
                    text(
                        size: _font_size.tiny,
                        fill: palette.muted,
                    )[
                        — #it.attribution
                    ],
                )
            ]
        ]
    ]

    set table(
        // Match the Markdown table rhythm: generous horizontal padding and
        // horizontal rules only, with no vertical grid lines.
        inset: (x: 1.25em, y: 0.7em),
        fill: (x, y) => if y > 0 and calc.odd(y) {
            palette.pre-bg
        } else {
            palette.bg
        },
        stroke: (x: none, y: 0.35pt + palette.border-muted),
    )

    show table.cell: it => {
        show: _typeset.with("table", bold: it.y == 0)
        set par(spacing: 0.7em)
        // CSS uses the document background for odd rows and the preformatted
        // background for every second row. Keep the same alternating rhythm
        // in paged Typst output, including repeated table headers.
        set text(
            size: 0.875em,
            number-width: "tabular",
            fill: if it.y == 0 { palette.accent } else { palette.fg },
        )
        it
    }

    // Footnotes restart on each page: superscript [a]; entries: [a] content.
    show footnote.entry: it => context {
        let note = it.note
        block[
            #link(note.location(), counter(footnote).display(note.numbering, at: note.location()))#h(0.3em)#note.body
        ]
    }

    _chapter_counters(body)
}

/// Introductory content with unnumbered headings; styles and outline entries
/// are preserved, and heading numbering resumes after this scope.
#let front-matter(body) = {
    set heading(numbering: none)
    body
}

/// Apply the document template. Use with a show rule, optionally pre-filling
/// configuration: `#show: template.with(theme: "dark", layout: "landscape")`.
#let template(
    theme: sys.inputs.at("theme", default: "light"),
    layout: "portrait",
    body,
) = context {
    // Validate at the public boundary so all output targets report the same
    // configuration errors.
    let _ = _palette(theme)
    assert(
        layout in ("portrait", "landscape"),
        message: "The layout must be either \"portrait\" or \"landscape\".",
    )

    set heading(numbering: (..numbers) => {
        if numbers.pos().len() <= 3 {
            numbering("1.1", ..numbers)
        }
    })
    set footnote(numbering: "[a]")
    set math.equation(numbering: _equation_numbering, supplement: [Eq.])
    show figure.where(kind: image): set figure(supplement: [Fig.], numbering: _figure_numbering)
    show figure.where(kind: table): set figure(supplement: [Tab.], numbering: _figure_numbering)

    show ref: _equation_ref

    _theme.update(theme)
    if target() == "html" {
        _html.template(theme: theme, _chapter_counters(body))
    } else {
        _paged_template(theme, layout, body)
    }
}
