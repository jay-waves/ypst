// Resolve against <base>, but compare with the actual document URL.
// Return null for another document, an empty fragment, or invalid encoding.
const sameDocumentFragment = (link) => {
    try {
        const href = link.getAttribute('href');
        if (href === null) return null;
        const target = new URL(href, document.baseURI);
        const current = new URL(document.URL);
        const fragment = target.hash.slice(1);
        target.hash = '';
        current.hash = '';
        if (!fragment || target.href !== current.href) return null;
        return decodeURIComponent(fragment);
    } catch {
        return null;
    }
};

// Keep keyboard and accessibility state aligned after folding or navigation.
const syncHeadingSections = root => {
    root.querySelectorAll('.heading-section').forEach(section => {
        const summary = section.querySelector(':scope > .heading-summary');
        if (!summary) return;
        const disabled = Boolean(section.parentElement.closest('.heading-section.is-collapsed'));
        summary.tabIndex = disabled ? -1 : 0;
        summary.setAttribute('aria-expanded', String(!section.classList.contains('is-collapsed')));
        if (disabled) summary.setAttribute('aria-disabled', 'true');
        else summary.removeAttribute('aria-disabled');
    });
};
