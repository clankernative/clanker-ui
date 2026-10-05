import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source=await readFile(new URL('../../packages/vanilla/components/theme-switcher/install.js',import.meta.url),'utf8');
const styles=await readFile(new URL('../../packages/vanilla/components/theme-switcher/styles.css',import.meta.url),'utf8');
const {install}=await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
class Element extends EventTarget {
  constructor(attrs={}){super();this.attrs=new Map(Object.entries(attrs));this.hidden=true;this.children=[];}
  getAttribute(name){return this.attrs.get(name)??null;}setAttribute(name,value){this.attrs.set(name,String(value));}
  hasAttribute(name){return this.attrs.has(name);}removeAttribute(name){this.attrs.delete(name);}
  contains(value){return value===this||this.children.includes(value);}
  closest(){return this.hasAttribute('data-cui-theme-value')?this:null;}
  matches(){return this.getAttribute('data-cui-component')==='theme-switcher';}
  querySelectorAll(){return this.children;}
}
function fixture({stored,storageThrows=false,mediaMissing=false}={}) {
  const target=new Element({'data-cui-theme':'app-original'}),media=new EventTarget();media.matches=true;
  const view=new EventTarget();view.CustomEvent=CustomEvent;
  let observer;view.MutationObserver=class {constructor(callback){this.callback=callback;observer=this;}observe(){}disconnect(){this.disconnected=true;}};
  const writes=[];view.localStorage={getItem(){if(storageThrows)throw new Error('blocked');return stored??null;},setItem(k,v){if(storageThrows)throw new Error('blocked');writes.push([k,v]);}};
  if(!mediaMissing)view.matchMedia=()=>media;
  const owner=new Element({'data-cui-component':'theme-switcher','data-cui-theme-default':'system','data-cui-theme-system-light':'light','data-cui-theme-system-dark':'dark','data-cui-theme-storage-key':'sample.theme'});
  owner.children=['system','light','dark'].map(value=>new Element({'data-cui-theme-value':value}));
  const root=new EventTarget();root.owners=[owner];root.querySelectorAll=()=>root.owners;
  root.defaultView=view;root.documentElement=target;root.getElementById=id=>id==='preview'?target:null;
  owner.ownerDocument=root;for(const b of owner.children)b.ownerDocument=root;
  const notices=[];owner.addEventListener('cui:theme-changed',e=>notices.push(e.detail));
  return {root,owner,target,media,view,writes,notices,refresh:()=>observer.callback(),observer:()=>observer};
}
function click(owner,button){const event=new Event('click');Object.defineProperty(event,'target',{value:button});owner.dispatchEvent(event);}
test('component CSS keeps hidden options out of layout despite the option display rule',()=>{
  assert.match(styles,/\.cui-theme-switcher \[hidden\]\s*\{\s*display:\s*none\s*!important;/);
});
test('system resolves OS palettes, announces actual resolution, and rechecks BFCache restoration',()=>{
  const f=fixture(),stop=install(f.root);assert.equal(install(f.root),stop);
  assert.equal(f.target.getAttribute('data-cui-theme'),'dark');assert.equal(f.owner.children[0].getAttribute('aria-pressed'),'true');
  f.media.matches=false;f.media.dispatchEvent(new Event('change'));assert.equal(f.target.getAttribute('data-cui-theme'),'light');
  assert.deepEqual(f.notices[0],{theme:'system',resolvedTheme:'light'});
  f.media.matches=true;f.view.dispatchEvent(new Event('pageshow'));assert.equal(f.target.getAttribute('data-cui-theme'),'dark');
  click(f.owner,f.owner.children[1]);assert.equal(f.target.getAttribute('data-cui-theme'),'light');assert.deepEqual(f.writes,[['sample.theme','light']]);
  f.media.dispatchEvent(new Event('change'));assert.equal(f.target.getAttribute('data-cui-theme'),'light');
  stop();stop();assert.equal(f.target.getAttribute('data-cui-theme'),'light');assert.ok(f.owner.children.every(b=>b.hidden));assert.equal(f.observer().disconnected,true);
  f.media.matches=false;f.media.dispatchEvent(new Event('change'));assert.equal(f.target.getAttribute('data-cui-theme'),'light');
});
test('optional storage and media APIs cannot disable direct choices',()=>{
  for(const opts of [{storageThrows:true},{stored:'untrusted'},{mediaMissing:true}]) {
    const f=fixture(opts),stop=install(f.root);assert.ok(f.owner.children.every(b=>!b.hidden));click(f.owner,f.owner.children[2]);assert.equal(f.target.getAttribute('data-cui-theme'),'dark');stop();
  }
  const f=fixture({stored:'light'}),stop=install(f.root);assert.equal(f.target.getAttribute('data-cui-theme'),'light');stop();
});
test('replacement children retain preference and rebind while removed owners release listeners',()=>{
  const f=fixture(),stop=install(f.root);click(f.owner,f.owner.children[1]);
  const old=f.owner.children;f.owner.children=['system','light','dark'].map(value=>new Element({'data-cui-theme-value':value}));
  f.refresh();assert.equal(f.target.getAttribute('data-cui-theme'),'light');assert.equal(f.owner.children[1].getAttribute('aria-pressed'),'true');assert.ok(old.every(b=>b.hidden));
  click(f.owner,f.owner.children[2]);assert.equal(f.target.getAttribute('data-cui-theme'),'dark');
  f.root.owners=[];f.refresh();assert.equal(f.target.getAttribute('data-cui-theme'),'dark');
  click(f.owner,f.owner.children[1]);assert.equal(f.target.getAttribute('data-cui-theme'),'dark');stop();
});
test('only one root installation owns a target and a waiting switcher reclaims it cleanly',()=>{
  const first=fixture(),second=fixture();
  second.target=first.target;
  second.root.documentElement=first.target;
  second.root.getElementById=id=>id==='preview'?first.target:null;
  second.owner.setAttribute('data-cui-theme-target-id','preview');
  const stopFirst=install(first.root),stopSecond=install(second.root);
  assert.ok(first.owner.children.every(button=>!button.hidden));
  assert.ok(second.owner.children.every(button=>button.hidden));
  assert.ok(second.owner.children.every(button=>button.getAttribute('aria-pressed')===null));

  second.media.matches=false;second.media.dispatchEvent(new Event('change'));
  assert.equal(first.target.getAttribute('data-cui-theme'),'dark');
  assert.ok(second.owner.children.every(button=>button.hidden));
  second.media.matches=true;second.media.dispatchEvent(new Event('change'));

  // App ownership wins teardown even when its write equals the current value.
  first.target.setAttribute('data-cui-theme','dark');
  stopFirst();
  assert.equal(first.target.getAttribute('data-cui-theme'),'dark');
  assert.ok(second.owner.children.every(button=>!button.hidden));
  assert.equal(second.owner.children[0].getAttribute('aria-pressed'),'true');
  second.media.matches=false;second.media.dispatchEvent(new Event('change'));
  assert.equal(first.target.getAttribute('data-cui-theme'),'light');
  assert.equal(second.owner.children[0].getAttribute('aria-pressed'),'true');
  stopSecond();
});
test('teardown preserves an app write equal to the last applied theme',()=>{
  const f=fixture();f.target.setAttribute('data-cui-theme','light');const stop=install(f.root);
  assert.equal(f.target.getAttribute('data-cui-theme'),'dark');
  f.target.setAttribute('data-cui-theme','dark');stop();
  assert.equal(f.target.getAttribute('data-cui-theme'),'dark');
});
test('teardown never overwrites a newer app preference; invalid recursive system mapping stays fallback',()=>{
  const f=fixture(),stop=install(f.root);f.target.setAttribute('data-cui-theme','new-app-choice');stop();assert.equal(f.target.getAttribute('data-cui-theme'),'new-app-choice');
  const g=fixture();g.owner.setAttribute('data-cui-theme-system-light','system');const off=install(g.root);assert.equal(g.target.getAttribute('data-cui-theme'),'app-original');assert.ok(g.owner.children.every(b=>b.hidden));off();
});
