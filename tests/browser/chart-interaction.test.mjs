import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
const source=await readFile(new URL('../../packages/vanilla/components/chart/interaction.js',import.meta.url),'utf8');
const {rangeFor,rangeFromCoordinates,nearest,readPoints,install}=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
// Deliberately small DOM adapter: lifecycle evidence, not browser acceptance.
class Node {
  constructor(document){this.ownerDocument=document;this.listeners=new Map();this.attrs=new Map();this.children=[];this.isConnected=true;this.dataset={};this.classList={add(){}};}
  addEventListener(name,fn){if(!this.listeners.has(name))this.listeners.set(name,new Set());this.listeners.get(name).add(fn);}
  removeEventListener(name,fn){this.listeners.get(name)?.delete(fn);}
  dispatchEvent(event){event.target??=this;for(const fn of [...this.listeners.get(event.type)??[]])fn(event);if(event.bubbles)this.parent?.dispatchEvent(event);return !event.defaultPrevented;}
  event(type,fields={}){const e={type,bubbles:true,defaultPrevented:false,preventDefault(){this.defaultPrevented=true;},...fields};this.dispatchEvent(e);return e;}
  setAttribute(key,value){this.attrs.set(key,String(value));}getAttribute(key){return this.attrs.get(key)??null;}removeAttribute(key){this.attrs.delete(key);}
  append(...nodes){for(const n of nodes){n.parent=this;this.children.push(n);}}
  after(node){this.parent.append(node);}
  remove(){this.isConnected=false;if(this.parent)this.parent.children=this.parent.children.filter(n=>n!==this);}
  contains(node){return node===this||this.children.some(child=>child.contains(node));}
  focus(){const previous=this.ownerDocument.activeElement;if(previous&&previous!==this)previous.event('focusout',{relatedTarget:this});this.ownerDocument.activeElement=this;this.event('focus',{bubbles:false});this.event('focusin');}
  matches(){return false;}querySelectorAll(){return [];}querySelector(){return null;}
}
function fixture(points=[{time:10,value:0,missing:false,x:10,y:100},{time:20,value:99,missing:true,x:20,y:0},{time:30,value:75,missing:false,x:30,y:25}]){
 const document=new Node(null);document.ownerDocument=null;document.body=new Node(document);document.activeElement=document.body;
 const view=new Node(document);document.defaultView=view;
 view.CustomEvent=class {constructor(type,options={}){Object.assign(this,{type,...options});}};
 const observers=[];view.MutationObserver=class {constructor(fn){this.fn=fn;observers.push(this);}observe(){}disconnect(){this.off=true;}trigger(){if(!this.off)this.fn();}};
 document.createElement=document.createElementNS=()=>new Node(document);
 function root(){
  const r=new Node(document);r.id='sample-chart';r.dataset={start:'0',end:'40'};r.parent=document;
  const svg=new Node(document);svg.setAttribute('viewBox','0 0 640 240');svg.setAttribute('aria-hidden','true');svg.setAttribute('role','img');r.append(svg);
  const plot=new Node(document);plot.setAttribute('x','0');plot.setAttribute('width','40');plot.setAttribute('y','10');plot.setAttribute('height','190');svg.querySelector=()=>plot;
  svg.getBoundingClientRect=()=>({left:0,width:640});svg.getScreenCTM=()=>({inverse:()=>({})});svg.createSVGPoint=()=>({matrixTransform(){return {x:this.x-100};}});
  const captures=new Set();svg.setPointerCapture=id=>captures.add(id);svg.releasePointerCapture=id=>{if(captures.delete(id))svg.event('lostpointercapture',{pointerId:id});};
  r.matches=()=>true;r.svg=svg;r.pointNodes=points.map((p,index)=>({dataset:Object.fromEntries(Object.entries({key:'sample-'+index,...p}).map(([k,v])=>[k,String(v)]))}));
  r.querySelectorAll=()=>r.pointNodes;r.querySelector=s=>s==='svg'?svg:s==='figcaption'?{textContent:'Samples'}:null;
  r.toolbar=()=>r.children.find(n=>n.className==='cui-chart__controls');return r;
 }
 let roots=[root()];document.querySelectorAll=()=>roots;const events=[];
 for(const name of ['range-committed','range-cancelled','point-activated','unmounted'])document.addEventListener('cui-chart:'+name,e=>events.push(e));
 return {document,view,events,observers,get root(){return roots[0];},replace(){roots[0].isConnected=false;document.activeElement=document.body;roots=[root()];observers.forEach(o=>o.trigger());}};
}
test('mapping distinguishes missing from measured zero and rejects unbounded or malformed points',()=>{
 const f=fixture(),data=readPoints(f.root);assert.equal(data.points[0].value,0);assert.equal(data.points[0].missing,false);assert.equal(data.points[1].missing,true);assert.equal(data.points[1].y,null);
 f.root.pointNodes[1].dataset.time='10';assert.equal(readPoints(f.root),null);
 f.root.pointNodes=Array(129).fill(f.root.pointNodes[0]);assert.equal(readPoints(f.root),null);
 assert.equal(nearest([{x:10},{x:20}],15),0);assert.equal(nearest([],10),-1);
});
test('range commitment is half-open, ordered and bounded by the accepted range',()=>{
 const data=readPoints(fixture().root);assert.deepEqual(rangeFor(data.points,2,0,0,40),{start:10,end:31});
 assert.deepEqual(rangeFor(data.points,2,2,0,30+1),{start:30,end:31});
 assert.equal(rangeFor(data.points,-1,0,0,40),null);assert.equal(rangeFor(data.points,0,3,0,40),null);
});
test('pointer ranges are continuous, half-open and clipped, independent of sample spacing',()=>{
 assert.deepEqual(rangeFromCoordinates(13,17,0,40,0,40),{start:13,end:17});
 assert.deepEqual(rangeFromCoordinates(17,13,0,40,0,40),{start:13,end:17});
 assert.deepEqual(rangeFromCoordinates(-100,200,0,40,0,40),{start:0,end:40});
 assert.equal(rangeFromCoordinates(15,15,0,40,0,40),null);
 assert.equal(rangeFromCoordinates(NaN,15,0,40,0,40),null);
 assert.equal(rangeFromCoordinates(10,15,0,40,0,0),null);
 for(let n=0;n<256;n++){
  const a=(n*37)%90-20,b=(n*53)%90-20;
  const r=rangeFromCoordinates(a,b,1735689600000,1736294400000,0,40);
  if(r){assert.ok(Number.isSafeInteger(r.start)&&Number.isSafeInteger(r.end));assert.ok(r.start>=1735689600000&&r.start<r.end&&r.end<=1736294400000);}
 }
});
test('sparse samples do not snap pointer selection, SVG defaults are suppressed, and matrix fallback works',()=>{
 const f=fixture(),stop=install(f.document),svg=f.root.svg;
 const down=svg.event('pointerdown',{button:0,pointerId:3,clientX:113,clientY:20});assert.equal(down.defaultPrevented,true);
 const move=svg.event('pointermove',{pointerId:3,clientX:121,clientY:20});assert.equal(move.defaultPrevented,true);assert.equal(f.events.length,0);
 svg.event('pointerup',{pointerId:3,clientX:121,clientY:20});assert.deepEqual(f.events.at(-1).detail,{start:13,end:21});
 svg.getScreenCTM=()=>null;svg.event('pointerdown',{button:0,pointerId:4,clientX:14,clientY:20});svg.event('pointermove',{pointerId:4,clientX:23,clientY:20});svg.event('pointerup',{pointerId:4,clientX:23,clientY:20});assert.deepEqual(f.events.at(-1).detail,{start:14,end:23});stop();
});
test('keyboard and non-drag controls emit only bounded committed events; browser shortcuts remain untouched',()=>{
 const f=fixture(),stop=install(f.document),svg=f.root.svg;assert.equal(install(f.document),stop);svg.focus();
 const shortcut=svg.event('keydown',{key:'ArrowLeft',altKey:true});assert.equal(shortcut.defaultPrevented,false);
 svg.event('keydown',{key:'ArrowRight',shiftKey:true});assert.equal(f.events.length,0);
 const toolbar=f.root.toolbar();toolbar.children.find(n=>n.textContent==='Apply selection').event('click');
 assert.deepEqual(f.events.at(-1).detail,{start:10,end:21});assert.ok(Object.isFrozen(f.events.at(-1).detail));
 svg.event('keydown',{key:'Enter'});assert.deepEqual(f.events.at(-1).detail,{key:'sample-1',time:20,value:99,missing:true});
 stop();stop();assert.equal(svg.getAttribute('aria-hidden'),'true');assert.equal(svg.getAttribute('tabindex'),null);assert.equal(svg.listeners.get('keydown').size,0);
});
test('pointer coordinates use the actual SVG matrix; preview is local and cancellation releases capture exactly once',()=>{
 const f=fixture(),stop=install(f.document),svg=f.root.svg;
 svg.event('pointerdown',{button:0,pointerId:1,clientX:110,clientY:20});svg.event('pointermove',{pointerId:1,clientX:130,clientY:20});
 assert.equal(f.events.length,0);svg.event('pointerup',{pointerId:1,clientX:130,clientY:20});assert.deepEqual(f.events.at(-1).detail,{start:10,end:30});
 const before=f.events.length;svg.event('pointerdown',{button:0,pointerId:2,clientX:110,clientY:20});svg.event('pointercancel',{pointerId:2});
 assert.equal(f.events.length,before+1);assert.equal(f.events.at(-1).type,'cui-chart:range-cancelled');stop();
});
test('unchanged-data native morph repairs removed controls and accessibility attributes',()=>{
 const f=fixture(),stop=install(f.document),svg=f.root.svg;svg.focus();const oldToolbar=f.root.toolbar();oldToolbar.remove();svg.setAttribute('role','img');svg.setAttribute('aria-hidden','true');svg.removeAttribute('tabindex');
 f.observers.forEach(o=>o.trigger());assert.equal(svg.getAttribute('role'),'group');assert.equal(svg.getAttribute('aria-hidden'),'false');assert.equal(svg.getAttribute('tabindex'),'0');assert.equal(svg.listeners.get('keydown').size,1);assert.ok(f.root.toolbar());assert.notEqual(f.root.toolbar(),oldToolbar);stop();
});
test('same-ID replacement removes exact-root handlers and restores chart focus without duplicating controls',()=>{
 const f=fixture(),stop=install(f.document),old=f.root.svg;f.root.toolbar().children[2].focus();f.replace();
  assert.equal(old.listeners.get('keydown').size,0);assert.equal(f.document.activeElement,f.root.svg);assert.equal(f.root.children.filter(n=>n.className==='cui-chart__controls').length,1);
  f.observers.forEach(o=>o.trigger());assert.equal(f.root.children.filter(n=>n.className==='cui-chart__controls').length,1);stop();
 assert.ok(f.observers.every(o=>o.off));assert.equal(f.view.listeners.get('pagehide').size,0);
});

