const crateSearch=document.getElementById('crate-search');
const crateCards=[...document.querySelectorAll('[data-crate-search]')];
const crateCount=document.getElementById('crate-search-count');
function applyCrateSearch(value){const query=value.trim().toLowerCase();let shown=0;for(const card of crateCards){const match=!query||card.dataset.crateSearch.toLowerCase().includes(query);card.hidden=!match;if(match)shown++}if(crateCount){crateCount.textContent=shown+' crates'}}
if(crateSearch){crateSearch.addEventListener('input',()=>applyCrateSearch(crateSearch.value))}
