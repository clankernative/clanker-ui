const selector = '[data-cui-component="file-upload"]';
const sessionsByRoot = new WeakMap();
const sessions = new Set();
const leases = new WeakMap();

/**
 * Enhance native file selection through an explicit app adapter. Requests contain
 * browser File references, the unchanged native input and a specimen-lifetime signal;
 * callback resolution is never server completion. Omitting the adapter preserves
 * native form submission. Overlapping roots lease each specimen to at most one owner.
 * @param {ParentNode & EventTarget} root
 * @param {{selection: (request: FileSelectionRequest) => unknown | Promise<unknown>, cancel?: (request: FileSelectionCancelRequest) => unknown | Promise<unknown>}} [adapter]
 * @returns {() => void} idempotent cleanup
 */
export function install(root, adapter) {
  if (!root?.addEventListener || !root?.querySelectorAll) throw new TypeError('A queryable event root is required.');
  if (adapter === undefined) return () => {};
  if (!adapter || typeof adapter !== 'object' || Reflect.ownKeys(adapter).some(key => typeof key !== 'string' || !['selection', 'cancel'].includes(key)) || typeof adapter.selection !== 'function' || (adapter.cancel !== undefined && typeof adapter.cancel !== 'function')) {
    throw new TypeError('File upload adapter requires selection(request) and optional cancel(request) functions.');
  }
  const existing = sessionsByRoot.get(root);
  if (existing) {
    if (existing.adapter !== adapter) throw new Error('This root already owns a different file upload adapter.');
    return existing.cleanup;
  }
  const document = root.ownerDocument ?? root;
  const view = document.defaultView ?? globalThis;
  const records = new Map();
  const session = { root, adapter, records, active: true, observer: null, wasConnected: root.isConnected === true };
  const inRoot = component => component === root || root.contains?.(component) === true;
  const components = () => [...(root.matches?.(selector) ? [root] : []), ...root.querySelectorAll(selector)].filter((node, index, all) => all.indexOf(node) === index);
  const inputFor = component => component.querySelector('[data-cui-file-upload-input]');
  const listFor = component => component.querySelector('[data-cui-file-upload-local-selection]');
  const feedbackFor = component => component.querySelector('[data-cui-file-upload-local-feedback]');
  const sameList = record => {
    const list = listFor(record.component), owned = record.ownedList;
    return !!list && !!owned && list.hidden === owned.hidden && list.childNodes.length === owned.nodes.length && owned.nodes.every((node, index) => list.childNodes[index] === node && node.textContent === owned.names[index]);
  };
  const canWriteList = record => {
    const list = listFor(record.component);
    return !!list && (sameList(record) || (!record.ownedList && list.hidden && list.childNodes.length === 0));
  };
  const renderNames = (record, files) => {
    if (!canWriteList(record)) { record.ownedList = null; return; }
    const list = listFor(record.component);
    const nodes = files.map(file => { const item = document.createElement('li'); item.textContent = file.name; return item; });
    list.replaceChildren(...nodes); list.hidden = nodes.length === 0;
    record.ownedList = { nodes, names: files.map(file => file.name), hidden: list.hidden };
  };
  const sameFeedback = record => {
    const output=feedbackFor(record.component),owned=record.ownedFeedback;
    return !!output&&!!owned&&output.hidden===false&&output.childNodes.length===1&&output.childNodes[0]===owned.node&&owned.node.textContent===owned.text;
  };
  const writeFeedback = (record,message) => {
    const output=feedbackFor(record.component);
    if(!output||(!sameFeedback(record)&&(record.ownedFeedback||!output.hidden||output.childNodes.length))){record.ownedFeedback=null;return;}
    const node=document.createElement('span');node.textContent=message;output.replaceChildren(node);output.hidden=false;
    record.ownedFeedback={node,text:message};
  };
  const clearFeedback = record => {
    if(!sameFeedback(record)){record.ownedFeedback=null;return;}
    const output=feedbackFor(record.component);output.replaceChildren();output.hidden=true;record.ownedFeedback=null;
  };
  const clearOwnedUI = record => {
    if (sameList(record)) { const list=listFor(record.component);list.replaceChildren();list.hidden=true; }
    if (sameFeedback(record)) { const output=feedbackFor(record.component);output.replaceChildren();output.hidden=true;record.ownedFeedback=null; }
  };
  const release = (component, record) => {
    if (leases.get(component) !== record) return;
    record.active = false; record.controller.abort();
    if (!inRoot(component) || (record.wasConnected && component.isConnected === false)) { /* detached or replaced: do not mutate the specimen */ }
    else clearOwnedUI(record);
    leases.delete(component); records.delete(component);
  };
  const owns = record => {
    if (!session.active || !record.active || leases.get(record.component) !== record) return false;
    if (!inRoot(record.component) || inputFor(record.component) !== record.input || (session.wasConnected && root.isConnected === false) || (record.wasConnected && record.component.isConnected === false)) {
      release(record.component, record); reconcileAll(); return false;
    }
    return true;
  };
  const acquire = component => {
    if (leases.has(component) || !inRoot(component) || (session.wasConnected && root.isConnected === false)) return;
    const controller = new view.AbortController();
    const record = { component, session, controller, active: true, input: inputFor(component), wasConnected: component.isConnected === true, generation: { selection: 0, cancel: 0 }, feedbackGeneration: 0, ownedList: null, ownedFeedback: null };
    records.set(component, record); leases.set(component, record);
  };
  const reportError = (record, kind) => {
    const output = feedbackFor(record.component);
    const attr = kind === 'cancel' ? 'data-cui-cancel-error-label' : 'data-cui-selection-error-label';
    const fallback = kind === 'cancel' ? 'File selection could not be cancelled.' : 'Files could not be handed to the application.';
    writeFeedback(record, output?.getAttribute(attr) || fallback);
  };
  const run = async (record, kind, files = []) => {
    if (!owns(record)) return;
    const input = inputFor(record.component);
    if (!input) return;
    const generation = ++record.generation[kind], feedbackGeneration = ++record.feedbackGeneration;
    const request = Object.freeze({ component: record.component, files: Object.freeze([...files]), input, signal: record.controller.signal });
    try {
      await (kind === 'selection' ? adapter.selection(request) : adapter.cancel?.(request));
      if (owns(record) && inputFor(record.component) === input && record.generation[kind] === generation && record.feedbackGeneration === feedbackGeneration) clearFeedback(record);
    } catch {
      if (owns(record) && inputFor(record.component) === input && record.generation[kind] === generation && record.feedbackGeneration === feedbackGeneration && !record.controller.signal.aborted) reportError(record, kind);
    }
  };
  const selection = (record, input) => {
    if (!owns(record) || inputFor(record.component) !== input || input.disabled) return;
    const files = [...(input.files ?? [])];
    renderNames(record, files); clearFeedback(record);
    void run(record, 'selection', files);
  };
  const eventComponent = target => target?.closest?.(selector);
  const change = event => {
    const component = eventComponent(event.target), record = component && leases.get(component);
    if (record?.session === session && inRoot(component)) selection(record, event.target);
  };
  const cancel = event => {
    const component=eventComponent(event.target),record=component&&leases.get(component);
    if(record?.session===session&&inRoot(component)&&inputFor(component)===event.target&&!event.target.disabled){clearFeedback(record);void run(record,'cancel',[]);}
  };
  const drag = event => {
    const zone=event.target?.closest?.('[data-cui-file-upload-dropzone]'),component=zone&&eventComponent(zone),record=component&&leases.get(component);
    if(record?.session!==session||!owns(record)||!zone||!component.contains(zone))return;
    const input=inputFor(component);if(!input||input.disabled)return;
    if(event.type==='dragover')event.preventDefault();
    if(event.type==='dragleave'&&zone.contains?.(event.relatedTarget))return;
    component.toggleAttribute('data-cui-drag-active',event.type==='dragenter'||event.type==='dragover');
    if(event.type!=='drop')return;
    event.preventDefault();component.removeAttribute('data-cui-drag-active');
    const dropped=[...(event.dataTransfer?.files??[])];if(!dropped.length)return;
    const files=input.multiple?dropped:dropped.slice(0,1);
    try {
      const transfer=new view.DataTransfer();files.forEach(file=>transfer.items.add(file));input.files=transfer.files;
    } catch { renderNames(record,files);clearFeedback(record);void run(record,'selection',files);return; }
    input.dispatchEvent(new view.Event('change',{bubbles:true}));
  };
  const sync = () => {
    if (!session.active) return;
    const current=new Set(components());
    let released=false;
    for(const [component,record] of [...records])if(!current.has(component)||!inRoot(component)||(session.wasConnected&&root.isConnected===false)||inputFor(component)!==record.input){release(component,record);released=true;}
    for(const component of current)acquire(component);
    if(released)reconcileAll();
  };
  session.sync=sync;
  const reconcileAll=()=>{for(const active of sessions)active.sync();};
  const cleanup = () => {
    if(!session.active)return;
    session.active=false;session.observer?.disconnect();root.removeEventListener('change',change);root.removeEventListener('cancel',cancel);
    for(const type of ['dragenter','dragover','dragleave','drop'])root.removeEventListener(type,drag);
    for(const [component,record] of [...records])release(component,record);
    sessions.delete(session);sessionsByRoot.delete(root);reconcileAll();
  };
  session.cleanup=cleanup;sessions.add(session);sessionsByRoot.set(root,session);
  root.addEventListener('change',change);root.addEventListener('cancel',cancel);
  for(const type of ['dragenter','dragover','dragleave','drop'])root.addEventListener(type,drag);
  session.observer=view.MutationObserver?new view.MutationObserver(sync):null;
  session.observer?.observe(root,{childList:true,subtree:true,attributes:true});sync();
  return cleanup;
}

/** @typedef {{component: HTMLElement, files: readonly File[], input: HTMLInputElement, signal: AbortSignal}} FileSelectionRequest */
/** @typedef {FileSelectionRequest} FileSelectionCancelRequest */
