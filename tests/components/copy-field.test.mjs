import { test } from 'node:test';
import assert from 'node:assert/strict';
import { install } from '../../packages/vanilla/components/copy-field/interaction.js';

function fixture(writeText) {
  const listeners = new Map();
  const labels = new Map([['data-cui-copy-label-idle','Copy value'],['data-cui-copy-label-copied','Copied'],['data-cui-copy-label-failed','Copy failed; select the value.']]);
  const button = { hidden: true, disabled: false, isConnected: true, getAttribute: key => labels.get(key), setAttribute: (key, value) => labels.set(key,value), closest: () => component };
  const input = {value:'TASK-42',isConnected:true};
  const status = {textContent:''};
  const component = {isConnected:true, querySelector: selector => selector.includes('source')?input:selector.includes('trigger')?button:status, dispatchEvent() {}};
  const root = {defaultView:{navigator:{clipboard:writeText?{writeText}:undefined}},querySelectorAll:()=>[component],addEventListener:(name,handler)=>listeners.set(name,handler),removeEventListener:name=>listeners.delete(name)};
  const click = () => listeners.get('click')?.({target:{closest:()=>button},preventDefault(){}});
  return {root,button,input,status,component,click,listeners};
}
test('no clipboard capability keeps the selectable no-JS fallback and installs no listeners',()=>{
  const f=fixture();const stop=install(f.root);assert.equal(f.button.hidden,true);assert.equal(f.listeners.size,0);stop();
});
test('successful copy uses the visible input and localized feedback, teardown resets progressive control',async()=>{
  const values=[];const f=fixture(async value=>values.push(value));const stop=install(f.root);
  assert.equal(f.button.hidden,false);await f.click();assert.deepEqual(values,['TASK-42']);assert.equal(f.status.textContent,'Copied');
  stop();assert.equal(f.button.hidden,true);assert.equal(f.status.textContent,'');assert.equal(f.listeners.size,0);
});
test('rejection keeps the native value and announces recovery',async()=>{
  const f=fixture(async()=>{throw new Error('denied');});const stop=install(f.root);await f.click();assert.equal(f.input.value,'TASK-42');assert.equal(f.status.textContent,'Copy failed; select the value.');stop();
});
test('pending clicks are coalesced and late completion cannot mutate after teardown',async()=>{
  let resolve,calls=0;const f=fixture(()=>{calls++;return new Promise(r=>resolve=r);});const stop=install(f.root);
  const first=f.click();await f.click();assert.equal(calls,1);stop();resolve();await first;assert.equal(f.status.textContent,'');assert.equal(f.button.hidden,true);
});
test('replacing only the clipboard input cannot claim the new visible value was copied',async()=>{
  let resolve;const f=fixture(()=>new Promise(r=>resolve=r));const stop=install(f.root);
  const pending=f.click();const original=f.component.querySelector;
  const replacement={value:'TASK-43',isConnected:true};f.input.isConnected=false;
  f.component.querySelector=selector=>selector.includes('source')?replacement:original(selector);
  resolve();await pending;assert.equal(f.status.textContent,'');assert.equal(f.button.disabled,false);assert.equal(f.button.getAttribute('aria-label'),'Copy value');stop();
});
test('leaving the installed root cannot receive pending clipboard feedback',async()=>{
  let resolve,owned=true;const f=fixture(()=>new Promise(r=>resolve=r));f.root.contains=()=>owned;const stop=install(f.root);
  const pending=f.click();owned=false;resolve();await pending;assert.equal(f.status.textContent,'');stop();
});
test('replaced or edited clipboard source cannot receive a stale success claim',async()=>{
  let resolve;const f=fixture(()=>new Promise(r=>resolve=r));const stop=install(f.root);const pending=f.click();f.input.value='TASK-43';resolve();await pending;assert.equal(f.status.textContent,'');stop();
  const g=fixture(()=>new Promise(r=>resolve=r));const end=install(g.root);const detached=g.click();g.component.isConnected=false;resolve();await detached;assert.equal(g.status.textContent,'');end();
});
