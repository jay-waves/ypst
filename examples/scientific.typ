#import "../lib.typ": template
#import "@preview/physica:0.9.8" as physica
#import "@preview/cetz:0.5.2" as cetz
#set document(title: "Scientific packages / 科学排版")
#show: template

= Physica notation

Use qualified names to keep package functions separate from template helpers.

$ physica.pdv(f, x) = 2 x, quad physica.curl (physica.grad f) = 0 $

Mixed partial derivatives and tensor indices:

$ physica.pdv(f, x, y, [1, 2]), quad physica.tensor(T, -mu, +nu) $

= CeTZ drawing

// CeTZ produces paged graphics. html.frame exports them as SVG in HTML.
#let drawing = cetz.canvas({
  import cetz.draw: *
  line((0, 0), (4, 0), stroke: rgb("#62666b"))
  circle((1, 0), radius: 0.35, fill: rgb("#e9ecea"), stroke: rgb("#62666b"))
  circle((3, 0), radius: 0.35, fill: rgb("#e9ecea"), stroke: rgb("#62666b"))
  content((1, 0), [$x$])
  content((3, 0), [$y$])
})

#figure(
  context if target() == "html" { html.frame(drawing) } else { drawing },
  caption: [Two nodes connected by a line.],
)

The Physica expressions use native math output. The CeTZ illustration is an
SVG in HTML, so its equations are part of the drawing rather than MathML.
