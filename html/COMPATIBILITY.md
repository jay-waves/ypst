# HTML stylesheet

`theme.css` primarily targets standalone Typst and Markdown HTML. It styles standard elements and includes font declarations, light/dark colors, and Typst note panels.

```html
<link rel="stylesheet" href="theme.css">
```

Set `data-theme="dark"` on `<html>` or `<body>` for dark mode. Typst exports select the theme automatically.

Optional font files belong in `ypst-fonts/` beside the CSS file, or beside the HTML when CSS is embedded. Keep their licenses. Highlighting and math-rendering resources remain the host's responsibility.

## Editor previews

Basic theme classes are recognized on a best-effort basis. Editor compatibility and application-style isolation are not guaranteed; the stylesheet uses page-level `body` rules.

- VS Code: load it through `markdown.styles`.
- Obsidian: prefer a separate, scoped snippet containing only the rules you need. Loading this entire file can affect the application layout.
- markdown-it: load it in a standalone HTML page containing the rendered output.

CSS alone does not add interactive notes or collapsible headings. Typst exports include their scripts.
