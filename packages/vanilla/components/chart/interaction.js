// Optional presentation only: bounded events, never queries, routes or commands.
const installed = new WeakMap();
const SVG = 'http://www.w3.org/2000/svg';
const ROOT = '[data-cui-chart][data-cui-chart-enhance="true"]';

export function rangeFor(points, first, last, start, end) {
  if (![first,last,start,end].every(Number.isSafeInteger) || start >= end ||
      first < 0 || last < 0 || first >= points.length || last >= points.length) return null;
  const left = Math.min(points[first].time, points[last].time);
  const right = Math.min(end, Math.max(points[first].time, points[last].time) + 1);
  return start <= left && left < right && right <= end ? Object.freeze({start:left,end:right}) : null;
}
// Pointer ranges follow the time axis, not the spacing of recorded samples.
export function rangeFromCoordinates(first, last, start, end, plotX, plotWidth) {
  if (![first,last,plotX,plotWidth].every(Number.isFinite) || plotWidth<=0 ||
      ![start,end,end-start].every(Number.isSafeInteger) || start>=end) return null;
  const time=x=>Math.round(start+(end-start)*Math.max(0,Math.min(1,(x-plotX)/plotWidth)));
  const a=time(first),b=time(last),left=Math.min(a,b),right=Math.max(a,b);
  return left<right ? Object.freeze({start:left,end:right}) : null;
}
export function nearest(points, x) {
  if (!points.length || !Number.isFinite(x)) return -1;
  let best = 0;
  for (let i=1;i<points.length;i++) if (Math.abs(points[i].x-x) < Math.abs(points[best].x-x)) best=i;
  return best;
}
function integer(value) {
  return /^-?\d+$/.test(value ?? '') && Number.isSafeInteger(Number(value)) ? Number(value) : null;
}
function decimal(value) {
  return /^-?\d+(?:\.\d+)?$/.test(value ?? '') && Number.isFinite(Number(value)) ? Number(value) : null;
}
export function readPoints(root) {
  const start=integer(root.dataset.start), end=integer(root.dataset.end);
  if (start === null || end === null || start >= end) return null;
  const nodes=[...root.querySelectorAll('[data-cui-chart-point]')];
  if (nodes.length > 128) return null;
  const points=[], keys=new Set();
  for (const node of nodes) {
    const {key,time,value,missing,x,y}=node.dataset;
    const t=integer(time), v=integer(value), px=decimal(x), py=missing==='true'?null:decimal(y);
    if (!key || key.length > 80 || /[\u0000-\u001f\u007f]/u.test(key) || keys.has(key) ||
        t===null || v===null || px===null || Math.abs(px)>2048 ||
        (missing!=='true' && missing!=='false') || (missing==='false' && (py===null || Math.abs(py)>1024)) ||
        t<start || t>=end || (points.length && points.at(-1).time>=t)) return null;
    keys.add(key); points.push(Object.freeze({key,time:t,value:v,missing:missing==='true',x:px,y:py}));
  }
  return Object.freeze({start,end,points:Object.freeze(points)});
}
function signature(root) {
  return JSON.stringify([root.dataset.start,root.dataset.end,
    root.querySelector('svg')?.getAttribute('viewBox'),
    [...root.querySelectorAll('[data-cui-chart-point]')].map(n=>[
      n.dataset.key,n.dataset.time,n.dataset.value,n.dataset.missing,n.dataset.x,n.dataset.y])]);
}
function mount(root, restoreFocus) {
  const data=readPoints(root), svg=root.querySelector('svg');
  if (!data || !svg || !data.points.length) return {stop(){},focused(){return false;}};
  const view=root.ownerDocument.defaultView, document=root.ownerDocument;
  const box=(svg.getAttribute('viewBox')??'').trim().split(/\s+/).map(Number);
  if (box.length!==4 || box.some(v=>!Number.isFinite(v)) || box[2]<=0 || box[3]<=0 || box[2]>2048 || box[3]>1024) return {stop(){},focused(){return false;}};
  const saved=new Map(['tabindex','role','aria-hidden','aria-label'].map(n=>[n,svg.getAttribute(n)]));
  svg.setAttribute('tabindex','0');svg.setAttribute('role','group');svg.setAttribute('aria-hidden','false');
  svg.setAttribute('aria-label',`${root.querySelector('figcaption')?.textContent??'Chart'}. Arrow keys browse samples. Enter inspects. Shift and arrows select a range. Escape cancels.`);
  const layer=document.createElementNS(SVG,'g');layer.setAttribute('aria-hidden','true');layer.classList.add('cui-chart__enhancement');
  const preview=document.createElementNS(SVG,'rect');preview.classList.add('cui-chart__selection');preview.setAttribute('visibility','hidden');
  const crosshair=document.createElementNS(SVG,'line');crosshair.classList.add('cui-chart__crosshair');crosshair.setAttribute('visibility','hidden');
  const marker=document.createElementNS(SVG,'circle');marker.classList.add('cui-chart__marker');marker.setAttribute('r','4');marker.setAttribute('visibility','hidden');
  layer.append(preview,crosshair,marker);svg.append(layer);
  const toolbar=document.createElement('div');toolbar.className='cui-chart__controls';toolbar.setAttribute('aria-label','Chart interaction');
  const status=document.createElement('output');status.className='cui-chart__status';status.setAttribute('aria-live','polite');
  const listeners=[];let current=0, anchor=null, pointerSelection=null, drag=null, stopped=false, focused=root.contains(document.activeElement);
  const plot=svg.querySelector('defs clipPath rect');
  const plotX=decimal(plot?.getAttribute('x'))??box[0], plotW=decimal(plot?.getAttribute('width'))??box[2];
  const plotY=decimal(plot?.getAttribute('y'))??0, plotH=decimal(plot?.getAttribute('height'))??box[3];
  const clampX=value=>Math.max(plotX,Math.min(plotX+plotW,value));
  function listen(node,name,fn,options){node.addEventListener(name,fn,options);listeners.push(()=>node.removeEventListener(name,fn,options));}
  function emit(name,detail){if(!stopped&&root.isConnected)root.dispatchEvent(new view.CustomEvent(name,{bubbles:true,detail:Object.freeze(detail)}));}
  function label(point){return `${new Date(point.time).toISOString()} · ${point.missing?'Missing':point.value}`;}
  function draw(){
    const p=data.points[current];crosshair.setAttribute('x1',p.x);crosshair.setAttribute('x2',p.x);crosshair.setAttribute('y1',plotY);crosshair.setAttribute('y2',plotY+plotH);crosshair.setAttribute('visibility','visible');
    marker.setAttribute('cx',p.x);if(!p.missing)marker.setAttribute('cy',p.y);marker.setAttribute('visibility',p.missing?'hidden':'visible');
    status.textContent=label(p);
    if(pointerSelection){
      const {first,last}=pointerSelection,range=rangeFromCoordinates(first,last,data.start,data.end,plotX,plotW);
      preview.setAttribute('x',Math.min(first,last));preview.setAttribute('y',plotY);preview.setAttribute('width',Math.abs(first-last));preview.setAttribute('height',plotH);preview.setAttribute('visibility','visible');
      status.textContent=range?`Selected time range: ${new Date(range.start).toISOString()} to ${new Date(range.end).toISOString()} (end exclusive). Apply selection to request fresh data.`:'Choose a wider time range.';
    }else if(anchor!==null){const a=data.points[anchor];preview.setAttribute('x',Math.min(a.x,p.x));preview.setAttribute('y',plotY);preview.setAttribute('width',Math.abs(a.x-p.x));preview.setAttribute('height',plotH);preview.setAttribute('visibility','visible');status.textContent=`Selected samples: ${label(a)} to ${label(p)}. Apply range to request fresh data.`;}
    else preview.setAttribute('visibility','hidden');
    apply.disabled=pointerSelection ? !rangeFromCoordinates(pointerSelection.first,pointerSelection.last,data.start,data.end,plotX,plotW) : anchor===null;
  }
  function activate(){const {key,time,value,missing}=data.points[current];emit('cui-chart:point-activated',{key,time,value,missing});}
  function commit(){const range=pointerSelection?rangeFromCoordinates(pointerSelection.first,pointerSelection.last,data.start,data.end,plotX,plotW):rangeFor(data.points,anchor,current,data.start,data.end);if(range)emit('cui-chart:range-committed',range);}
  function cancel(){if(drag){const pointer=drag.pointer;drag=null;try{svg.releasePointerCapture(pointer);}catch{}}anchor=null;pointerSelection=null;draw();emit('cui-chart:range-cancelled',{});}
  function move(delta){pointerSelection=null;current=Math.max(0,Math.min(data.points.length-1,current+delta));draw();svg.focus({preventScroll:true});}
  function button(text,fn){const b=document.createElement('button');b.type='button';b.textContent=text;listen(b,'click',fn);toolbar.append(b);return b;}
  button('Previous sample',()=>move(-1));button('Next sample',()=>move(1));button('Inspect sample',activate);
  button('Start selection',()=>{pointerSelection=null;anchor=current;draw();svg.focus({preventScroll:true});});const apply=button('Apply selection',commit);button('Cancel selection',cancel);
  toolbar.append(status);svg.after(toolbar);
  function x(event){
    const matrix=svg.getScreenCTM?.();
    if(matrix&&svg.createSVGPoint){const point=svg.createSVGPoint();point.x=event.clientX;point.y=event.clientY;try{return point.matrixTransform(matrix.inverse()).x;}catch{return NaN;}}
    const r=svg.getBoundingClientRect();return r.width>0?box[0]+(event.clientX-r.left)*box[2]/r.width:NaN;
  }
  listen(svg,'focus',draw);
  listen(root,'focusin',()=>{focused=true;});listen(root,'focusout',event=>{focused=root.contains(event.relatedTarget);});
  listen(svg,'keydown',event=>{
    if(event.altKey||event.ctrlKey||event.metaKey)return;
    if(event.key==='Escape'){event.preventDefault();cancel();return;}
    if(event.key==='Enter'){event.preventDefault();activate();return;}
    if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;
    event.preventDefault();pointerSelection=null;if(event.shiftKey&&anchor===null)anchor=current;
    current=event.key==='Home'?0:event.key==='End'?data.points.length-1:Math.max(0,Math.min(data.points.length-1,current+(event.key==='ArrowRight'?1:-1)));draw();
  });
  listen(svg,'pointerdown',event=>{
    if(event.button!==0||event.isPrimary===false||event.altKey||event.ctrlKey||event.metaKey)return;
    const position=x(event),index=nearest(data.points,position);if(index<0||position<plotX||position>plotX+plotW)return;
    current=index;anchor=null;pointerSelection=null;drag={pointer:event.pointerId,begin:index,first:position,clientX:event.clientX,moved:false};
    try{svg.setPointerCapture(event.pointerId);}catch{drag=null;return;}
    if(event.pointerType!=='touch')event.preventDefault();svg.focus({preventScroll:true});draw();
  });
  listen(svg,'pointermove',event=>{
    const position=x(event),index=nearest(data.points,position);if(index<0)return;
    if(drag&&drag.pointer===event.pointerId){current=index;drag.moved ||= Math.abs(event.clientX-drag.clientX)>=6;if(drag.moved)pointerSelection={first:drag.first,last:clampX(position)};if(event.pointerType!=='touch')event.preventDefault();draw();}
    else if(!drag&&event.pointerType!=='touch'){current=index;draw();}
  });
  listen(svg,'pointerup',event=>{
    if(!drag||drag.pointer!==event.pointerId)return;const ended=drag;drag=null;try{svg.releasePointerCapture(event.pointerId);}catch{}
    const position=x(event);current=nearest(data.points,position);if(current<0)current=ended.begin;
    if(Number.isFinite(position)&&(ended.moved||Math.abs(event.clientX-ended.clientX)>=6)){anchor=null;pointerSelection={first:ended.first,last:clampX(position)};draw();commit();}else{anchor=null;pointerSelection=null;draw();activate();}
  });
  listen(svg,'pointercancel',cancel);listen(svg,'lostpointercapture',()=>{if(drag)cancel();});
  listen(document,'visibilitychange',()=>{if(document.hidden&&drag)cancel();});listen(view,'pagehide',()=>{if(drag)cancel();});
  draw();if(restoreFocus&&document.activeElement===document.body)svg.focus({preventScroll:true});
  return {focused:()=>focused,intact:()=>root.contains(svg)&&layer.isConnected&&toolbar.isConnected&&root.contains(layer)&&root.contains(toolbar)&&svg.getAttribute('tabindex')==='0'&&svg.getAttribute('role')==='group'&&svg.getAttribute('aria-hidden')==='false',stop(){
    if(stopped)return;stopped=true;if(drag){const pointer=drag.pointer;drag=null;try{svg.releasePointerCapture(pointer);}catch{}}
    for(const remove of listeners.reverse())remove();layer.remove();toolbar.remove();
    for(const [name,value]of saved)value===null?svg.removeAttribute(name):svg.setAttribute(name,value);
    document.dispatchEvent(new view.CustomEvent('cui-chart:unmounted',{detail:Object.freeze({chartId:root.id})}));
  }};
}

