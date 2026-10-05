const selector = '[data-cui-component="data-viewport"]';
const navigationSelector = '[data-cui-viewport-navigation]';
const sessionsByRoot = new WeakMap();
const sessions = new Set();
const leases = new WeakMap();
const MAX_TOTAL = 10_000_000;

/**
 * Observe bounded window metadata and explicitly marked admitted navigation controls.
 * Requests carry component/control context and a specimen-lifetime AbortSignal, never
 * records or query strings. The adapter owns all retrieval and navigation transport.
 * Add data-cui-viewport-navigation to each admitted native link/button/form control;
 * it must already work without JavaScript. Omitting an adapter is a no-op.
 * Overlapping roots lease a specimen to at most one installer.
 * @param {ParentNode & EventTarget} root
 * @param {{window: (request: ViewportWindowRequest) => unknown | Promise<unknown>, navigate?: (request: ViewportNavigationRequest) => unknown | Promise<unknown>} | {window?: (request: ViewportWindowRequest) => unknown | Promise<unknown>, navigate: (request: ViewportNavigationRequest) => unknown | Promise<unknown>}} [adapter]
 * @returns {() => void} idempotent cleanup
 */
export function install(root, adapter) {
  if (!root?.addEventListener || !root?.querySelectorAll) throw new TypeError('A queryable event root is required.');
  if (adapter === undefined) return () => {};
  if (!adapter || typeof adapter !== 'object' || Reflect.ownKeys(adapter).some(key => typeof key !== 'string' || !['window', 'navigate'].includes(key)) || (adapter.window !== undefined && typeof adapter.window !== 'function') || (adapter.navigate !== undefined && typeof adapter.navigate !== 'function') || (!adapter.window && !adapter.navigate)) {
    throw new TypeError('Viewport adapter accepts window(request) and/or navigate(request) functions only.');
  }
  const existing=sessionsByRoot.get(root);
  if(existing){if(existing.adapter!==adapter)throw new Error('This root already owns a different viewport adapter.');return existing.cleanup;}
  const document=root.ownerDocument??root,view=document.defaultView??globalThis;
  const records=new Map();
  const session={root,adapter,records,active:true,observer:null,wasConnected:root.isConnected===true};
  const inRoot=component=>component===root||root.contains?.(component)===true;
  const components=()=>[...(root.matches?.(selector)?[root]:[]),...root.querySelectorAll(selector)].filter((node,index,all)=>all.indexOf(node)===index);
  const statusFor=component=>component.querySelector('[data-cui-viewport-status]');
  const sameStatus=record=>{const output=statusFor(record.component),owned=record.ownedStatus;return !!output&&!!owned&&output.hidden===false&&output.childNodes.length===1&&output.childNodes[0]===owned.node&&owned.node.textContent===owned.text;};
  const setStatus=(record,message)=>{
    const output=statusFor(record.component);
    if(!output||(!sameStatus(record)&&(record.ownedStatus||!output.hidden||output.childNodes.length))){record.ownedStatus=null;return;}
    const node=document.createElement('span');node.textContent=message;output.replaceChildren(node);output.hidden=false;record.ownedStatus={node,text:message};
  };
  const clearStatus=record=>{if(!sameStatus(record)){record.ownedStatus=null;return;}const output=statusFor(record.component);output.replaceChildren();output.hidden=true;record.ownedStatus=null;};
  const validInteger=(value,min,max)=>Number.isSafeInteger(value)&&value>=min&&value<=max;
  const activeRecord=record=>{
    if(!session.active||!record.active||leases.get(record.component)!==record)return false;
    if(!inRoot(record.component)||(session.wasConnected&&root.isConnected===false)||(record.wasConnected&&record.component.isConnected===false)){
      release(record.component,record);reconcileAll();return false;
    }
    return true;
  };
  const boundedWindow=(component,scroller)=>{
    const total=Number(component.getAttribute('data-cui-window-total'));
    const itemSize=Number(component.getAttribute('data-cui-window-item-size'));
    const overscan=Number(component.getAttribute('data-cui-window-overscan'));
    const declaredStart=Number(component.getAttribute('data-cui-window-start'));
    if(![total,itemSize,overscan,declaredStart].every(Number.isSafeInteger)||!validInteger(total,1,MAX_TOTAL)||!validInteger(itemSize,16,4096)||!validInteger(overscan,1,100)||!validInteger(declaredStart,0,total-1))return null;
    const start=Math.min(total-1,Math.max(0,Math.floor(scroller.scrollTop/itemSize)-overscan));
    const visible=Math.max(1,Math.ceil(scroller.clientHeight/itemSize));
    const end=Math.min(total,start+visible+overscan*2);
    return validInteger(end,start+1,total)?{start,end,total}:null;
  };
  const emit=async(record,kind,callback,request,generation,isCurrent=()=>true)=>{
    if(!activeRecord(record)||!isCurrent())return false;
    const feedbackGeneration=++record.feedbackGeneration;
    try{
      await callback(Object.freeze(request));
      if(activeRecord(record)&&isCurrent()&&record.generation[kind]===generation&&record.feedbackGeneration===feedbackGeneration){clearStatus(record);return true;}
    }catch{
      if(activeRecord(record)&&isCurrent()&&record.generation[kind]===generation&&record.feedbackGeneration===feedbackGeneration&&!record.controller.signal.aborted){
        const output=statusFor(record.component),attr=kind==='navigation'?'data-cui-navigation-error-label':'data-cui-window-error-label';
        setStatus(record,output?.getAttribute(attr)||(kind==='navigation'?'Navigation could not be requested.':'The visible window could not be updated.'));
      }
    }
    return false;
  };
  const requestWindow=(record,scroller)=>{
    if(!session.adapter.window||!activeRecord(record))return;
    const window=boundedWindow(record.component,scroller);if(!window)return;
    const key=`${window.start}:${window.end}:${window.total}`;
    if(key===record.windowKey||key===record.pendingWindow)return;
    record.pendingWindow=key;const generation=++record.generation.window;clearStatus(record);
    const request={...window,component:record.component,scroller,signal:record.controller.signal};
    void emit(record,'window',session.adapter.window,request,generation,()=>record.scroller===scroller).then(success=>{
      if(leases.get(record.component)!==record)return;
      if(record.pendingWindow===key)record.pendingWindow=null;
      if(success&&record.generation.window===generation)record.windowKey=key;
    });
  };
  const bindScroller=record=>{
    const scroller=record.component.querySelector('[data-cui-scroll]');
    if(record.scroller===scroller)return;
    record.scroller?.removeEventListener('scroll',record.onScroll);record.resize?.disconnect();
    if(record.scroller!==scroller){record.generation.window++;record.windowKey=null;record.pendingWindow=null;}
    record.scroller=scroller;
    if(!scroller)return;
    record.onScroll=()=>requestWindow(record,scroller);scroller.addEventListener('scroll',record.onScroll,{passive:true});
    const Resize=view.ResizeObserver;
    if(Resize){record.resize=new Resize(()=>requestWindow(record,scroller));record.resize.observe(scroller);}
  };
  const release=(component,record)=>{
    if(leases.get(component)!==record)return;
    record.active=false;record.controller.abort();record.scroller?.removeEventListener('scroll',record.onScroll);record.resize?.disconnect();
    if(inRoot(component)&&!(record.wasConnected&&component.isConnected===false)&&sameStatus(record)){const output=statusFor(component);output.replaceChildren();output.hidden=true;record.ownedStatus=null;}
    leases.delete(component);records.delete(component);
  };
  const acquire=component=>{
    if(leases.has(component)||!inRoot(component)||(session.wasConnected&&root.isConnected===false))return;
    const controller=new view.AbortController();
    const record={component,session,controller,active:true,wasConnected:component.isConnected===true,generation:{window:0,navigation:0},feedbackGeneration:0,windowKey:null,pendingWindow:null,ownedStatus:null,scroller:null,resize:null};
    records.set(component,record);leases.set(component,record);bindScroller(record);
  };
  const click=event=>{
    if(!session.adapter.navigate)return;
    const control=event.target?.closest?.(navigationSelector),component=control?.closest?.(selector),record=component&&leases.get(component);
    if(!record||record.session!==session||!activeRecord(record)||!component.contains(control))return;
    const generation=++record.generation.navigation;clearStatus(record);
    const scroller=component.querySelector('[data-cui-scroll]');
    const window=scroller?boundedWindow(component,scroller):null;
    const request={component,control,signal:record.controller.signal,...(window?{window:Object.freeze(window)}:{})};
    void emit(record,'navigation',session.adapter.navigate,request,generation,()=>component.contains(control));
    // Deliberately do not prevent default: the admitted native navigation remains authoritative.
  };
  const reconcile=()=>{
    if(!session.active)return;
    const current=new Set(components());let released=false;
    if(session.wasConnected&&root.isConnected===false){for(const [component,record] of [...records]){release(component,record);released=true;}}
    else{
      for(const [component,record] of [...records])if(!current.has(component)||!inRoot(component)){release(component,record);released=true;}
      for(const component of current)acquire(component);
      for(const record of records.values())bindScroller(record);
    }
    if(released)reconcileAll();
  };
  session.sync=reconcile;
  const reconcileAll=()=>{for(const active of sessions)active.sync();};
  const cleanup=()=>{
    if(!session.active)return;session.active=false;session.observer?.disconnect();root.removeEventListener('click',click);
    for(const [component,record] of [...records])release(component,record);
    sessions.delete(session);sessionsByRoot.delete(root);reconcileAll();
  };
  session.cleanup=cleanup;sessions.add(session);sessionsByRoot.set(root,session);root.addEventListener('click',click);
  session.observer=view.MutationObserver?new view.MutationObserver(reconcile):null;
  session.observer?.observe(root,{childList:true,subtree:true,attributes:true});reconcile();return cleanup;
}

/** @typedef {{start:number,end:number,total:number,component:HTMLElement,scroller:HTMLElement,signal:AbortSignal}} ViewportWindowRequest */
/** @typedef {{component:HTMLElement,control:HTMLElement,signal:AbortSignal,window?:Readonly<{start:number,end:number,total:number}>}} ViewportNavigationRequest */
