if(new URLSearchParams(location.search).get('workspace')==='1'){document.documentElement.dataset.workspace='embedded'}
const messagesEl=document.getElementById('messages');
const content=document.getElementById('content');
const statusEl=document.getElementById('status');
const mailCount=document.getElementById('mailCount');

const refreshBtn=document.getElementById('refresh');
const composeBtn=document.getElementById('composeBtn');
const replyMsg=document.getElementById('replyMsg');
const composeEl=document.getElementById('compose');
const searchEl=document.getElementById('search');
const filePick=document.getElementById('filePick');
const composeAttachments=document.getElementById('composeAttachments');
const markRead=document.getElementById('markRead');
const markUnread=document.getElementById('markUnread');
const deleteMsg=document.getElementById('deleteMsg');
const logout=document.getElementById('logout');
const attachBtn=document.getElementById('attachBtn');
const cancelBtn=document.getElementById('cancel');
const sendBtn=document.getElementById('sendBtn');
const toEl=document.getElementById('to');
const subjectEl=document.getElementById('subject');
const bodyEl=document.getElementById('body');

let selected='',messages=[],checked=new Set(),draftAttachments=[],loading=false,sending=false,statusTimer=0,openedMessage=null;

function esc(s){return (s||'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function flash(s,warn,timeout=1800){
    clearTimeout(statusTimer);
    statusEl.textContent=s||'';
    statusEl.classList.toggle('warn',!!warn);
    if(s)statusTimer=setTimeout(()=>{statusEl.textContent='';statusEl.classList.remove('warn')},timeout);
}
function fmtSize(n){return n>1048576?(n/1048576).toFixed(1)+' MB':n>1024?Math.round(n/1024)+' KB':n+' B'}
function setContentState(text, loadingState = false){
    content.innerHTML=loadingState
        ?`<div class="empty"><span class="loading-indicator" aria-hidden="true"></span>${esc(text)}</div>`
        :`<div class="empty">${esc(text)}</div>`;
}
function copyText(v){if(navigator.clipboard){navigator.clipboard.writeText(v||'');}flash('Copied')}
async function api(path,opts){
    const r=await fetch(path,opts);
    if(r.status===401){statusEl.textContent='Login required';throw new Error('auth');}
    if(!r.ok)throw new Error(await r.text());
    return r.json();
}
function actionIds(){return checked.size?[...checked]:(selected?[selected]:[]);}
function setActionState(){
    const ids=actionIds();
    const items=ids.map(id=>messages.find(m=>m.id===id)).filter(Boolean);
    replyMsg.disabled=!selected;
    markRead.disabled=!items.some(m=>m.unread);
    markUnread.disabled=!items.some(m=>!m.unread);
    deleteMsg.disabled=!items.length;
}
function filteredMessages(){
    if(!searchEl)return messages;
    const q=searchEl.value.trim().toLowerCase();
    if(!q)return messages;
    return messages.filter(m=>[m.from,m.to,m.subject,m.preview,m.date].some(v=>(v||'').toLowerCase().includes(q)));
}
function updateDraftStore(){
    localStorage.setItem('webmailDraft',JSON.stringify({to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments}));
}
function restoreDraft(){
    try{
        const raw=localStorage.getItem('webmailDraft');
        if(!raw)return;
        const draft=JSON.parse(raw);
        toEl.value=draft.to||'';
        subjectEl.value=draft.subject||'';
        bodyEl.value=draft.body||'';
        draftAttachments=Array.isArray(draft.attachments)?draft.attachments:[];
        renderDraftAttachments();
    }catch(e){}
}
function clearDraft(){
    toEl.value=subjectEl.value=bodyEl.value='';
    draftAttachments=[];
    renderDraftAttachments();
    localStorage.removeItem('webmailDraft');
}
function openCompose(){
    composeEl.classList.add('open');
    toEl.focus();
}
function closeCompose(){
    if(toEl.value||subjectEl.value||bodyEl.value||draftAttachments.length){updateDraftStore();}
    composeEl.classList.remove('open');
}
function renderList(){
    const unread=messages.filter(m=>m.unread).length;
    document.title=(unread?`(${unread}) `:'')+'Edgerun Mail';
    mailCount.textContent=`${messages.length} total · ${unread} unread`;
    const visible=filteredMessages();
    const rows=visible.map(m=>{
        const hasSelection=checked.has(m.id)?' checked':'';
        const unreadClass=m.unread?' unread':'';
        const pendingClass=m.pending?' pending':'';
        const activeClass=m.id===selected?' active':'';
        return `<div class="item${unreadClass}${pendingClass}${activeClass}${hasSelection}" data-id="${encodeURIComponent(m.id)}"><input class="pick" type="checkbox" aria-label="Select message" ${checked.has(m.id)?'checked':''}><div class="from">${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="13" height="13"><use href="#i-alert"/></svg></span>':''}${esc(m.from||'(unknown)')}${m.attachmentCount?'<span class="paperclip" title="'+m.attachmentCount+' attachment(s)"><svg><use href="#i-paperclip"/></svg></span>':''}</div><div class="subject">${esc(m.subject)}</div><div class="preview">${esc(m.preview)}</div></div>`;
    });
    messagesEl.innerHTML=rows.join('')||`<div class="empty">${messages.length?'No matches':'No mail'}</div>`;
    setActionState();
}
async function load(silent){
    if(loading)return;
    loading=true;
    refreshBtn.classList.add('spin');
    if(!messages.length)setContentState('Loading mailbox…', true);
    try{
        const data=await api('/api/messages');
        const pending=new Map(messages.filter(m=>m.pending).map(m=>[m.id,m]));
        messages=data.messages.map(m=>pending.get(m.id)||m);
        if(selected&&!messages.some(m=>m.id===selected)){
            selected='';
            openedMessage=null;
            content.innerHTML='<div class="empty">Select a message</div>';
        }
        renderList();
        if(!silent)flash('Updated');
    }catch(e){
        if(!silent)flash(e.message,true);
    }finally{
        loading=false;
        refreshBtn.classList.remove('spin');
    }
}
messagesEl.addEventListener('click', e=>{
    const item=e.target.closest('.item');
    if(!item)return;
    const id=decodeURIComponent(item.dataset.id);
    if(e.target.closest('.pick')){
        checked.has(id)?checked.delete(id):checked.add(id);
        renderList();
        return;
    }
    openMsg(id);
});

function renderMailAttachments(attachments){
    if(!attachments.length)return '';
    return `<div class="attachments">${attachments.map(a=>`<a class="attachment" href="/api/message/${encodeURIComponent(openedMessage?.id||'')}/attachment/${a.index}" download="${esc(a.name)}"><svg width="15" height="15"><use href="#i-paperclip"/></svg>${esc(a.name)} <span>${fmtSize(a.size)}</span></a>`).join('')}</div>`;
}
async function openMsg(id){
    selected=id;
    openedMessage=null;
    const local=messages.find(m=>m.id===id);
    if(local&&local.unread){
        local.unread=false;
        renderList();
        api('/api/message/'+encodeURIComponent(id)+'/read',{method:'POST'}).catch(()=>{local.unread=true;renderList();flash('Could not mark read',true);});
    }else{
        renderList();
    }
    setContentState('Loading message…', true);
    try{
        const m=await api('/api/message/'+encodeURIComponent(id));
        if(selected!==id)return;
        openedMessage=m;
        const attachments=(m.attachments||[]).map(a=>`<a class="attachment" href="/api/message/${encodeURIComponent(id)}/attachment/${a.index}" download="${esc(a.name)}"><svg width="15" height="15"><use href="#i-paperclip"/></svg>${esc(a.name)} <span>${fmtSize(a.size)}</span></a>`).join('');
        content.innerHTML=`<h1>${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="18" height="18"><use href="#i-alert"/></svg></span>':''}${esc(m.subject)}</h1>${m.warning?'<div class="warning"><svg width="16" height="16"><use href="#i-alert"/></svg> '+esc(m.warningReason)+'</div>':''}<div class="meta"><div class="meta-line">From: <span>${esc(m.from)}</span><button class="icon copy" data-copy="${esc(m.from)}" title="Copy sender" aria-label="Copy sender"><svg><use href="#i-copy"/></svg></button></div><div class="meta-line">To: <span>${esc(m.to)}</span><button class="icon copy" data-copy="${esc(m.to)}" title="Copy recipient" aria-label="Copy recipient"><svg><use href="#i-copy"/></svg></button></div><div>${esc(m.date)}</div></div>${attachments?'<div class="attachments">'+attachments+'</div>':''}<pre>${esc(m.body||m.raw)}</pre>`;
        setActionState();
    }catch(e){
        content.innerHTML='<div class="empty">Could not open message</div>';
        flash(e.message,true);
    }
}

document.addEventListener('click', e=>{
    const b=e.target.closest('.copy');
    if(b)copyText(b.dataset.copy||'');
});
refreshBtn.onclick=()=>load(false);
if(searchEl)searchEl.oninput=renderList;
composeBtn.onclick=()=>openCompose();
replyMsg.onclick=async()=>{
    let m=openedMessage;
    if(!m&&selected)m=await api('/api/message/'+encodeURIComponent(selected));
    if(!m)return;
    toEl.value=(m.from||'').replace(/^.*<([^>]+)>.*$/,'$1');
    subjectEl.value=/^re:/i.test(m.subject||'')?m.subject:'Re: '+(m.subject||'');
    bodyEl.value=`\n\nOn ${m.date||'the original message'}, ${m.from||'sender'} wrote:\n`+(m.body||'').split('\n').map(line=>'> '+line).join('\n');
    draftAttachments=[];
    renderDraftAttachments();
    updateDraftStore();
    openCompose();
};
cancelBtn.onclick=closeCompose;

async function action(name){
    const ids=actionIds();
    if(!ids.length)return;
    const old=messages.map(m=>({...m}));
    if(name==='delete'){
        messages=messages.filter(m=>!ids.includes(m.id));
        checked.clear();
        selected='';
        content.innerHTML='<div class="empty">Select a message</div>';
    }else{
        messages.forEach(m=>{if(ids.includes(m.id)){m.pending=true;m.unread=name==='unread';}});
    }
    renderList();
    try{
        await Promise.all(ids.map(id=>api('/api/message/'+encodeURIComponent(id)+'/'+name,{method:'POST'})));
        flash(name==='delete'?'Deleted':name==='read'?'Marked read':'Marked unread');
        load(true);
    }catch(e){
        messages=old;
        renderList();
        flash(e.message,true);
    }
}
markRead.onclick=()=>action('read');
markUnread.onclick=()=>action('unread');
deleteMsg.onclick=()=>{
    const n=actionIds().length;
    if(n&&confirm('Delete '+n+' message'+(n>1?'s':'')+'?'))action('delete');
};
logout.onclick=async()=>{
    selected='';
    content.innerHTML='<div class="empty">Logged out</div>';
    messagesEl.innerHTML='';
    statusEl.textContent='Logged out';
    try{await fetch('/api/messages',{headers:{Authorization:'Basic '+btoa('logout:logout')}})}catch(e){}
};

function renderDraftAttachments(){
    composeAttachments.innerHTML=draftAttachments.map((a,i)=>`<span class="chip"><svg width="13" height="13"><use href="#i-paperclip"/></svg> ${esc(a.name)} ${fmtSize(a.size)} <button type="button" data-idx="${i}" title="Remove attachment" aria-label="Remove attachment">&times;</button></span>`).join('');
}
attachBtn.onclick=()=>filePick.click();
composeAttachments.onclick=e=>{
    const b=e.target.closest('button');
    if(!b)return;
    draftAttachments.splice(Number(b.dataset.idx),1);
    renderDraftAttachments();
    updateDraftStore();
};
filePick.onchange=async()=>{
    for(const file of filePick.files){
        const data=await new Promise((ok,bad)=>{
            const r=new FileReader();
            r.onload=()=>ok(String(r.result).split(',')[1]||'');
            r.onerror=bad;
            r.readAsDataURL(file);
        });
        draftAttachments.push({name:file.name,contentType:file.type||'application/octet-stream',size:file.size,data});
    }
    filePick.value='';
    renderDraftAttachments();
    updateDraftStore();
    flash('Attached');
};
[toEl,subjectEl,bodyEl].forEach(el=>el.addEventListener('input',updateDraftStore));

composeEl.onsubmit=async e=>{
    e.preventDefault();
    if(sending)return;
    sending=true;
    sendBtn.disabled=true;
    sendBtn.classList.add('spin');
    const draft={to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments.map(({name,contentType,data})=>({name,contentType,data}))};
    flash('Sending...');
    composeEl.classList.remove('open');
    try{
        await api('/api/send',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(draft)});
        clearDraft();
        flash('Sent');
        load(true);
    }catch(err){
        composeEl.classList.add('open');
        flash(err.message,true);
    }finally{
        sending=false;
        sendBtn.disabled=false;
        sendBtn.classList.remove('spin');
    }
};

restoreDraft();
load(false).catch(err=>flash(err.message,true));
setInterval(()=>{if(!document.hidden)load(true)},10000);
