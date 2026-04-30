const crateSearch=document.getElementById('crate-search');
if(!customElements.get('er-crate-row')){customElements.define('er-crate-row',class extends HTMLElement{})}
const crateCards=[...document.querySelectorAll('[data-crate-search]')];
const crateCount=document.getElementById('crate-search-count');
const crateSort=document.getElementById('crate-sort');
const crateList=document.querySelector('.crate-list');
function applyCrateSearch(value){const query=value.trim().toLowerCase();let shown=0;for(const card of crateCards){const match=!query||card.dataset.crateSearch.toLowerCase().includes(query);card.hidden=!match;if(match)shown++}if(crateCount){crateCount.textContent=shown+' crates'}}
if(crateSearch){crateSearch.addEventListener('input',()=>applyCrateSearch(crateSearch.value))}
const initialCrateQuery=new URLSearchParams(location.search).get('q');
if(crateSearch&&initialCrateQuery){crateSearch.value=initialCrateQuery;applyCrateSearch(initialCrateQuery)}
function crateSortValue(card,key){if(key==='name')return card.dataset.name||'';return Number(card.dataset[key]||0)}
function applyCrateSort(){if(!crateSort||!crateList)return;const key=crateSort.value;const sorted=[...crateCards].sort((a,b)=>key==='name'?crateSortValue(a,key).localeCompare(crateSortValue(b,key)):crateSortValue(b,key)-crateSortValue(a,key)||crateSortValue(a,'name').localeCompare(crateSortValue(b,'name')));for(const card of sorted){crateList.appendChild(card)}}
if(crateSort){crateSort.addEventListener('change',applyCrateSort)}
