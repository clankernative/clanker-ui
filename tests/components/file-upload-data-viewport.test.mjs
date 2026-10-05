import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { install as installUpload } from '../../packages/vanilla/components/file-upload/interaction.js';
import { install as installViewport } from '../../packages/vanilla/components/data-viewport/interaction.js';

class Element extends EventTarget {
  constructor(attrs = {}) { super(); this.attrs={...attrs};this.children=[];this._textContent='';this.hidden=false;this.isConnected=true;this.scrollTop=0;this.clientHeight=240;this.ownerDocument={defaultView:globalThis,createElement:()=>new Element()}; }
  get childNodes(){return this.children;}
  get textContent(){return this._textContent===undefined?this.children.map(child=>child.textContent).join(''):this._textContent;}
  set textContent(value){this.children=[];this._textContent=String(value);}
  getAttribute(name){return this.attrs[name]??null;} setAttribute(name,value){this.attrs[name]=String(value);} removeAttribute(name){delete this.attrs[name];}
  matches(selector){if(selector==='[data-cui-component="file-upload"]')return this.attrs['data-cui-component']==='file-upload';if(selector==='[data-cui-component="data-viewport"]')return this.attrs['data-cui-component']==='data-viewport';if(selector==='[data-cui-viewport-navigation]')return Object.hasOwn(this.attrs,'data-cui-viewport-navigation');return false;}
  querySelector(selector){return this.parts?.[selector]??null;}
  querySelectorAll(selector){return this.descendants?.filter(x=>x.matches(selector))??[];}
  contains(node){return node===this||this.descendants?.includes(node)||Object.values(this.parts??{}).includes(node);}
  closest(selector){for(let n=this;n;n=n.parentElement)if(n.matches?.(selector))return n;return null;}
  replaceChildren(...children){this.children=children;this._textContent=undefined;}
  toggleAttribute(name,on){if(on)this.setAttribute(name,'');else this.removeAttribute(name);}
  send(type,target=this,extra={}){const event={type,target,preventDefault(){this.defaultPrevented=true;},...extra};for(let n=target;n;n=n.parentElement)for(const fn of n._listeners?.get(type)??[])fn(event);return event;}
  addEventListener(type,fn,options){super.addEventListener(type,fn,options);this._listeners??=new Map();const list=this._listeners.get(type)??[];list.push(fn);this._listeners.set(type,list);}
  removeEventListener(type,fn,options){super.removeEventListener(type,fn,options);const list=this._listeners?.get(type)??[];this._listeners?.set(type,list.filter(x=>x!==fn));}
}
class Root extends Element {
  constructor(nodes=[]){super();this.nodes=nodes;for(const n of nodes)n.parentElement=this;this.ownerDocument={defaultView:globalThis,createElement:()=>new Element()};}
  querySelectorAll(selector){return this.nodes.flatMap(n=>[...(n.matches(selector)?[n]:[]),...n.querySelectorAll(selector)]);}
  contains(node){return this.nodes.some(n=>n===node||n.contains(node));}
  send(type,target,extra={}){return super.send(type,target,extra);}
}
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function uploadFixture(asRoot=false){
  const c=new Element({'data-cui-component':'file-upload'}),input=new Element();input.files=[];input.multiple=true;input.disabled=false;input.parentElement=c;
  const list=new Element(),feedback=new Element({'data-cui-selection-error-label':'Selection adapter failed.','data-cui-cancel-error-label':'Cancel adapter failed.'});list.hidden=true;feedback.hidden=true;
  c.parts={'[data-cui-file-upload-input]':input,'[data-cui-file-upload-local-selection]':list,'[data-cui-file-upload-local-feedback]':feedback};c.descendants=[input,list,feedback];for(const e of c.descendants)e.parentElement=c;
  const root=asRoot?c:new Root([c]);return{root,c,input,list,feedback};
}
function viewportFixture({windowed=false}={}){
  const attrs={'data-cui-component':'data-viewport'};
  if(windowed)Object.assign(attrs,{'data-cui-window-total':'100','data-cui-window-item-size':'40','data-cui-window-overscan':'2','data-cui-window-start':'0'});
  const c=new Element(attrs),scroller=new Element();scroller.parentElement=c;
  const status=new Element({'data-cui-window-error-label':'Window request failed.','data-cui-navigation-error-label':'Navigation failed.'});status.hidden=true;status.parentElement=c;
  const control=new Element({'data-cui-viewport-navigation':''});control.parentElement=c;
  c.parts={'[data-cui-scroll]':scroller,'[data-cui-viewport-status]':status};c.descendants=[scroller,status,control];
  const root=new Root([c]);return{root,c,scroller,status,control};
}

const fakeObservers=[];
class FakeMutationObserver { constructor(callback){this.callback=callback;this.disconnected=false;fakeObservers.push(this);} observe(){} disconnect(){this.disconnected=true;} fire(){if(!this.disconnected)this.callback([]);} }

