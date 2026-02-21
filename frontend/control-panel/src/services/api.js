export async function fetchStatus(token = '') {
  const headers = token ? { 'x-edgerun-token': token } : {};
  const res = await fetch('/api/status', { headers });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `status ${res.status}`);
  }
  return res.json();
}

export async function runTask(task, token = '') {
  const headers = token ? { 'x-edgerun-token': token } : {};
  const res = await fetch(`/api/run/${task}`, { method: 'POST', headers });
  if (!res.ok) throw new Error(`run failed: ${res.status}`);
}
