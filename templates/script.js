'use strict';

// This reminds me of 8th grade

const nav = document.querySelector('nav');
const navModeBtns = nav.querySelector('.mode');
const mainBody = document.querySelector('body > main');
const searchInput = document.getElementById('nav-search');
const searchGlass = document.getElementById('nav-clear-glass');
const searchX = document.getElementById('nav-clear-x');

let searchNav = undefined;
let searchQuery = '';

let memberFunctionsList = null;

function createCopyButton(icon, text, callback = undefined) {
    const button = document.createElement('button');
    button.innerHTML = `${icon}`;
    button.addEventListener('click', _ => {
        if (navigator.clipboard) {
            navigator.clipboard.writeText(text)
                .then(() => {
                    button.innerHTML = `${feather.icons.check.toSvg()}`;
                    button.classList.add('success');
                    if (callback) {
                        callback();
                    }
                },
                () => {
                    button.innerHTML = `${feather.icons.x.toSvg()}`;
                    button.classList.add('failure');
                });
        }
        else {
            button.innerHTML = `${feather.icons.x.toSvg()}`;
            button.classList.add('failure');
        }
        setTimeout(_ => {
            button.innerHTML = `${icon}`;
            button.classList.remove('success');
            button.classList.remove('failure');
        }, 1500);
    });
    return button;
}

// Add copy button to code blocks
Prism.hooks.add('complete', env => {
    // Check if inline or actual code block (credit to line-numbers plugin)
    const pre = env.element.parentNode;
    if (!pre || !/pre/i.test(pre.nodeName)) {
        return;
    }

    // Return if there already is a toolbar
    if (pre.classList.contains('has-toolbar')) {
        return;
    }

    pre.classList.add('has-toolbar');

    const wrapper = document.createElement('div');
    wrapper.classList.add('toolbar-wrapper');
    pre.parentNode.replaceChild(wrapper, pre);

    // Add toolbar
    const toolbar = document.createElement('div');
    toolbar.classList.add('toolbar');
    wrapper.appendChild(toolbar);
    wrapper.appendChild(pre);

    toolbar.appendChild(createCopyButton(
        feather.icons.copy.toSvg(),
        env.code
    ));
});

searchInput.addEventListener('input', e => {
    search(e.target.value);
});

function headingLink(value) {
    return value
        // make lower-case
        .toLowerCase()
        // remove non-alphanumeric
        .replace(/[^a-z0-9\s]/g, '')
        // remove duplicate whitespace
        // convert to hyphens
        .replace(/\s+/g, '-')
}

function highlight() {
    // Add links to all top-level headings
    document.querySelectorAll('.text > h1, .text > h2, .text > h3')
        .forEach(head => {
            if (!head.querySelector('.get-header-link')) {
                let currentUrl = window.location.href;
                while (currentUrl.endsWith('/')) {
                    currentUrl = currentUrl.slice(0, -1)
                }
                const linkBtn = createCopyButton(
                    feather.icons.link.toSvg(),
                    `${currentUrl}#${head.getAttribute('id')}`,
                    () => {
                        window.location.hash = head.getAttribute('id');
                    }
                );
                linkBtn.classList.add('get-header-link');
                head.appendChild(linkBtn);
            }
        });
    
    // Highlight warning quotes
    document.querySelectorAll('blockquote > p')
        .forEach(quote => {
            if (quote.innerText.includes('⚠️')) {
                quote.parentElement.classList.add('warning');
            }
            if (quote.innerText.includes('ℹ️')) {
                quote.parentElement.classList.add('info');
            }
            if (quote.innerText.includes('📗')) {
                quote.parentElement.classList.add('book');
            }
        });

    Prism.highlightAll();
    feather.replace();
    twemoji.parse(document.body);
}

function clearSearch() {
    searchInput.value = '';
    search('');
}

function search(query) {
    searchQuery = query;
    if (!memberFunctionsList && selectedNavTab() == 'entities') {
        fetch(`${FLASH_OUTPUT_URL}/functions.json`)
            .then(res => res.json())
            .then(res => {
                memberFunctionsList = res;
                search(searchQuery);
            });
    }
    updateNav();
}

function getFullName(node) {
    let parent = node;
    const result = [node.textContent.trim()];
    while (parent.parentElement) {
        parent = parent.parentElement;
        if (parent.tagName === 'DETAILS') {
            result.splice(0, 0, parent.querySelector('summary').textContent.trim());
        }
    }
    return result;
}

