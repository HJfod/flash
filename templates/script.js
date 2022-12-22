
feather.replace();

// This reminds me of 8th grade

const entNav = document.getElementById("nav-entities");
const fileNav = document.getElementById("nav-files");
const entTab = document.getElementById("nav-tab-entities");
const fileTab = document.getElementById("nav-tab-files");

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
