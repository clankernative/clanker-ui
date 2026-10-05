// Isolated unit fakes exercise module control flow only; they are not browser proofs.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
async function moduleInstall(path) {
  const source=await readFile(new URL(path,import.meta.url),'utf8');
  return (await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`)).install;
}
const installModal=await moduleInstall('../../packages/vanilla/components/modal/interaction.js');
const installDrawer=await moduleInstall('../../packages/vanilla/components/drawer/interaction.js');
const installPopover=await moduleInstall('../../packages/vanilla/components/popover/interaction.js');

class Root {
  listeners = new Map(); dialogs = []; popovers = []; defaultView;
  addEventListener(type, fn) { const list=this.listeners.get(type)||[];list.push(fn);this.listeners.set(type,list); }
  removeEventListener(type, fn) { this.listeners.set(type,(this.listeners.get(type)||[]).filter(x=>x!==fn)); }
  emit(type, event) { for (const fn of this.listeners.get(type)||[]) fn(event); }
  querySelectorAll(selector) { return selector.includes('popover') ? this.popovers.filter(x=>x.open) : this.dialogs; }
  contains(node) { return !!node && node.isConnected !== false; }
}
class Node {
  constructor(selector) { this.selector=selector;this.isConnected=true;this.focusCount=0; }
  closest(selector) { return this.selector===selector?this:(this.closestNode?.matches?.(selector)?this.closestNode:null); }
  focus() { this.focusCount++; }
  matches(selector) { return this.selector===selector; }
}
function overlay(moduleInstall, options={}) {
  const root=new Root(), trigger=new Node('[data-cui-modal-trigger]'), dialog=new Node('[data-cui-modal-dialog]'), component=new Node(moduleInstall===installDrawer?'[data-cui-component="drawer"]':'[data-cui-component="modal"]');
  dialog.open=false;dialog.showCalls=0;dialog.closeCalls=0;
  dialog.showModal=Object.prototype.hasOwnProperty.call(options,'showModal')?options.showModal:function(){this.showCalls++;this.open=true;};
  dialog.close=function(){this.closeCalls++;this.open=false;root.emit('close',{target:this});};
  dialog.matches=(selector)=>selector==='[data-cui-modal-dialog]';
  trigger.closestNode=component;dialog.closestNode=component;component.querySelector=(selector)=>selector==='[data-cui-modal-dialog]'?dialog:selector==='[data-cui-modal-trigger]'?trigger:null;
  root.dialogs=[dialog];const cleanup=moduleInstall(root);return {root,trigger,dialog,component,cleanup};
}
function click(target){let prevented=false;return {target,preventDefault(){prevented=true;},get defaultPrevented(){return prevented;}};}

test('modal and drawer keep fallback when target/API/enhancement is unavailable', async()=>{
  for (const install of [installModal,installDrawer]) {
    const missing=overlay(install);missing.component.querySelector=()=>null;const e=click(missing.trigger);missing.root.emit('click',e);assert.equal(e.defaultPrevented,false);missing.cleanup();
    const unsupported=overlay(install,{showModal:null});const f=click(unsupported.trigger);unsupported.root.emit('click',f);assert.equal(f.defaultPrevented,false);unsupported.cleanup();
    const failed=overlay(install,{showModal(){throw new Error('unsupported');}});const g=click(failed.trigger);failed.root.emit('click',g);assert.equal(g.defaultPrevented,false);failed.cleanup();
    const unopened=overlay(install,{showModal(){}});const h=click(unopened.trigger);unopened.root.emit('click',h);assert.equal(h.defaultPrevented,false);unopened.cleanup();
  }
});
test('modal and drawer installers have disjoint component ownership when both are installed',()=>{
  const modal=overlay(installModal),drawerComponent=new Node('[data-cui-component="drawer"]'),drawerTrigger=new Node('[data-cui-modal-trigger]'),drawerDialog=new Node('[data-cui-modal-dialog]');
  drawerTrigger.closestNode=drawerComponent;drawerDialog.closestNode=drawerComponent;drawerDialog.open=false;drawerDialog.showCalls=0;drawerDialog.showModal=function(){this.showCalls++;this.open=true;};drawerDialog.close=function(){this.open=false;modal.root.emit('close',{target:this});};drawerDialog.matches=s=>s==='[data-cui-modal-dialog]';drawerComponent.querySelector=s=>s==='[data-cui-modal-dialog]'?drawerDialog:s==='[data-cui-modal-trigger]'?drawerTrigger:null;
  modal.root.dialogs.push(drawerDialog);const disposeDrawer=installDrawer(modal.root);
  const first=click(drawerTrigger);modal.root.emit('click',first);assert.equal(first.defaultPrevented,true);assert.equal(drawerDialog.showCalls,1);assert.equal(modal.dialog.showCalls,0);
  const second=click(modal.trigger);modal.root.emit('click',second);assert.equal(second.defaultPrevented,true);assert.equal(modal.dialog.showCalls,1);assert.equal(drawerDialog.showCalls,1);
  disposeDrawer();modal.cleanup();
});
test('modal and drawer only cancel fallback after successful showModal; close restores connected opener once',()=>{
  for (const install of [installModal,installDrawer]) {
    const x=overlay(install);const e=click(x.trigger);x.root.emit('click',e);assert.equal(e.defaultPrevented,true);assert.equal(x.dialog.open,true);
    const again=click(x.trigger);x.root.emit('click',again);assert.equal(again.defaultPrevented,true);assert.equal(x.dialog.showCalls,1);
    x.dialog.close();assert.equal(x.trigger.focusCount,1);x.cleanup();
  }
});
test('close control, backdrop, and native cancel flow preserve close semantics',()=>{
  const x=overlay(installModal);x.root.emit('click',click(x.trigger));
  const close=new Node('[data-cui-modal-close]');close.closestNode=x.component;x.root.emit('click',click(close));assert.equal(x.dialog.open,false);assert.equal(x.trigger.focusCount,1);
  x.root.emit('click',click(x.trigger));const backdrop=click(x.dialog);x.root.emit('click',backdrop);assert.equal(x.dialog.open,false);assert.equal(x.trigger.focusCount,2);
  x.root.emit('click',click(x.trigger));let prevented=false;x.root.emit('cancel',{target:x.dialog,preventDefault(){prevented=true;}});assert.equal(prevented,false);x.dialog.close();assert.equal(x.trigger.focusCount,3);x.cleanup();
});
test('install is idempotent per exact root and teardown closes only its owned dialog without restoring focus',()=>{
  const x=overlay(installModal),dispose=installModal(x.root),again=installModal(x.root);assert.equal(again,dispose);x.root.emit('click',click(x.trigger));dispose();dispose();assert.equal(x.dialog.open,false);assert.equal(x.trigger.focusCount,0);assert.equal((x.root.listeners.get('click')||[]).length,0);
});
test('dialog close does not steal focus from a disconnected opener and teardown closes owned dialogs',()=>{
  const x=overlay(installModal);const e=click(x.trigger);x.root.emit('click',e);x.trigger.isConnected=false;x.dialog.close();assert.equal(x.trigger.focusCount,0);
  const second=overlay(installModal);second.root.emit('click',click(second.trigger));second.cleanup();assert.equal(second.dialog.open,false);assert.equal(second.dialog.closeCalls,1);assert.equal(second.trigger.focusCount,0);
});

class Popover extends Node {
  constructor(){super('details[data-cui-component="popover"]');this.open=true;this.attrs={};this.summary=new Node('summary');this.summary.closestNode=this;this.panel={style:{values:{},setProperty(k,v){this.values[k]=v;},getPropertyValue(k){return this.values[k]||'';}},getBoundingClientRect:()=>{const left=80+(Number.parseFloat(this.panel.style.values['--cui-popover-shift-x'])||0);return {left,right:left+100,width:100,top:60,bottom:120,height:60};}};this.summary.getBoundingClientRect=()=>({left:146,right:196,top:60,bottom:80});this.classList={contains:(name)=>name==='cui-popover--end'};}
  querySelector(sel){return sel==='summary'?this.summary:sel==='.cui-popover__panel'?this.panel:null;}
  setAttribute(k,v){this.attrs[k]=v;}getAttribute(){return null;}contains(target){return target===this||target===this.summary||target===this.panel;}
}
test('popover supports Escape/outside dismissal, focus safety, collision positioning, and cleanup',()=>{
  const root=new Root(),view={innerWidth:200,innerHeight:100,listeners:new Map(),addEventListener(t,f){this.listeners.set(t,f);},removeEventListener(t){this.listeners.delete(t);}};root.defaultView=view;
  const p=new Popover();root.popovers=[p];const dispose=installPopover(root);assert.equal(installPopover(root),dispose);
  root.emit('toggle',{target:p});assert.equal(p.attrs['data-cui-popover-side'],'above');assert.equal(p.panel.style.values['--cui-popover-shift-x'],'12px');root.emit('toggle',{target:p});assert.equal(p.panel.style.values['--cui-popover-shift-x'],'12px');
  let prevented=false;root.emit('keydown',{target:p.summary,key:'Escape',preventDefault(){prevented=true;}});assert.equal(prevented,true);assert.equal(p.open,false);assert.equal(p.summary.focusCount,1);
  p.open=true;root.emit('pointerdown',{target:new Node('outside')});assert.equal(p.open,false);
  p.open=true;dispose();assert.equal(p.open,true);assert.equal(view.listeners.size,0);
});
test('complete manifests close over their assets and retain explicit Native gaps',async()=>{
  for(const name of ['modal','drawer','popover']){const dir=`../../packages/vanilla/components/${name}/`;const manifest=JSON.parse(await readFile(new URL(`${dir}component.json`,import.meta.url),'utf8'));assert.equal(manifest.status,'ready');assert.equal(manifest.integration.native.status,'adapter-required');for(const path of [manifest.assets.template,manifest.assets.styles,...manifest.assets.scripts,...manifest.fixtures])await readFile(new URL(`../../packages/vanilla/${path.replace(/^components\//,'components/')}`,import.meta.url));}
});
test('component styles retain reduced-motion and forced-colors modes',async()=>{
  for(const name of ['modal','drawer','popover']){const css=await readFile(new URL(`../../packages/vanilla/components/${name}/styles.css`,import.meta.url),'utf8');assert.match(css,/prefers-reduced-motion/);assert.match(css,/forced-colors/);}
});
test('popover does not restore focus to a removed summary and tolerates morph removal at teardown',()=>{
  const root=new Root(),p=new Popover();root.popovers=[p];const dispose=installPopover(root);p.summary.isConnected=false;root.emit('keydown',{target:p.summary,key:'Escape',preventDefault(){}});assert.equal(p.summary.focusCount,0);root.popovers=[];dispose();assert.equal(p.open,false);
});