function furryMatch(str, query) {
    // remove all whitespace from query since entities can't have that anyway
    // todo: maybe split query to words instead and only require some of those to match instead of whole query
    query = query.replace(/\s/g, '');

    if (!query.length) {
        return undefined;
    }
    
    let score = 0;
    let matchedString = '';
    let toMatch = 0;
    let matchedInARow = 0;
    for (let i = 0; i < str.length; i++) {
        const current = str[i];
        // if matches query
        if (current.toLowerCase() === query[toMatch].toLowerCase()) {
            // uppercase is a weighted bonus
            if (current.toUpperCase() === current) {
                score += 2;
            }
            // lowercase is a bonus for matching case
            else {
                score += 1;
            }
            // first letter match is a bonus
            if (i === 0) {
                score += 5;
            }

            // multiple successive matches in a row is a bonus
            score += matchedInARow;
            matchedInARow++;

            // if this was the first match in a row, open up a span in the resulting string
            if (matchedInARow === 1) {
                matchedString += '<span class="matched">';
            }
            matchedString += current;

            // match next char in query next
            toMatch++;
            // if at end, stop matching
            if (toMatch === query.length) {
                matchedString += '</span>';
                matchedString += str.substring(i + 1);
                break;
            }
        }
        else {
            // close span if there were a bunch of consequent matches
            if (matchedInARow) {
                matchedString += '</span>';
            }
            matchedString += current;
            matchedInARow = 0;
        }
    }
    // all characters in query must have been matched
    return toMatch === query.length ?
        {
            // the more of the string was matched by the query, the better
            score: (score - (str.length - query.length) / 10),
            matched: matchedString
        } : undefined;
}

function furryMatchMany(list, query, separator) {
    let matched = '';
    let score = 0;
    let someMatched = false;
    let i = 0;
    for (const item of list) {
        if (matched.length) {
            matched += `<span class="scope">${separator}</span>`;
        }
        const match = furryMatch(item, query);
        if (match) {
            matched += match.matched;
            score += match.score;
            someMatched = true;
            // namespace match is a penaulty
            if (i !== list.length - 1) {
                score -= 5;
            }
        }
        else {
            matched += item;
        }
        i++;
    }
    return someMatched ? { score, matched } : undefined;
}

function currentNav() {
    return nav.querySelector(`#nav-content-${selectedNavTab()}`);
}

function selectedNavTab() {
    return navModeBtns
        .querySelector(`.selected`)
        .getAttribute('id')
        .replace('nav-tab-', '');
}

function updateNav() {
    if (searchQuery.length) {
        // hide current navigation
        currentNav().style.display = 'none';
        if (searchNav) {
            searchNav.remove();
        }

        searchGlass.style.display = 'none';
        searchX.style.display = null;

        const searchResults = document.createElement('div');
        searchResults.classList.add('content');
        if (currentNav().classList.contains('monospace')) {
            searchResults.classList.add('monospace');
        }
    
        const results = [];
        currentNav().querySelectorAll('a').forEach(a => {
            const match = furryMatchMany(
                getFullName(a), searchQuery,
                selectedNavTab() == 'entities' ? '::' : '/'
            );
            if (match) {
                const clone = a.cloneNode(false);
                const svg = a.querySelector('svg');
                clone.innerHTML = match.matched;
                // copy any icons over
                if (svg) {
                    clone.insertBefore(svg.cloneNode(true), clone.firstChild);
                }
                results.push([match.score, clone]);
            }
        });
        if (selectedNavTab() == 'entities') {
            memberFunctionsList?.forEach(fun => {
                let f = fun.split('::');
                const name = f.pop();
                const match = furryMatchMany([name], searchQuery, '::');
                if (match) {
                    const node = document.createElement('a');
                    const url = `${FLASH_OUTPUT_URL}/classes/${f.join('/')}#${name.replace(/\s+\([0-9]+\)/, '')}`;
                    node.setAttribute('href', url);
                    node.addEventListener('click', e => {
                        navigate(url);
                        e.preventDefault();
                    });
                    f = f.map(a => `<span class="namespace">${a}</span>`);
                    f.push(match.matched);
                    node.innerHTML = feather.icons.code.toSvg({ 'class': 'icon class' }) + 
                        f.join('<span class="scope">::</span>');
                    results.push([match.score, node]);
                }
            });
        }
        // Sort by match quality (also limit results for better performance)
        results.sort((a, b) => b[0] - a[0]).slice(0, 350).forEach(([_, clone]) => {
            searchResults.appendChild(clone);
        });

        // No results found
        if (!results.length) {
            const info = document.createElement('p');
            info.classList.add('nothing-found');
            info.innerText = 'No results found';
            searchResults.appendChild(info);
        }
    
        currentNav().parentElement.insertBefore(searchResults, currentNav());
    
        searchNav = searchResults;
    }
    else {
        if (searchNav) {
            searchNav.remove();
            searchNav = undefined;
        }

        searchGlass.style.display = null;
        searchX.style.display = 'none';

        // hide all navs but show the currently selected one
        nav.querySelectorAll('.content').forEach(content => {
            if (content.getAttribute('id').replace('nav-content-', '') === selectedNavTab()) {
                content.style.display = null;
            }
            else {
                content.style.display = 'none'
            }
        });
    }
}

