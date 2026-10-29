#document("index.html")[
#let __ypst_preview_image(url, ..args) = {
  let picture = image(..args)
  show image: it => {
    let attrs = (src: url,)
    if it.alt != none { attrs.insert("alt", it.alt) }
    if it.width != auto { attrs.insert("style", "width: " + repr(it.width)) }
    if it.height != auto { attrs.insert("style", "height: " + repr(it.height)) }
    html.elem("img", attrs: attrs)
  }
  picture
}
#import "../lib.typ": template, theorem
#show: template
= #html.elem("span", attrs: (id: "L3", "data-line": "3", class: "source-line-anchor"))标题 Heading
= #html.elem("span", attrs: (id: "L4", "data-line": "4", class: "source-line-anchor"))数学标题 $f(x) = alpha x^2 + beta$
#html.elem("span", attrs: (id: "L5", "data-line": "5", class: "source-line-anchor"))正文公式 $f(x) = alpha x^2 + beta$。

== #html.elem("span", attrs: (id: "L7", "data-line": "7", class: "source-line-anchor"))二级标题 $sum_(i=1)^n x_i$
=== #html.elem("span", attrs: (id: "L8", "data-line": "8", class: "source-line-anchor"))三级标题 $bold(v) in RR^n$
==== #html.elem("span", attrs: (id: "L9", "data-line": "9", class: "source-line-anchor"))四级标题 $A x = b$

#html.elem("span", attrs: (id: "L11", "data-line": "11", class: "source-line-anchor"))正文 Latin，中文。 *加粗 Bold* _强调 Italic_

#strong[加粗 #emph[强调 Both 中文]]

#emph[强调 #strong[加粗 Both 中文]]

#table(columns: 2, table.header([表头 Header], [值 Value]), [中文 Text], [450])

`code 中文`

```cpp
// 中文 comment
int value = 1;
```

#theorem[中文 theorem *加粗 Bold*]

]
