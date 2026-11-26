// The published site in a real browser: large view of the media, and no video wrongly shown as missing.
// Needs Node 22+ (WebSocket) and Chrome (CHROME=/path/to/chrome to choose it).
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, rmSync, createReadStream, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, extname, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const docs = resolve(dirname(fileURLToPath(import.meta.url)), '../../docs');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.jpg': 'image/jpeg', '.mp4': 'video/mp4', '.svg': 'image/svg+xml', '.ttf': 'font/ttf' };
// Served as if still to come: every media of the site is there now.
const withheld = /plugin-database-diagram(-en)?\.png$/;
const server = createServer((req, res) => {
  const file = join(docs, new URL(req.url, 'http://x').pathname);
  try {
    if (!file.startsWith(docs) || withheld.test(file) || !statSync(file).isFile()) throw new Error('not a file');
    res.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream' });
    createReadStream(file).pipe(res);
  } catch { res.writeHead(404); res.end(); }
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/`;

const profile = mkdtempSync(join(tmpdir(), 'site-test-'));
const chrome = spawn(process.env.CHROME || 'google-chrome', ['--headless=new', '--disable-gpu', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const sleep = ms => new Promise(r => setTimeout(r, ms));
try {
  const port = await new Promise((ok, ko) => {
    chrome.stderr.on('data', d => { const m = String(d).match(/DevTools listening on ws:\/\/[^:]+:(\d+)/); if (m) ok(m[1]); });
    chrome.on('error', ko);
  });
  let target;
  for (let i = 0; i < 50 && !target; i++) { target = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find(t => t.type === 'page'); if (!target) await sleep(200); }
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise(r => { ws.onopen = r; });
  let id = 0; const wait = new Map(), errors = [];
  ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id) wait.get(m.id)(m.result); if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text); };
  const send = (method, params = {}) => new Promise(r => { wait.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const js = async x => (await send('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
  const go = async p => { await send('Page.navigate', { url: base + p }); await sleep(2000); };
  const click = async sel => {
    const [x, y] = await js(`(()=>{const e=document.querySelector(${JSON.stringify(sel)});e.scrollIntoView({block:'center',behavior:'instant'});const b=e.getBoundingClientRect();return [b.x+b.width/2,b.y+b.height/2]})()`);
    await sleep(500);
    for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
    await sleep(700);
  };
  const key = async (k, code) => {
    for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: k, code: k, windowsVirtualKeyCode: code, text: type === 'keyDown' && k === 'Enter' ? '\r' : undefined });
    await sleep(500);
  };
  const view = () => js(`(()=>{const d=document.querySelector('dialog.zoom'),m=d.querySelector('.zoom-body>*');return {open:d.open,real:d.classList.contains('real'),tag:m?m.tagName:null,src:m?(m.currentSrc||m.src).split('/').pop():null,width:m?Math.round(m.getBoundingClientRect().width):0,controls:!!(m&&m.controls),close:d.querySelector('.zoom-close').getAttribute('aria-label')}})()`);

  await send('Runtime.enable'); await send('Page.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1366, height: 768, deviceScaleFactor: 1, mobile: false });
  await go('plugin-git.html');
  await js(`localStorage.setItem('ardoise-lang','fr')`);

  // A video that loads is never replaced by "to come", whatever the loading order.
  for (let i = 0; i < 4; i++) {
    await go('plugin-git.html');
    assert.equal(await js(`document.querySelectorAll('.slot.missing-fr, .slot.missing-en').length`), 0, 'media wrongly shown as missing');
  }

  const image = '.slot:has(img[src="media/plugin-git-commit.png"]) .win-body';
  await click(image);
  let v = await view();
  assert.deepEqual([v.open, v.tag, v.src, v.close], [true, 'IMG', 'plugin-git-commit.png', 'Fermer']);
  await click('.zoom-body>img');
  assert.equal((await view()).width, 1600, 'second click shows the real size');
  await key('Escape', 27);
  assert.equal((await view()).open, false, 'Escape closes');

  await click('.slot .win-body:has(video)');
  v = await view();
  assert.deepEqual([v.open, v.tag, v.controls], [true, 'VIDEO', true]);
  await click('.zoom-close');
  assert.equal((await view()).open, false, 'the button closes');

  await js(`document.querySelector(${JSON.stringify(image)}).focus()`);
  await key('Enter', 13);
  assert.equal((await view()).open, true, 'Enter opens');
  await key('Escape', 27);

  await js(`setLang('en')`); await sleep(1000);
  await click(image);
  v = await view();
  assert.deepEqual([v.src, v.close], ['plugin-git-commit-en.png', 'Close']);
  await key('Escape', 27);

  await go('index.html');
  await click('.hero-media .win-body');
  assert.equal((await view()).src, 'hero-en.png');
  await key('Escape', 27);

  // A media still to come does not open.
  await go('plugin-database.html');
  const pending = '.slot:has(img[src="media/plugin-database-diagram.png"])';
  await js(`document.querySelector(${JSON.stringify(pending)}).scrollIntoView({block:'center',behavior:'instant'})`); await sleep(1500);
  await click(pending + ' .win-body');
  assert.equal((await view()).open, false);

  assert.deepEqual(errors, []);
  ws.close();
  console.log('browser tests: ok');
} finally {
  chrome.kill(); server.close(); await sleep(500);
  rmSync(profile, { recursive: true, force: true, maxRetries: 5 });
}
