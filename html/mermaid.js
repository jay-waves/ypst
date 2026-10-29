(() => {
    if (!window.pele) return;
    document.querySelectorAll('pre > code[data-lang="mermaid"]').forEach(code => {
        const element = code.parentElement;
        element.classList.add('mermaid');
        window.pele.mount(element, code.textContent, { fontSize: 14 });
    });
})();
