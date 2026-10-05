import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source = await readFile(new URL('../../packages/vanilla/browser/lifecycle.js',import.meta.url),'utf8');
const {install,captureScrollPosition,restoreScrollPosition,preserveScroll} = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
function root() {
  const value=new EventTarget(); value.defaultView=new EventTarget(); value.defaultView.CustomEvent=CustomEvent; return value;
}
function dispatch(view,name,persisted=false) { const event=new Event(name);Object.defineProperty(event,'persisted',{value:persisted});view.dispatchEvent(event); }
test('installation is exact-root idempotent and cleanup reverses owners once',()=>{
  const value=root(),calls=[];
  const one=()=>{calls.push('one');return()=>calls.push('-one');};
  const two=()=>{calls.push('two');return()=>calls.push('-two');};
  const stop=install(value,[one,two]); assert.equal(install(value,[one,two]),stop);
  assert.throws(()=>install(value,[one]),/different/);
  stop();stop();assert.deepEqual(calls,['one','two','-two','-one']);
  install(value,[one])();assert.equal(calls.length,6);
});
test('failed installation rolls back earlier owners and keeps original failure',()=>{
  const value=root(),calls=[];
  const first=()=>()=>{calls.push('cleanup');throw new Error('cleanup failure');};
  assert.throws(()=>install(value,[first,()=>{throw new Error('initial failure');}]),/initial failure/);
  assert.deepEqual(calls,['cleanup']); install(value,[])();
  assert.throws(()=>install(root(),[()=>null]),/return cleanup/);
});
test('BFCache keeps the instance; ordinary pagehide releases it',()=>{
  const value=root(),observations=[];let clean=0;
  value.addEventListener('cui:observation',e=>observations.push(e.detail));
  const start=()=>()=>{clean++;}; const stop=install(value,[start]);
  dispatch(value.defaultView,'pagehide',true);assert.equal(clean,0);
  dispatch(value.defaultView,'pageshow',true);assert.equal(install(value,[start]),stop);
  dispatch(value.defaultView,'pagehide',false);assert.equal(clean,1);
  assert.deepEqual(observations.map(v=>v.name),['runtime.install','page.suspended','page.restored','page.suspended','runtime.stop']);
  assert.ok(observations.every(v=>Object.keys(v).sort().join(',')==='name,outcome,schemaVersion,scope'));
});
function scrollOwner() {
  const value=new EventTarget();value.scrollTop=100;value.scrollLeft=-20;value.isConnected=true;
  const body={};let focused=0;
  const input={id:'draft',focus:options=>{focused++;assert.equal(options.preventScroll,true);}};
  const anchor={id:'row-a',position:130,getBoundingClientRect:()=>({top:anchor.position-value.scrollTop,bottom:anchor.position-value.scrollTop+30})};
  value.ownerDocument={body,activeElement:input,getElementById:id=>id==='row-a'?anchor:id==='draft'?input:null};
  value.contains=e=>e===input||e===anchor;
  value.getBoundingClientRect=()=>({top:10,bottom:110});value.querySelectorAll=()=>[anchor];
  return {value,anchor,input,body,focused:()=>focused};
}
test('anchor and focus restoration preserve pixel offset and negative RTL offset',()=>{
  const {value,anchor,body,focused}=scrollOwner();const snap=captureScrollPosition(value);
  assert.deepEqual(snap,{top:100,left:-20,anchor:{id:'row-a',offset:20},focus:'draft'});
  anchor.position+=40;value.scrollTop=0;value.scrollLeft=0;value.ownerDocument.activeElement=body;
  restoreScrollPosition(value,snap);assert.equal(value.scrollTop,140);assert.equal(value.scrollLeft,-20);assert.equal(focused(),1);
  value.ownerDocument.activeElement={id:'other-user-focus'};restoreScrollPosition(value,snap);assert.equal(focused(),1);
  restoreScrollPosition(value,{top:NaN,left:0});assert.equal(value.scrollTop,140);
});
test('host-supplied synchronous and async updates retain their result and errors',async()=>{
  const {value,anchor}=scrollOwner();assert.equal(preserveScroll(value,()=>{anchor.position+=40;return 7;}),7);assert.equal(value.scrollTop,140);
  assert.equal(await preserveScroll(value,async()=>{anchor.position+=20;return 8;}),8);assert.equal(value.scrollTop,160);
  assert.throws(()=>preserveScroll(value,()=>{throw new Error('failed');}),/failed/);
  await assert.rejects(preserveScroll(value,async()=>{throw new Error('rejected');}),/rejected/);
});
test('pending updates cannot overwrite continued user scrolling or detached roots',async()=>{
  const {value,anchor}=scrollOwner();let done;
  const pending=preserveScroll(value,()=>new Promise(resolve=>{done=resolve;}));
  value.dispatchEvent(new Event('wheel'));value.scrollTop=180;anchor.position+=100;done(9);
  assert.equal(await pending,9);assert.equal(value.scrollTop,180);
  value.isConnected=false;await preserveScroll(value,async()=>{value.scrollTop=230;anchor.position+=100;});assert.equal(value.scrollTop,230);
});
