import { createTerminalDrawer } from './components/terminalDrawer.js';
import { fetchStatus, runTask } from './services/api.js';

const TASKS = [
  { id: 'doctor', label: 'Doctor', group: 'Core' },
  { id: 'setup', label: 'Setup', group: 'Core' },
  { id: 'build-workspace', label: 'Build Workspace', group: 'Build' },
  { id: 'test-runtime', label: 'Test Runtime', group: 'Test' },
  { id: 'ci-all', label: 'CI All', group: 'CI' },
];

function cardHtml(meta, data) {
  return `<div class="card">
    <div class="task">${meta.label}</div>
    <div>${data.task}</div>
    <div>state: ${data.state}</div>
    <div>runs: ${data.runs}</div>
    <div>exit: ${data.last_exit ?? '-'}</div>
    <button data-run="${data.task}">Run</button>
    <pre>${(data.last_output || '').slice(-2000)}</pre>
  </div>`;
}

async function refresh() {
  const summaryEl = document.getElementById('summary');
  const tasksEl = document.getElementById('tasks');
  try {
    const body = await fetchStatus();
    const map = new Map(body.tasks.map((t) => [t.task, t]));
    const merged = TASKS.map((meta) => ({
      meta,
      task: map.get(meta.id) || { task: meta.id, state: 'idle', runs: 0, last_output: '' },
    }));
    summaryEl.textContent = `visible=${merged.length}`;
    tasksEl.innerHTML = merged.map((x) => cardHtml(x.meta, x.task)).join('');
  } catch (err) {
    summaryEl.textContent = `status fetch failed: ${String(err.message || err)}`;
    tasksEl.innerHTML = '';
  }
}

document.addEventListener('click', async (ev) => {
  const target = ev.target;
  if (!(target instanceof HTMLElement)) return;
  const task = target.dataset.run;
  if (!task) return;
  await runTask(task);
  await refresh();
});

const drawer = createTerminalDrawer();
drawer.mount();

setInterval(refresh, 2000);
refresh();
