(() => {
    const root = document.querySelector('.typst-document');
    if (!root) return;

    const storageKey = `typst-collapsed-headings:${location.pathname}`;
    let saved = new Set();
    try {
        saved = new Set(JSON.parse(localStorage.getItem(storageKey) || '[]'));
    } catch {}

    const stack = [];
    const occurrences = new Map();
    const children = [...root.childNodes];
    root.replaceChildren();

    const updateAvailability = () => syncHeadingSections(root);

    children.forEach((node, index) => {
        const heading = node.nodeType === Node.ELEMENT_NODE && /^H[1-6]$/.test(node.tagName) ? node : null;
        if (!heading) {
            (stack.at(-1)?.content || root).append(node);
            return;
        }

        const level = Number(heading.tagName.slice(1));
        while (stack.length && stack.at(-1).level >= level) stack.pop();

        const section = document.createElement('section');
        section.className = 'heading-section';
        section.dataset.headingLevel = String(level);
        const title = heading.textContent.trim().replace(/\s+/g, ' ');
        const base = heading.id || `${level}:${title.toLocaleLowerCase()}`;
        const occurrence = (occurrences.get(base) || 0) + 1;
        occurrences.set(base, occurrence);
        section.dataset.headingKey = `${base}:${occurrence}`;
        if (saved.has(section.dataset.headingKey)) section.classList.add('is-collapsed');

        const summary = document.createElement('div');
        summary.className = 'heading-summary';
        summary.append(heading);
        section.append(summary);

        const content = document.createElement('div');
        content.className = 'heading-content';
        content.id = `ypst-heading-content-${index}`;
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'heading-toggle';
        button.textContent = '¶';
        button.setAttribute('aria-label', `Toggle section: ${title}`);
        button.setAttribute('aria-controls', content.id);
        heading.append(button);
        section.append(content);
        (stack.at(-1)?.content || root).append(section);
        stack.push({ level, content });
    });

    updateAvailability();

    const toggle = summary => {
        const section = summary.closest('.heading-section');
        if (!section) return;
        section.classList.toggle('is-collapsed');
        updateAvailability();
        try {
            localStorage.setItem(storageKey, JSON.stringify([...root.querySelectorAll('.heading-section.is-collapsed')]
                .map(item => item.dataset.headingKey)));
        } catch {}
    };

    root.addEventListener('click', event => {
        const button = event.target.closest('.heading-toggle');
        const heading = event.target.closest('.heading-summary > :is(h1,h2,h3,h4,h5,h6)');
        const interactive = event.target.closest('a[href], button, input, select, textarea, summary, [role="button"]');
        if (button || (heading && !interactive)) toggle((button || heading).closest('.heading-summary'));
    });
})();
