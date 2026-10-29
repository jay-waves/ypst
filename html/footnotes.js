(() => {
    const convertFootnotes = () => {
        // Keep native details as the no-script fallback. With scripts, put the
        // marker in the final body paragraph and the panel after the body.
        document.querySelectorAll('.sidenote > .sidenote-toggle').forEach((details, index) => {
            const wrapper = details.parentElement;
            const aside = details.querySelector(':scope > aside');
            if (!aside) return;
            aside.id = `sidenote-panel-${index + 1}`;
            aside.hidden = true;
            const marker = document.createElement('button');
            marker.type = 'button';
            marker.id = `${aside.id}-ref`;
            marker.className = 'sidenote-marker';
            const symbol = document.createElement('sup');
            symbol.textContent = '※';
            marker.append(symbol);
            marker.setAttribute('aria-label', 'Toggle sidenote');
            marker.setAttribute('aria-controls', aside.id);
            marker.setAttribute('aria-expanded', 'false');
            const body = wrapper.querySelector(':scope > .sidenote-body');
            // Descend through trailing block wrappers rather than placing the
            // reference after them, which would start a separate line.
            let tail = body;
            while (true) {
                while (tail.lastChild?.nodeType === Node.TEXT_NODE && !tail.lastChild.textContent.trim()) {
                    tail.lastChild.remove();
                }
                const last = tail.lastChild;
                if (last?.nodeType === Node.ELEMENT_NODE &&
                    ['block', 'list-item', 'flow-root'].includes(getComputedStyle(last).display)) {
                    tail = last;
                } else break;
            }
            if (tail.lastChild?.nodeType === Node.TEXT_NODE) {
                tail.lastChild.textContent = tail.lastChild.textContent.trimEnd();
            }
            // Keep the reference attached to the preceding text at line breaks.
            tail.append(document.createTextNode('\u2060'), marker);
            details.replaceWith(aside);
            marker.addEventListener('click', () => {
                aside.hidden = !aside.hidden;
                marker.setAttribute('aria-expanded', String(!aside.hidden));
            });
        });
        const endnotes = document.querySelector('[role="doc-endnotes"]');
        if (!endnotes) return;

        const notes = new Map([...endnotes.querySelectorAll('li[id]')].map(note => [note.id, note]));
        const panels = new WeakMap();
        const insertedAfter = new Map();
        const converted = new Set();
        let activePanel = null;
        let index = 0;

        document.querySelectorAll('[role="doc-noteref"] a[href]').forEach(reference => {
            const sourceId = sameDocumentFragment(reference);
            if (sourceId === null) return;
            const source = notes.get(sourceId);
            if (!source) return;

            const table = reference.closest('table');
            const paragraph = table ? null : reference.closest('p, h1, h2, h3, h4, h5, h6');
            const container = table || paragraph || reference.closest('li, blockquote, td, th, .sidenote-body') || reference.closest('[role="doc-noteref"]') || reference.parentElement;
            if (!container) return;

            let bySource = panels.get(container);
            if (!bySource) panels.set(container, bySource = new Map());
            let panel = bySource.get(sourceId);
            if (!panel) {
                const note = document.createElement('aside');
                note.className = 'inline-footnote';
                note.id = `inline-footnote-${++index}`;
                note.setAttribute('role', 'doc-footnote');
                note.hidden = true;

                const copy = source.cloneNode(true);
                copy.querySelectorAll('[role="doc-backlink"]').forEach(backlink => backlink.remove());
                copy.querySelectorAll('[data-source-line]').forEach(node => node.removeAttribute('data-source-line'));
                const content = document.createElement('div');
                content.className = 'note-content';
                content.append(...copy.childNodes);
                note.append(content);

                const previous = insertedAfter.get(container);
                if (container === table || container === paragraph || container.matches('h1,h2,h3,h4,h5,h6')) {
                    (previous || container).after(note);
                    insertedAfter.set(container, note);
                } else if (container.matches('li,blockquote,td,th')) {
                    if (previous) previous.after(note);
                    else container.append(note);
                    insertedAfter.set(container, note);
                } else {
                    (previous || container).after(note);
                    insertedAfter.set(container, note);
                }

                panel = { note, references: [] };
                bySource.set(sourceId, panel);
                converted.add(sourceId);
            }

            if (!reference.id) reference.id = `inline-footnote-ref-${++index}`;
            panel.references.push(reference);
            reference.href = new URL(`#${panel.note.id}`, document.URL).href;
            reference.setAttribute('data-footnote-ref', panel.note.id);
            reference.setAttribute('aria-controls', panel.note.id);
            reference.setAttribute('aria-expanded', 'false');
            reference.addEventListener('click', event => {
                event.preventDefault();
                const opening = panel.note.hidden;
                if (activePanel) {
                    activePanel.note.hidden = true;
                    for (const ref of activePanel.references) ref.setAttribute('aria-expanded', 'false');
                }
                activePanel = opening ? panel : null;
                if (activePanel) {
                    panel.note.hidden = false;
                    for (const ref of panel.references) ref.setAttribute('aria-expanded', 'true');
                }
            });
        });

        for (const id of converted) notes.get(id)?.remove();
        if (!endnotes.querySelector('li[id]')) endnotes.remove();
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', convertFootnotes, { once: true });
    } else {
        convertFootnotes();
    }
})();
