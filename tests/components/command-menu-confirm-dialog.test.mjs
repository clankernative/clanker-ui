// Isolated fakes verify component control flow, not browser-native dialog/form conformance.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const load = async (name) => (await import(`data:text/javascript;base64,${Buffer.from(await readFile(new URL(`../../packages/vanilla/components/${name}/interaction.js`, import.meta.url))).toString('base64')}`)).install;
class Root {
  listeners = new Map(); components = [];
  addEventListener(t, f) { this.listeners.set(t, [...(this.listeners.get(t) || []), f]); }
  removeEventListener(t, f) { this.listeners.set(t, (this.listeners.get(t) || []).filter(x => x !== f)); }
  emit(t, e) { for (const f of this.listeners.get(t) || []) f(e); }
  querySelectorAll(s) { return s.includes('data-cui-command-shortcut') ? this.components.filter(c => c.selector === '[data-cui-component="command-menu"]') : this.components.flatMap(c => c.querySelectorAll(s)); }
  contains(n) { return !!n && n.isConnected !== false; }
}
class El {
  constructor(selector) { this.selector = selector; this.isConnected = true; this.attrs = {}; this.hidden = false; this.listeners = new Map(); this.value = ''; this.focusCount = 0; }
  closest(s) { return this.selector === s ? this : this.parent?.closest(s) || null; }
  matches(s) { return this.selector === s; }
  getAttribute(k) { return this.attrs[k] ?? null; }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  removeAttribute(k) { delete this.attrs[k]; }
  remove() { if (this.parent?.children) this.parent.children = this.parent.children.filter(x => x !== this); }
  toggleAttribute(k, on) { on ? this.setAttribute(k, '') : this.removeAttribute(k); }
  focus() { this.focusCount++; }
  click() { this.clickCount = (this.clickCount || 0) + 1; }
  scrollIntoView() {}
  addEventListener(t, f) { this.listeners.set(t, f); }
  querySelectorAll(s) { return this.children?.filter(x => s.includes('command-item') ? x.selector === '[data-cui-command-item]' : s.includes('command-group') ? x.selector === '[data-cui-command-group]' : false) || []; }
}
const evt = (target, extra = {}) => ({ target, prevented: false, preventDefault() { this.prevented = true; }, ...extra });
function commandFixture() {
  const root = new Root(), component = new El('[data-cui-component="command-menu"]'), trigger = new El('[data-cui-command-trigger]'), dialog = new El('[data-cui-command-dialog]'), search = new El('[data-cui-command-search]'), results = new El('[data-cui-command-results]'), empty = new El('[data-cui-command-empty]'), group = new El('[data-cui-command-group]');
  for (const x of [trigger, dialog, search, results, empty, group]) x.parent = component;
  dialog.open = false; dialog.showModal = function () { this.open = true; }; dialog.close = function () { this.open = false; root.emit('close', { target: this }); };
  component.attrs['data-cui-command-shortcut'] = 'k'; component.querySelector = s => ({ '[data-cui-command-dialog]': dialog, '[data-cui-command-trigger]': trigger, '[data-cui-command-search]': search, '[data-cui-command-empty]': empty }[s] || null);
  const a = new El('[data-cui-command-item]'), b = new El('[data-cui-command-item]'); for (const [i,x] of [a,b].entries()) { x.parent = component; x.attrs['data-cui-command-text'] = i ? 'settings profile' : 'reports analytics'; }
  group.querySelector = () => [a,b].find(x => !x.hidden) || null;
  component.querySelectorAll = s => s.includes('command-dialog') ? [dialog] : s.includes('command-item') ? (s.includes(':not([hidden])') ? [a,b].filter(x => !x.hidden) : [a,b]) : s.includes('command-group') ? [group] : [];
  component.querySelector = ((old) => s => s.includes('data-cui-command-item') ? [a,b].find(x => x.attrs['data-cui-command-active'] !== undefined && !x.hidden) || null : old(s))(component.querySelector);
  root.components = [component]; return { root, component, trigger, dialog, search, empty, a, b };
}
test('command menu retains navigation fallback unless dialog enhancement succeeds; cleanup restores focus', async () => {
  const install = await load('command-menu');
  for (const mutate of [d => d.showModal = null, d => d.showModal = () => { throw Error(); }, d => d.showModal = () => {}]) {
    const x = commandFixture(); mutate(x.dialog); const dispose = install(x.root); const e = evt(x.trigger); x.root.emit('click', e); assert.equal(e.prevented, false); dispose();
  }
  const x = commandFixture(), dispose = install(x.root), e = evt(x.trigger); x.root.emit('click', e); assert.equal(e.prevented, true); assert.equal(x.dialog.open, true); assert.equal(x.search.focusCount, 1); x.dialog.close(); assert.equal(x.trigger.focusCount, 1); dispose();
});
test('command menu filters grouped links, moves active option, enters link, and guards global shortcuts', async () => {
  const install = await load('command-menu'), x = commandFixture(), dispose = install(x.root); x.root.emit('click', evt(x.trigger));
  x.search.value = 'profile'; x.root.emit('input', evt(x.search)); assert.equal(x.a.hidden, true); assert.equal(x.b.hidden, false); assert.equal(x.empty.hidden, true);
  const down = evt(x.search, { key: 'ArrowDown' }); x.root.emit('keydown', down); assert.equal(down.prevented, true); assert.equal(x.b.attrs['data-cui-command-active'], '');
  const enter = evt(x.search, { key: 'Enter' }); x.root.emit('keydown', enter); assert.equal(x.b.clickCount, 1);
  x.dialog.close(); let shortcut = evt(new El('body'), { key: 'k', metaKey: true }); x.root.emit('keydown', shortcut); assert.equal(shortcut.prevented, true);
  shortcut = evt(new El('input'), { key: 'k', ctrlKey: true }); x.root.emit('keydown', shortcut); assert.equal(shortcut.prevented, false);
  shortcut = evt(new El('body'), { key: 'k', ctrlKey: true, isComposing: true }); x.root.emit('keydown', shortcut); assert.equal(shortcut.prevented, false); dispose();
});
function confirmFixture() {
  const root = new Root(), component = new El('[data-cui-component="confirm-dialog"]'), trigger = new El('[data-cui-confirm-trigger]'), dialog = new El('[data-cui-confirm-dialog]'), cancel = new El('[data-cui-confirm-cancel]'), accept = new El('[data-cui-confirm-accept]');
  for (const x of [trigger,dialog,cancel,accept]) x.parent = component;
  const calls=[]; const form={id:'delete-form',requestSubmit(submitter){calls.push(submitter);},checkValidity(){return true;}}; trigger.form=form; trigger.attrs.form=form.id;
  dialog.open=false; dialog.children=[]; dialog.contains=n=>dialog.children.includes(n); dialog.append=n=>{dialog.children.push(n);n.parent=dialog;}; dialog.ownerDocument={createElement:()=>new El('p')};
  dialog.showModal=function(){this.open=true;}; dialog.close=function(){this.open=false;root.emit('close',{target:this});};
  component.querySelector=s=>({'[data-cui-confirm-trigger]':trigger,'[data-cui-confirm-dialog]':dialog}[s]||null); component.querySelectorAll=s=>s.includes('data-cui-confirm-dialog')?[dialog]:[]; root.components=[component]; return {root,component,trigger,dialog,cancel,accept,calls,form};
}
test('command menu install is idempotent and teardown closes only its own dialog without focus restoration', async () => {
  const install=await load('command-menu'),x=commandFixture(),dispose=install(x.root);assert.equal(install(x.root),dispose);x.root.emit('click',evt(x.trigger));dispose();dispose();assert.equal(x.dialog.open,false);assert.equal(x.trigger.focusCount,0);assert.equal((x.root.listeners.get('click')||[]).length,0);
});
test('confirm dialog preserves original submitter, never submits on cancel, and restores focus', async () => {
  const install=await load('confirm-dialog'),x=confirmFixture(),dispose=install(x.root);let e=evt(x.trigger);x.root.emit('click',e);assert.equal(e.prevented,true);assert.equal(x.dialog.open,true);x.root.emit('click',evt(x.accept));assert.deepEqual(x.calls,[x.trigger]);await Promise.resolve();assert.equal(x.dialog.open,false);assert.equal(x.trigger.focusCount,1);
  x.root.emit('click',evt(x.trigger));x.root.emit('click',evt(x.cancel));assert.equal(x.calls.length,1);assert.equal(x.dialog.open,false);dispose();
});
test('invalid form remains native-focusable before opening; newly invalid form closes before native reporting', async () => {
  const install=await load('confirm-dialog'),x=confirmFixture(),dispose=install(x.root);let reports=0;x.form.checkValidity=()=>false;x.form.reportValidity=()=>{reports++;return false;};
  const triggerEvent=evt(x.trigger);x.root.emit('click',triggerEvent);assert.equal(x.dialog.open,false);assert.equal(triggerEvent.prevented,false);assert.equal(reports,1);
  x.form.checkValidity=()=>true;x.root.emit('click',evt(x.trigger));assert.equal(x.dialog.open,true);x.form.checkValidity=()=>false;x.root.emit('click',evt(x.accept));assert.equal(x.dialog.open,false);assert.equal(reports,2);assert.equal(x.trigger.focusCount,0);assert.equal(x.calls.length,0);dispose();
});
test('confirm dialog preserves real submit fallback when association or enhancement API is unavailable', async () => {
  const install=await load('confirm-dialog');for(const change of [x=>x.trigger.form=null,x=>x.dialog.showModal=null,x=>x.form.requestSubmit=null]){const x=confirmFixture();change(x);const dispose=install(x.root);const e=evt(x.trigger);x.root.emit('click',e);assert.equal(e.prevented,false);assert.equal(x.calls.length,0);dispose();}
});
test('confirm dialog app port receives original form and submitter; rejected and canceled intents do not claim success', async () => {
  const install=await load('confirm-dialog'),x=confirmFixture(),requests=[];
  const dispose=install(x.root,{submit(request){requests.push(request);}});
  x.root.emit('click',evt(x.trigger));x.root.emit('click',evt(x.accept));assert.equal(requests.length,1);assert.equal(requests[0].form,x.form);assert.equal(requests[0].submitter,x.trigger);assert.equal(requests[0].signal.aborted,false);await Promise.resolve();assert.equal(x.dialog.open,false);assert.equal(x.calls.length,0);dispose();
  const y=confirmFixture();let reject;const disposeY=install(y.root,{failureLabel:'Could not remove this item.',submit(){return new Promise((_,r)=>reject=r);}});
  y.root.emit('click',evt(y.trigger));y.root.emit('click',evt(y.accept));reject(Error('app port failed'));await Promise.resolve();await Promise.resolve();assert.equal(y.dialog.open,true);assert.equal(y.calls.length,0);assert.equal(y.dialog.children[0].textContent,'Could not remove this item.');assert.equal(y.dialog.children[0].getAttribute('role'),'alert');
  y.root.emit('click',evt(y.accept));assert.equal(y.dialog.children.length,0);y.root.emit('click',evt(y.cancel));assert.equal(y.dialog.open,false);disposeY();
});
test('confirm dialog validates options and label, supports custom adapter without requestSubmit, and aborts stale work on replacement/teardown', async () => {
  const install=await load('confirm-dialog');for(const options of [null, {submit:3},{submit(){},extra:true},{failureLabel:''},{failureLabel:'  '},{failureLabel:'x'.repeat(201)}])assert.throws(()=>install(new Root(),options),TypeError);
  const x=confirmFixture();x.form.requestSubmit=null;let request;const port={submit(value){request=value;}},dispose=install(x.root,port);assert.equal(install(x.root,port),dispose);assert.throws(()=>install(x.root,{submit(){}}),/different options/);x.root.emit('click',evt(x.trigger));assert.equal(x.dialog.open,true);x.root.emit('click',evt(x.accept));assert.ok(request);x.dialog.close();assert.equal(request.signal.aborted,true);assert.equal(x.calls.length,0);assert.equal(x.trigger.focusCount,1);dispose();
  const originalObserver=globalThis.MutationObserver;let observed;
  globalThis.MutationObserver=class{constructor(callback){observed=callback;}observe(){}disconnect(){}};
  try {const y=confirmFixture();let finish,request;const cleanup=install(y.root,{submit(value){request=value;return new Promise(r=>finish=r);}});y.root.emit('click',evt(y.trigger));y.root.emit('click',evt(y.accept));const replacement=new El('[data-cui-confirm-dialog]');replacement.parent=y.component;y.component.querySelector=s=>s==='[data-cui-confirm-dialog]'?replacement:s==='[data-cui-confirm-trigger]'?y.trigger:null;observed();assert.equal(request.signal.aborted,true);assert.equal(y.dialog.children.length,0);finish();await Promise.resolve();assert.equal(y.dialog.open,true);cleanup();assert.equal(y.trigger.focusCount,0);const detached=confirmFixture();let detachedRequest;const cleanupDetached=install(detached.root,{submit(value){detachedRequest=value;return new Promise(()=>{});}});detached.root.emit('click',evt(detached.trigger));detached.root.emit('click',evt(detached.accept));detached.dialog.isConnected=false;observed();assert.equal(detachedRequest.signal.aborted,true);cleanupDetached();} finally {if(originalObserver===undefined)delete globalThis.MutationObserver;else globalThis.MutationObserver=originalObserver;}
  const w=confirmFixture();let pending;const disposeW=install(w.root,{submit(request){pending=request;return new Promise(()=>{});}});w.root.emit('click',evt(w.trigger));w.root.emit('click',evt(w.accept));disposeW();assert.equal(pending.signal.aborted,true);assert.equal(w.dialog.open,false);assert.equal(w.trigger.focusCount,0);
  const z=confirmFixture();let rejectSync=true;const cleanupZ=install(z.root,{submit(){if(rejectSync)throw Error('adapter');}});z.root.emit('click',evt(z.trigger));z.root.emit('click',evt(z.accept));assert.equal(z.dialog.children[0].textContent,'Unable to complete this action. Please try again.');rejectSync=false;z.root.emit('click',evt(z.accept));await Promise.resolve();assert.equal(z.dialog.open,false);cleanupZ();
});
test('confirm dialog publishes the app-owned browser port contract', async () => {
  const declaration=await readFile(new URL('../../packages/vanilla/components/confirm-dialog/browser.d.ts',import.meta.url),'utf8');
  assert.match(declaration,/ConfirmDialogSubmitRequest/);assert.match(declaration,/form: HTMLFormElement/);assert.match(declaration,/submitter: HTMLButtonElement/);assert.match(declaration,/signal: AbortSignal/);assert.match(declaration,/submit\?\(request: ConfirmDialogSubmitRequest\): void \| Promise<void>/);assert.match(declaration,/failureLabel\?: string/);
});
test('complete manifests retain explicit Native gaps and portable CSS accessibility modes', async () => {
  for(const name of ['command-menu','confirm-dialog']){const base=new URL(`../../packages/vanilla/components/${name}/`,import.meta.url),manifest=JSON.parse(await readFile(new URL('component.json',base)));assert.equal(manifest.status,'ready');assert.equal(manifest.integration.native.status,'adapter-required');for(const path of [manifest.assets.template,manifest.assets.styles,...manifest.assets.scripts,...manifest.fixtures])await readFile(new URL(`../../packages/vanilla/${path}`,import.meta.url));assert.ok(manifest.contract.requiredFields.every(x=>/^[a-z][a-z0-9_]*$/.test(x)));const css=await readFile(new URL('styles.css',base),'utf8');assert.match(css,/prefers-reduced-motion/);assert.match(css,/forced-colors/);}
});