test('file adapter receives original File/input/component and a per-specimen lifetime signal; filename region remains separate',async()=>{
  const f=uploadFixture(),nativeFile={name:'report.pdf',size:9},seen=[];
  const stop=installUpload(f.root,{selection:request=>seen.push(request)});
  f.input.files=[nativeFile];f.root.send('change',f.input);await tick();
  assert.equal(seen.length,1);assert.equal(seen[0].files[0],nativeFile);assert.equal(seen[0].input,f.input);assert.equal(seen[0].component,f.c);assert.equal(seen[0].signal.aborted,false);
  assert.equal(f.list.children[0].textContent,'report.pdf');assert.equal(f.feedback.hidden,true);stop();
  assert.equal(f.list.children.length,0);assert.equal(seen[0].signal.aborted,true);
});
test('old upload rejection cannot replace newer success; feedback cleanup preserves app-owned writes',async()=>{
  const f=uploadFixture();let rejectFirst,resolveSecond,calls=0;
  const stop=installUpload(f.root,{selection(){if(++calls===1)return new Promise((_,reject)=>rejectFirst=reject);if(calls===2)return new Promise(resolve=>resolveSecond=resolve);throw Error('third handoff rejected');}});
  f.input.files=[{name:'first.txt'}];f.root.send('change',f.input);await tick();
  f.input.files=[{name:'second.txt'}];f.root.send('change',f.input);await tick();resolveSecond();await tick();
  rejectFirst(Error('stale'));await tick();assert.equal(f.feedback.hidden,true);assert.equal(f.list.children[0].textContent,'second.txt');
  f.input.files=[{name:'third.txt'}];f.root.send('change',f.input);await tick();assert.equal(f.feedback.textContent,'Selection adapter failed.');
  f.feedback.textContent='Selection adapter failed.';f.feedback.hidden=false;stop();assert.equal(f.feedback.textContent,'Selection adapter failed.');assert.equal(f.feedback.hidden,false);
});
test('chooser cancel errors are visible and cleanup aborts late work without mutating detached specimens',async()=>{
  const f=uploadFixture();let request,release;
  const stop=installUpload(f.root,{selection:r=>{request=r;return new Promise(resolve=>release=resolve);},cancel:r=>{assert.equal(r.files.length,0);throw Error('private detail');}});
  f.input.files=[{name:'wait.txt'}];f.root.send('change',f.input);f.root.send('cancel',f.input);await tick();assert.equal(f.feedback.textContent,'Cancel adapter failed.');
  f.root.nodes=[];f.c.parentElement=null;f.c.isConnected=false;stop();assert.equal(request.signal.aborted,true);release();await tick();assert.equal(f.feedback.textContent,'Cancel adapter failed.');
});
test('replacing the native input aborts its specimen lease and rejects stale completion',async()=>{
  const previous=globalThis.MutationObserver;globalThis.MutationObserver=FakeMutationObserver;
  try{
    const f=uploadFixture();let request,reject;
    const stop=installUpload(f.root,{selection:r=>{request=r;return new Promise((_,fail)=>reject=fail);}});
    f.input.files=[{name:'old.txt'}];f.root.send('change',f.input);await tick();
    const replacement=new Element();replacement.files=[];replacement.multiple=true;replacement.disabled=false;replacement.parentElement=f.c;
    f.c.parts['[data-cui-file-upload-input]']=replacement;f.c.descendants[0]=replacement;
    fakeObservers.at(-1).fire();assert.equal(request.signal.aborted,true);reject(Error('late'));
    await tick();assert.equal(f.feedback.hidden,true);assert.equal(f.list.children.length,0);stop();
  }finally{if(previous===undefined)delete globalThis.MutationObserver;else globalThis.MutationObserver=previous;}
});
test('detached root-self file specimen aborts its lease even without a parent observer',async()=>{
  const f=uploadFixture(true);let request,resolve;
  const stop=installUpload(f.c,{selection:r=>{request=r;return new Promise(done=>resolve=done);}});
  f.input.files=[{name:'pending.txt'}];f.c.send('change',f.input);await tick();f.c.isConnected=false;resolve();await tick();
  assert.equal(request.signal.aborted,true);assert.equal(f.feedback.textContent,'');stop();
});
test('ancestor and root-self installs never double-own a file specimen; lease transfers after cleanup',async()=>{
  const f=uploadFixture(true),ancestor=new Root([f.c]);let ownCalls=0,ancestorCalls=0;
  const stopSelf=installUpload(f.c,{selection(){ownCalls++;}}),stopAncestor=installUpload(ancestor,{selection(){ancestorCalls++;}});
  f.input.files=[{name:'one.txt'}];ancestor.send('change',f.input);await tick();assert.equal(ownCalls,1);assert.equal(ancestorCalls,0);
  stopSelf();ancestor.send('change',f.input);await tick();assert.equal(ancestorCalls,1);stopAncestor();
});
test('MutationObserver reconciliation leases newly admitted file components and rejects unknown adapter keys',()=>{
  const previous=globalThis.MutationObserver;globalThis.MutationObserver=FakeMutationObserver;
  try{
    const root=new Root(),calls=[];const stop=installUpload(root,{selection:r=>calls.push(r.component)});
    const f=uploadFixture(true);f.c.parentElement=root;root.nodes.push(f.c);fakeObservers.at(-1).fire();f.input.files=[{name:'added.txt'}];root.send('change',f.input);assert.deepEqual(calls,[f.c]);stop();
    assert.throws(()=>installUpload(root,{selection(){},fetch(){}}),TypeError);
  }finally{if(previous===undefined)delete globalThis.MutationObserver;else globalThis.MutationObserver=previous;}
});

