// Compile from the repository root with --root .; see README.md.
#import "../lib.typ": (
  template, front-matter, sidenote, definition, theorem,
  lemma, corollary, proof, diagram, node, edge,
)
#set document(title: "Template components / 模板用法")
#show: template.with(layout: sys.inputs.at("layout", default: "portrait"))

#front-matter[
  = Template components / 模板用法
  Author: Your name · Date: 2026-10-14

  Front matter has unnumbered headings. Use `--input theme=dark` for a dark
  theme and `--input layout=landscape` for two-column PDF output.
]

= Notes / 注释 <notes>

A native footnote keeps supporting details close to the text.#footnote[
  PDF places this at the foot of the page; HTML makes it an expandable panel.
] <shared-note>

The same footnote can be referenced again.#footnote(<shared-note>)

#sidenote[
  A text sidenote / 文字侧注. Keep the main argument here.

  A second paragraph can contain *bold text* and an inline equation $E = m c^2$.
][Additional context appears in the PDF margin or an HTML disclosure.]

// path() resolves the asset relative to this file, including through a package.
#sidenote(side-image: path("assets/signal.svg"))[
  A sidenote can combine an illustration with explanatory text.
][A signal passes through a processing block.]

#sidenote(side-image: image(path("assets/signal.svg")))[
  Image content is also accepted; the aside can be image-only.
][]

= Statements / 定理与证明

#definition[For a real vector $v$, its squared Euclidean norm is $v^T v$.]
#theorem[For any real vector $v$, $v^T v >= 0$.]
#proof[Each term in $v^T v = sum_i v_i^2$ is nonnegative.]
#lemma[If $v^T v = 0$, every component of $v$ is zero.]
#corollary[The Euclidean norm is zero exactly at the zero vector.]

Refer back to the notes in @notes.

= Equations and figures / 公式与图表

$ y = A x + b $ <model>
Equation @model describes an affine model.

#figure(
  image("assets/signal.svg", width: 70%),
  caption: [Signal processing / 信号处理],
) <signal>

#figure(
  table(
    columns: 2,
    table.header([Symbol / 符号], [Meaning / 含义]),
    [$x$], [Input vector],
    [$A$], [Linear map],
    [$b$], [Offset],
  ),
  caption: [Model parameters / 模型参数],
) <parameters>

See @signal and @parameters. Figure and equation numbering follow the output
target's conventions.

== Fletcher diagram

#let flow = diagram(
  node((0, 0), [$x$], name: <input>),
  node((1, 0), [$A x + b$], name: <process>),
  node((2, 0), [$y$], name: <output>),
  edge(<input>, <process>, "->"),
  edge(<process>, <output>, "->"),
)

// The bundled Fletcher version needs an SVG frame for HTML export.
#context if target() == "html" { html.frame(flow) } else { flow }

== Code and quotation

Inline `code 中文` and a block with a language label:

```python
def affine(x, a, b):
    return a * x + b
```

#quote(block: true, attribution: [Example author])[
  Put a short quotation or a separate observation here.
]
