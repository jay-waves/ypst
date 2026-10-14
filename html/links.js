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
