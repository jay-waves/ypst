(() => {
    let generation = 0;
    const navigate = () => {
        const token = ++generation;
        const match = /^#L([1-9]\d*)$/.exec(location.hash);
        if (!match) return;
        const line = Number(match[1]);
        if (!Number.isSafeInteger(line)) return;
        const scroll = () => {
            if (token !== generation) return;
            const nodes = [...document.querySelectorAll('.source-line-anchor[data-line]')];
            let first = null, previous = null;
            for (const node of nodes) {
                const n = Number(node.dataset.line);
                if (!Number.isSafeInteger(n) || n < 1) continue;
                if (!first || n < Number(first.dataset.line)) first = node;
                if (n <= line && (!previous || n > Number(previous.dataset.line))) previous = node;
            }
            const target = previous || first;
            if (!target) return;
            for (let node = target; node; node = node.parentElement) {
                if (node.tagName === 'DETAILS') node.open = true;
                if (node.classList.contains('heading-section')) node.classList.remove('is-collapsed');
            }
            document.querySelectorAll('.heading-section').forEach(section => {
                const summary = section.querySelector(':scope > .heading-summary');
                if (!summary) return;
                const disabled = Boolean(section.parentElement.closest('.heading-section.is-collapsed'));
                summary.tabIndex = disabled ? -1 : 0;
                summary.setAttribute('aria-expanded', String(!section.classList.contains('is-collapsed')));
                if (disabled) summary.setAttribute('aria-disabled', 'true');
                else summary.removeAttribute('aria-disabled');
            });
            (target.closest('h1,h2,h3,h4,h5,h6,p') || target).scrollIntoView({ block: 'start' });
        };
        scroll();
        Promise.all([document.fonts?.ready, new Promise(resolve => {
            if (document.readyState === 'complete') resolve();
            else window.addEventListener('load', resolve, { once: true });
        })]).then(scroll);
    };
    window.addEventListener('hashchange', navigate);
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', navigate, { once: true });
    else navigate();
})();
