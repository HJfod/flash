
feather.replace();

// This reminds me of 8th grade

const entNav = document.getElementById('nav-entities');
const fileNav = document.getElementById('nav-files');
const entTab = document.getElementById('nav-tab-entities');
const fileTab = document.getElementById('nav-tab-files');
const mainBody = document.querySelector('body > main');
const searchInput = document.getElementById('nav-search');
const searchGlass = document.getElementById('nav-clear-glass');
const searchX = document.getElementById('nav-clear-x');

let searchNav = undefined;
let searchQuery = '';

searchInput.addEventListener('input', e => {
    search(e.target.value);
});

function clearSearch() {
    searchInput.value = '';
    search('');
}

function search(query) {
    searchQuery = query;
    updateNav();
}

function getFullName(node) {
    let parent = node;
    const result = [node.innerText.trim()];
    while (parent.parentElement) {
        parent = parent.parentElement;
        if (parent.tagName === 'DETAILS') {
            result.splice(0, 0, parent.querySelector('summary').innerText.trim());
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
    if (fileTab.classList.contains('selected')) {
        return fileNav;
    } else {
        return entNav;
    }
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
    
        const results = [];
        currentNav().querySelectorAll('a').forEach(a => {
            const match = furryMatchMany(
                getFullName(a), searchQuery,
                fileTab.classList.contains('selected') ? '/' : '::'
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
        // Sort by match quality
        results.sort((a, b) => b[0] - a[0]).forEach(([_, clone]) => {
            searchResults.appendChild(clone);
        });

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

        if (fileTab.classList.contains('selected')) {
            fileNav.style.display = null;
            entNav.style.display = 'none';
        } else {
            fileNav.style.display = 'none';
            entNav.style.display = null;
        }
    }
}

function showEntityNav() {
    entTab.classList.add('selected');
    fileTab.classList.remove('selected');
    updateNav();
}

function showFileNav() {
    entTab.classList.remove('selected');
    fileTab.classList.add('selected');
    updateNav();
}

function navigate(url) {
    // todo: progress indicator
    fetch(`${url}/content.html`)
        .then(res => res.text())
        .then(content => {
            window.history.pushState({
                html: content,
                // "title": 
            }, "", url);
            mainBody.innerHTML = content;
            mainBody.scrollTo({ left: 0, top: 0 });
            feather.replace();
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
        // document.title = e.state.title;
        feather.replace();
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
    for (const cls of document.body.classList) {
        if (cls.startsWith('flash-theme-')) {
            document.body.classList.remove(cls);
        }
    }
    document.body.classList.add(`flash-theme-${name}`);
}
