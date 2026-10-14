(() => {
    const convertFootnotes = () => {
        // Keep native details as the no-script fallback. With scripts, put the
        // marker in the final body paragraph and the panel after the body.
        document.querySelectorAll('.sidenote > .sidenote-toggle').forEach((details, index) => {
            const wrapper = details.parentElement;
            const aside = details.querySelector(':scope > aside');
            if (!aside) return;
            const label = aside.querySelector(':scope > .sidenote-label');
            label.setAttribute('aria-label', 'Close sidenote and return to reference');
            aside.id = `sidenote-panel-${index + 1}`;
            aside.hidden = true;
            const marker = document.createElement('button');
            marker.type = 'button';
            marker.id = `${aside.id}-ref`;
            label.href = new URL(`#${marker.id}`, document.URL).href;
            marker.className = 'sidenote-marker';
            marker.textContent = '*';
            marker.setAttribute('aria-label', 'Toggle sidenote');
            marker.setAttribute('aria-controls', aside.id);
            marker.setAttribute('aria-expanded', 'false');
            const sup = document.createElement('sup');
            sup.append(marker);
            const body = wrapper.querySelector(':scope > .sidenote-body');
            const last = body.lastElementChild;
            (last?.matches('p') ? last : body).append(sup);
            details.replaceWith(aside);
            label.addEventListener('click', event => {
                event.preventDefault();
                aside.hidden = true;
                marker.setAttribute('aria-expanded', 'false');
                marker.focus({ preventScroll: true });
                marker.scrollIntoView({ block: 'nearest' });
            });
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

                const label = document.createElement('a');
                label.className = 'footnote-label';
                label.setAttribute('aria-label', 'Close footnote and return to reference');

                const copy = source.cloneNode(true);
                copy.querySelectorAll('[role="doc-backlink"]').forEach(backlink => backlink.remove());
                copy.querySelectorAll('[data-source-line]').forEach(node => node.removeAttribute('data-source-line'));
                const content = document.createElement('div');
                content.className = 'note-content';
                content.append(...copy.childNodes);
                note.append(label, content);

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

                panel = { note, label, references: [] };
                bySource.set(sourceId, panel);
                converted.add(sourceId);

                label.addEventListener('click', event => {
                    event.preventDefault();
                    panel.note.hidden = true;
                    for (const ref of panel.references) ref.setAttribute('aria-expanded', 'false');
                    const ref = panel.references.find(item => item.id === sameDocumentFragment(label)) || panel.references[0];
                    ref?.focus({ preventScroll: true });
                    ref?.scrollIntoView({ block: 'nearest' });
                });
            }

            if (!reference.id) reference.id = `inline-footnote-ref-${++index}`;
            panel.references.push(reference);
            reference.href = new URL(`#${panel.note.id}`, document.URL).href;
            reference.setAttribute('data-footnote-ref', panel.note.id);
            reference.setAttribute('aria-controls', panel.note.id);
            reference.setAttribute('aria-expanded', 'false');
            reference.addEventListener('click', event => {
                event.preventDefault();
                panel.note.hidden = !panel.note.hidden;
                for (const ref of panel.references) ref.setAttribute('aria-expanded', String(!panel.note.hidden));
                if (!panel.note.hidden) panel.label.href = new URL(`#${reference.id}`, document.URL).href;
            });
            panel.label.textContent = reference.textContent;
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