test('window scrolling deduplicates bounded metadata requests and latest failure owns its feedback',async()=>{
  const f=viewportFixture({windowed:true}),requests=[];let rejectOld,resolveNew,calls=0;
  const stop=installViewport(f.root,{window(request){requests.push(request);if(++calls===1)return new Promise((_,reject)=>rejectOld=reject);return new Promise(resolve=>resolveNew=resolve);}});
  f.scroller.scrollTop=320;f.root.send('scroll',f.scroller);await tick();
  assert.deepEqual([requests[0].start,requests[0].end,requests[0].total],[6,16,100]);assert.equal(requests[0].component,f.c);assert.equal(requests[0].scroller,f.scroller);assert.equal(requests[0].signal.aborted,false);
  f.scroller.scrollTop=400;f.root.send('scroll',f.scroller);await tick();resolveNew();await tick();rejectOld(Error('stale'));await tick();assert.equal(f.status.hidden,true);
  f.root.send('scroll',f.scroller);await tick();assert.equal(requests.length,2);stop();assert.equal(requests[0].signal.aborted,true);
});
test('paged navigation adapter receives marked native control context without window attrs or default prevention',async()=>{
  const f=viewportFixture(),requests=[];const stop=installViewport(f.root,{navigate:request=>requests.push(request)});
  const event=f.root.send('click',f.control);await tick();assert.equal(event.defaultPrevented,undefined);assert.equal(requests.length,1);
  assert.equal(requests[0].component,f.c);assert.equal(requests[0].control,f.control);assert.equal(requests[0].signal.aborted,false);assert.equal('window'in requests[0],false);stop();
});
test('removed viewport is not mutated by stale callback and app-owned status survives cleanup',async()=>{
  const f=viewportFixture({windowed:true});let reject;const stop=installViewport(f.root,{window:()=>new Promise((_,r)=>reject=r)});
  f.scroller.scrollTop=200;f.root.send('scroll',f.scroller);await tick();f.root.nodes=[];f.c.parentElement=null;
  f.status.textContent='App-owned error';f.status.hidden=false;stop();reject(Error('late'));await tick();
  assert.equal(f.status.textContent,'App-owned error');assert.equal(f.status.hidden,false);
});
test('viewport MutationObserver releases replaced specimens and aborts pending adapters',async()=>{
  const previous=globalThis.MutationObserver;globalThis.MutationObserver=FakeMutationObserver;
  try{
    const f=viewportFixture({windowed:true});let request,resolve;
    const stop=installViewport(f.root,{window:r=>{request=r;return new Promise(done=>resolve=done);}});
    f.scroller.scrollTop=200;f.root.send('scroll',f.scroller);await tick();f.root.nodes=[];f.c.parentElement=null;f.c.isConnected=false;fakeObservers.at(-1).fire();
    assert.equal(request.signal.aborted,true);resolve();await tick();assert.equal(f.status.textContent,'');stop();
  }finally{if(previous===undefined)delete globalThis.MutationObserver;else globalThis.MutationObserver=previous;}
});
test('detached root-self viewport aborts its specimen signal without an ancestor observer',async()=>{
  const f=viewportFixture({windowed:true});let request,resolve;
  const stop=installViewport(f.c,{window:r=>{request=r;return new Promise(done=>resolve=done);}});
  f.scroller.scrollTop=200;f.c.send('scroll',f.scroller);await tick();f.c.isConnected=false;resolve();await tick();
  assert.equal(request.signal.aborted,true);assert.equal(f.status.textContent,'');stop();
});
test('viewport adapter options are closed and missing adapter is a no-op',()=>{
  const f=viewportFixture();assert.doesNotThrow(()=>installViewport(f.root));assert.throws(()=>installViewport(f.root,{query(){}}),TypeError);
});
test('hidden enhancement rows stay hidden without JavaScript and fixed window rows match spacer units',async()=>{
  const uploadCss=await readFile(new URL('../../packages/vanilla/components/file-upload/styles.css',import.meta.url),'utf8');
  const viewportCss=await readFile(new URL('../../packages/vanilla/components/data-viewport/styles.css',import.meta.url),'utf8');
  assert.match(uploadCss,/\.cui-file-upload__local-selection\[hidden\].*display:none!important/);
  assert.match(viewportCss,/\.cui-data-viewport--windowed \.cui-data-viewport__items\{gap:0;min-block-size:0;padding:0\}/);
  assert.match(viewportCss,/\.cui-data-viewport--windowed \.cui-data-viewport__item\{box-sizing:border-box;block-size:var\(--cui-data-viewport-item-size\);overflow:auto/);
  assert.match(viewportCss,/\.cui-data-viewport--windowed \.cui-data-viewport__cell-content\{box-sizing:border-box;block-size:var\(--cui-data-viewport-item-size\);overflow:auto/);
});
