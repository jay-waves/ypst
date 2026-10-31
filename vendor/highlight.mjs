import { rawLanguage, highlightSource } from './raw-highlight.mjs';
import { parseHTML } from 'linkedom/worker';
globalThis.highlightTypst = (source, name) => {
  const language = rawLanguage(name);
  const { document } = parseHTML('<html><body></body></html>');
  const code = document.createElement('code');
  if (language) code.innerHTML = highlightSource(source, language);
  else code.textContent = source;
  const quote = value => JSON.stringify(value);
  const children = node => [...node.childNodes].map(child => {
    if (child.nodeType === 3) return '#text(' + quote(child.data) + ')';
    if (child.nodeType === 1) return '#html.elem("span", attrs: (class: ' + quote(child.className) + ',), [' + children(child) + '])';
    return '';
  }).join('');
  const cls = language ? 'hljs language-' + language : 'hljs';
  return '#html.elem("pre", html.elem("code", attrs: (class: ' + quote(cls) + ',), [' + children(code) + ']))';
};
