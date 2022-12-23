
feather.replace();

// This reminds me of 8th grade

const entNav = document.getElementById("nav-entities");
const fileNav = document.getElementById("nav-files");
const entTab = document.getElementById("nav-tab-entities");
const fileTab = document.getElementById("nav-tab-files");
const mainBody = document.querySelector("body > main");

function showEntityNav() {
    entNav.style.display = null;
    fileNav.style.display = 'none';
    entTab.classList.add('selected');
    fileTab.classList.remove('selected');
}

function showFileNav() {
    entNav.style.display = 'none';
    fileNav.style.display = null;
    entTab.classList.remove('selected');
    fileTab.classList.add('selected');
}

function navigate(url) {
    fetch(`${url}/content.html`)
        .then(res => res.text())
        .then(content => {
            window.history.pushState({
                html: content,
                // "title": 
            }, "", url);
            mainBody.innerHTML = content;
        })
        .catch(err => {
            console.error(err);
        });
}

window.onpopstate = e => {
    if (e.state) {
        mainBody.innerHTML = e.state.html;
        // document.title = e.state.title;
    }
};
