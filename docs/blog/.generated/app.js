
const root=document.documentElement;
const stored=localStorage.getItem('theme');
if(stored){root.dataset.theme=stored}
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:44px;height:44px;display:grid;place-items:center;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);cursor:pointer;font:24px/1 system-ui}button:hover{border-color:var(--accent)}</style><button type="button"></button>';const btn=this.shadowRoot.querySelector('button');const current=()=>root.dataset.theme||(matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light');const render=()=>{const dark=current()==='dark';btn.textContent=dark?'☾':'☀';btn.title=dark?'Dark mode: switch to light mode':'Light mode: switch to dark mode';btn.setAttribute('aria-label',btn.title)};btn.onclick=()=>{const next=current()==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next);render()};render()}})}


const search=document.getElementById('search');
const cards=[...document.querySelectorAll('.post-card')];
const count=document.getElementById('search-count');
const topicButtons=[...document.querySelectorAll('[data-topic]')];
function applyFilter(term){const q=term.trim().toLowerCase();let shown=0;for(const card of cards){const ok=!q||card.dataset.search.includes(q);card.hidden=!ok;if(ok)shown++}if(count){count.textContent=shown+' post'+(shown===1?'':'s')}for(const btn of topicButtons){btn.setAttribute('aria-pressed',btn.dataset.topic.toLowerCase()===q?'true':'false')}}
if(search){search.addEventListener('input',e=>applyFilter(e.target.value))}
const initialQuery=new URLSearchParams(location.search).get('q');
if(search&&initialQuery){search.value=initialQuery;applyFilter(initialQuery)}
for(const btn of topicButtons){btn.addEventListener('click',()=>{if(search){search.value=btn.dataset.topic;applyFilter(btn.dataset.topic);search.focus()}})}
