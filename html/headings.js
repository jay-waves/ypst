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

    const updateAvailability = () => {
        root.querySelectorAll('.heading-section').forEach(section => {
            const summary = section.querySelector(':scope > .heading-summary');
            const disabled = Boolean(section.parentElement.closest('.heading-section.is-collapsed'));
            summary.tabIndex = disabled ? -1 : 0;
            summary.setAttribute('aria-expanded', String(!section.classList.contains('is-collapsed')));
            if (disabled) summary.setAttribute('aria-disabled', 'true');
            else summary.removeAttribute('aria-disabled');
        });
    };

    children.forEach(node => {
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
        summary.setAttribute('role', 'button');
        summary.append(heading);
        section.append(summary);

        const content = document.createElement('div');
        content.className = 'heading-content';
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
        const heading = event.target.closest('.heading-summary > :is(h1,h2,h3,h4,h5,h6)');
        const summary = heading?.parentElement;
        if (summary && summary.getAttribute('aria-disabled') !== 'true') {
            toggle(summary);
            return;
        }

        const link = event.target.closest('a[href]');
        if (!link) return;
        const fragment = sameDocumentFragment(link);
        if (fragment === null) return;
        const target = document.getElementById(fragment);
        if (!target || !root.contains(target)) return;
        for (let section = target?.closest('.heading-section'); section;
            section = section.parentElement.closest('.heading-section')) {
            section.classList.remove('is-collapsed');
        }
        updateAvailability();
    });

    root.addEventListener('keydown', event => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        const summary = event.target.closest('.heading-summary');
        if (!summary || summary.getAttribute('aria-disabled') === 'true') return;
        event.preventDefault();
        toggle(summary);
    });
})();
