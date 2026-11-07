(() => {
    let generation = 0;
    const navigate = href => {
        const token = ++generation;
        const fragment = sameDocumentFragment(href);
        if (fragment === null) return;
        let target = document.getElementById(fragment);
        const match = /^L([1-9]\d*)$/.exec(fragment);
        if (!target && match) {
            const line = Number(match[1]);
            if (!Number.isSafeInteger(line)) return;
            let first = null, previous = null;
            for (const node of document.querySelectorAll('.source-line-anchor[data-line]')) {
                const n = Number(node.dataset.line);
                if (!Number.isSafeInteger(n) || n < 1) continue;
                if (!first || n < Number(first.dataset.line)) first = node;
                if (n <= line && (!previous || n > Number(previous.dataset.line))) previous = node;
            }
            target = previous || first;
        }
        if (!target) return;
        for (let node = target; node; node = node.parentElement) {
            if (node.tagName === 'DETAILS') node.open = true;
            if (node.classList.contains('heading-section')) node.classList.remove('is-collapsed');
        }
        syncHeadingSections(document);
        const scroll = () => {
            if (token === generation) (target.closest('h1,h2,h3,h4,h5,h6,p') || target).scrollIntoView({ block: 'start' });
        };
        scroll();
        Promise.all([document.fonts?.ready, new Promise(resolve => {
            if (document.readyState === 'complete') resolve();
            else window.addEventListener('load', resolve, { once: true });
        })]).then(scroll);
    };
    window.addEventListener('hashchange', () => navigate(location.href));
    document.addEventListener('click', event => {
        if (event.defaultPrevented || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        const link = event.target.closest('a[href]');
        if (link && !link.hasAttribute('download') && (!link.target || link.target === '_self')) navigate(link.href);
    });
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', () => navigate(location.href), { once: true });
    else navigate(location.href);
})();