function scrollAndOpenElement(id) {
    if (id) {
        if (id.startsWith('#')) {
            id = id.substring(1);
        }
        const target = document.getElementById(id);
        if (target) {
            target.scrollIntoView();
            document.querySelectorAll('.highlight')
                .forEach(h => h.classList.remove('highlight'));
            target.classList.add('highlight');
            if (target.tagName === 'DETAILS') {
                target.open = true;
            }
        }
    }
}

function showNav(id) {
    [...navModeBtns.children].forEach(node => node.classList.remove('selected'));
    navModeBtns.querySelector(`#nav-tab-${id}`).classList.add('selected');
    updateNav();
}

function navigate(url) {
    const trueURL = url.split('#').shift();
    const head = url.split('#').pop();
    Promise.all([
        fetch(`${trueURL}/content.html`).then(res => res.text()),
        fetch(`${trueURL}/metadata.json`).then(res => res.json()),
    ]).then(([content, metadata]) => {
            window.history.pushState({
                html: content,
                ...metadata,
            }, "", url);
            document.title = metadata.title;
            mainBody.innerHTML = content;
            mainBody.scrollTo({ left: 0, top: 0 });
            nav.querySelectorAll('a.selected').forEach(a => a.classList.remove('selected'));
            nav.querySelector(`[href="${url}"]`)?.classList.add('selected');
            highlight();
            // hide navbar
            nav.classList.add('collapsed');
            scrollAndOpenElement(head);
        })
        .catch(err => {
            console.error(err);
        });
    
    // Prevent calling default onclick handler
    return false;
}

window.onpopstate = e => {
    if (e.state) {
        mainBody.innerHTML = e.state.html;
        document.title = e.state.title;
        highlight();
    }
};

document.querySelectorAll('[data-pick-theme]').forEach(btn => {
    btn.addEventListener('click', e => {
        pickTheme(btn.getAttribute('data-pick-theme'));
        // deselect other buttons
        btn.parentElement.querySelectorAll('.selected')
            .forEach(b => b.classList.remove('selected'));
        // select this one
        btn.classList.add('selected');
    });
});

function pickTheme(name) {
    if (!name) return;
    for (const cls of document.body.classList) {
        if (cls.startsWith('flash-theme-')) {
            document.body.classList.remove(cls);
        }
    }
    document.body.classList.add(`flash-theme-${name}`);
    localStorage.setItem('theme', name);
}

function toggleMenu() {
    nav.classList.toggle('collapsed');
}

// Highlight everything
highlight();

// Mark the current page in nav as seleted
{
    let currentUrl = window.location.pathname;
    while (currentUrl.endsWith('/')) {
        currentUrl = currentUrl.slice(0, -1)
    }
    const a = nav.querySelector(`[href="${currentUrl}"]`);
    if (a) {
        // Find the parent nav section of the selected item
        let parentNav = a.closest('.content');
        showNav(parentNav.getAttribute('id').replace('nav-content-', ''));

        // Open all enclosing <details> elements
        let details = a.closest('details');
        while (parentNav.contains(details)) {
            details.open = true;
            details = details.parentNode.closest('details') ?? null;
        }

        // Scroll the selected item into view
        a.classList.add('selected');
        a.scrollIntoView(false);

        scrollAndOpenElement(window.location.hash);
    }
}

// Detect header link change
window.addEventListener('hashchange', () => {
    scrollAndOpenElement(window.location.hash);
});

// Restore selected theme by clicking the selected theme button
document.querySelector(`[data-pick-theme="${
    localStorage.getItem('theme') ?? 'dark'
}"]`)?.click();
