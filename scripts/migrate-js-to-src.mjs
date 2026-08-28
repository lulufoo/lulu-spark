import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync, cpSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

const repoRoot = process.cwd();
const jsRoot = join(repoRoot, 'frontend/js');
const srcRoot = join(repoRoot, 'frontend/src');

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) out.push(...walk(abs));
    else out.push(abs);
  }
  return out;
}

function isReexportStub(abs) {
  const text = readFileSync(abs, 'utf8').trim();
  if (!text.startsWith('export')) return false;
  return /from\s+['"][^'"]*\/src\/[^'"]+['"]/.test(text) || /from\s+['"]\.\.\/\.\.\/src\//.test(text);
}

const copied = [];
for (const abs of walk(jsRoot)) {
  const rel = relative(jsRoot, abs);
  if (rel === 'main.js') continue;
  if (isReexportStub(abs)) continue;
  const dest = join(srcRoot, rel);
  if (existsSync(dest)) {
    console.log('skip existing', rel);
    continue;
  }
  mkdirSync(dirname(dest), { recursive: true });
  cpSync(abs, dest);
  copied.push(rel);
}

cpSync(join(jsRoot, 'main.js'), join(srcRoot, 'boot.js'));
copied.push('boot.js (from main.js)');

let boot = readFileSync(join(srcRoot, 'boot.js'), 'utf8');
boot = boot.replace("from '../src/hash-router.tsx'", "from './hash-router.tsx'");
writeFileSync(join(srcRoot, 'boot.js'), boot);

const toastDest = join(srcRoot, 'app-shell/toast.js');
if (!existsSync(toastDest)) {
  mkdirSync(dirname(toastDest), { recursive: true });
  writeFileSync(toastDest, "export { showToast } from '../toast.tsx';\n");
}

const hubShim = join(srcRoot, 'home-entry-shell/hub.js');
if (!existsSync(hubShim)) {
  mkdirSync(dirname(hubShim), { recursive: true });
  writeFileSync(hubShim, "export { mountHomeHub } from '../home/hub.js';\n");
}

const chatShim = join(srcRoot, 'home-entry-shell/chat-render.js');
if (!existsSync(chatShim)) {
  writeFileSync(
    chatShim,
    "export { hydrateHomeChatMarkdown, renderHomeChatMarkdown } from '../home/chat-render.ts';\n",
  );
}

console.log('copied', copied.length, 'files');
copied.forEach((f) => console.log(' ', f));
