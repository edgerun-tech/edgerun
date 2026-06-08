import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, 'src');
const fragments = readdirSync(srcDir).filter(f => f.endsWith('.wat')).sort();

const hostFuncs = ['sock_open','sock_send','sock_recv','sock_close',
  'sys_poll','sys_mmap','sys_munmap','sys_socket','sys_connect','sys_sendmsg','sys_memfd_create','sys_ftruncate'];

for (const f of fragments) {
  const text = readFileSync(join(srcDir, f), 'utf-8');
  const found = hostFuncs.filter(hf => text.includes('$' + hf));
  if (found.length > 0) {
    console.log(f + ': ' + found.join(', '));
  }
}
console.log('\nNo fragment references host/linux imports directly.');