test('seeded drag, morph, replacement and teardown schedules preserve exact-root ownership',()=>{
 function replay(seed){
  const f=fixture(),transcript=[];let stop=install(f.document),random=seed,drag=false,commits=0;
  const draft={value:'84'};f.document.commandDraft=draft;
  for(let step=0;step<64;step++){
   random=(Math.imul(random,1664525)+1013904223)>>>0;
   const action=(random>>>8)%7,svg=f.root.svg,trace=`seed=${seed} step=${step} action=${action}`;
   switch(action){
    case 0: svg.event('pointerdown',{button:0,pointerId:1,clientX:110,clientY:20});drag=true;break;
    case 1: svg.event('pointermove',{pointerId:1,clientX:130,clientY:20});break;
    case 2: svg.event('pointerup',{pointerId:1,clientX:130,clientY:20});if(drag)commits++;drag=false;break;
    case 3: svg.event('pointercancel',{pointerId:1});drag=false;break;
    case 4: f.root.toolbar().children[2].focus();f.replace();drag=false;break;
    case 5: f.root.toolbar().remove();svg.setAttribute('aria-hidden','true');f.observers.forEach(o=>o.trigger());drag=false;break;
    case 6: stop();stop();assert.equal(svg.listeners.get('keydown').size,0,trace);stop=install(f.document);drag=false;break;
   }
   const actual=f.events.filter(e=>e.type==='cui-chart:range-committed');
   assert.equal(actual.length,commits,trace);
   for(const event of actual)assert.deepEqual(event.detail,{start:10,end:30},trace);
   assert.equal(f.root.svg.listeners.get('keydown').size,1,trace);
   assert.equal(f.root.children.filter(n=>n.className==='cui-chart__controls').length,1,trace);
   assert.equal(f.view.listeners.get('pagehide').size,2,trace); // One installer and one mounted root.
   assert.equal(f.document.commandDraft,draft,trace);assert.equal(draft.value,'84',trace);
   transcript.push([action,commits,f.events.map(e=>[e.type,e.detail])]);
  }
  stop();assert.ok(f.observers.every(o=>o.off));assert.equal(f.view.listeners.get('pagehide').size,0);
  return transcript;
 }
 for(let seed=1;seed<=32;seed++)assert.deepEqual(replay(seed),replay(seed),`seed=${seed}`);
});
