// Resolve against <base>, but compare with the actual document URL.
// Return null for another document, an empty fragment, or invalid encoding.
const sameDocumentFragment = href => {
    try {
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
        const button = section.querySelector(':scope > .heading-summary .heading-toggle');
        button.setAttribute('aria-expanded', String(!section.classList.contains('is-collapsed')));
    });
};