export function install(scope=document) {
  if(installed.has(scope))return installed.get(scope);
  const document=scope.ownerDocument??scope, view=document.defaultView;
  const roots=new Map(),focusIds=new Set();let stopped=false;
  function scan(){
    if(stopped)return;
    for(const [root,record]of roots){
      if(!root.isConnected||!root.matches(ROOT)||(record.owner.intact&&!record.owner.intact())||record.signature!==signature(root)){
        if(record.owner.focused())focusIds.add(root.id);record.owner.stop();roots.delete(root);
      }
    }
    const candidates=[...(scope.matches?.(ROOT)?[scope]:[]),...scope.querySelectorAll(ROOT)];
    for(const root of candidates)if(!roots.has(root)){const owner=mount(root,focusIds.delete(root.id));roots.set(root,{signature:signature(root),owner});}
  }
  scan();const observer=new view.MutationObserver(scan);
  observer.observe(scope,{childList:true,subtree:true,attributes:true,attributeFilter:['data-cui-chart-enhance','data-start','data-end','data-key','data-time','data-value','data-missing','data-x','data-y','viewBox','viewbox','tabindex','role','aria-hidden']});
  function stop(){if(stopped)return;stopped=true;observer.disconnect();for(const record of roots.values())record.owner.stop();roots.clear();focusIds.clear();view.removeEventListener('pagehide',hide);installed.delete(scope);}
  function hide(event){if(!event.persisted)stop();}
  view.addEventListener('pagehide',hide);installed.set(scope,stop);return stop;
}
