const SELECTOR = '[data-cui-component="theme-switcher"]';
const TOKEN = /^[A-Za-z][A-Za-z0-9_.:-]*$/;
const installations = new WeakMap();
const targetLeases = new WeakMap();
const includes = (values,value) => typeof value === 'string' && values.has(value);

/** Owns bounded presentation preference only; never writes an application model. */
export function install(root=document) {
  if (installations.has(root)) return installations.get(root);
  const document=root.ownerDocument ?? root;
  const view=document.defaultView;
  const owners=new Map();
  const pendingLeases=new Map();
  let stopped=false;
  function configuration(owner) {
    const buttons=[...owner.querySelectorAll('[data-cui-theme-value]')];
    return {buttons, values:buttons.map(button=>button.getAttribute('data-cui-theme-value')),
      defaultChoice:owner.getAttribute('data-cui-theme-default'),
      light:owner.getAttribute('data-cui-theme-system-light'),dark:owner.getAttribute('data-cui-theme-system-dark'),
      storage:owner.getAttribute('data-cui-theme-storage-key'),targetId:owner.getAttribute('data-cui-theme-target-id')};
  }
  function mount(owner,config,selectedBefore) {
    const {buttons,values,defaultChoice,light,dark,storage,targetId}=config;
    const choices=new Set(values);
    const target=targetId ? document.getElementById(targetId) : document.documentElement;
    const key=storage===null ? 'clanker-theme' : storage || null;
    if (!view || !target || buttons.length<2 || buttons.length>8 || choices.size!==buttons.length ||
      values.some(value=>!value || !TOKEN.test(value)) || !includes(choices,defaultChoice) ||
      !includes(choices,light) || !includes(choices,dark) || light==='system' || dark==='system' ||
      (key && !TOKEN.test(key)) || (targetId && !TOKEN.test(targetId))) return null;
    let lease=targetLeases.get(target);
    if(!lease){lease={owner:null,waiters:new Set()};targetLeases.set(target,lease);}
    if(lease.owner){lease.waiters.add(refresh);pendingLeases.set(lease,refresh);return null;}
    lease.owner=owner;
    lease.waiters.delete(refresh);pendingLeases.delete(lease);
    const controller=new AbortController();
    const originalEnhanced={present:owner.hasAttribute('data-cui-enhanced'),value:owner.getAttribute('data-cui-enhanced')};
    const originalButtons=buttons.map(button=>({button,hidden:button.hidden,pressed:button.getAttribute('aria-pressed')}));
    let media;
    try { media=view.matchMedia?.('(prefers-color-scheme: dark)'); } catch { /* Direct choices still work. */ }
    let selected=includes(choices,selectedBefore) ? selectedBefore : defaultChoice;
    if (!selectedBefore) {
      try {const stored=key ? view.localStorage?.getItem(key) : null;if(includes(choices,stored)) selected=stored;} catch { /* Preference storage is optional. */ }
    }
    let applied;
    const resolved=()=>selected==='system' ? (media?.matches ? dark : light) : selected;
    function apply(announce=false) {
      // Native reference palettes require an explicit opt-in. Resolve the user's
      // system choice to its configured concrete palette, retaining system as the
      // selected/stored choice. Do not rely on an ambient application theme.
      applied=resolved();target.setAttribute('data-cui-theme',applied);
      for (const button of buttons) {button.hidden=false;button.setAttribute('aria-pressed',String(button.getAttribute('data-cui-theme-value')===selected));}
      owner.setAttribute('data-cui-enhanced','');
      if (announce && view.CustomEvent) owner.dispatchEvent(new view.CustomEvent('cui:theme-changed',{
        bubbles:true,detail:Object.freeze({theme:selected,resolvedTheme:applied})
      }));
    }
    function change(event) {
      const button=event.target?.closest?.('[data-cui-theme-value]');
      const choice=button?.getAttribute('data-cui-theme-value');
      if (!button || !owner.contains(button) || !includes(choices,choice) || button.disabled) return;
      selected=choice;
      try {if(key) view.localStorage?.setItem(key,choice);} catch { /* Keep the in-memory preference. */ }
      apply(true);
    }
    const schemeChange=()=>{if(selected==='system')apply(true);};
    owner.addEventListener('click',change,{signal:controller.signal});
    if (media?.addEventListener) media.addEventListener('change',schemeChange,{signal:controller.signal});
    else media?.addListener?.(schemeChange);
    // Re-evaluate persisted pages after the OS changed while the page was frozen.
    view.addEventListener('pageshow',schemeChange,{signal:controller.signal});
    apply();
    return {config, target, selected:()=>selected, stop(){
      controller.abort();if(!media?.addEventListener)media?.removeListener?.(schemeChange);
      for (const {button,hidden,pressed} of originalButtons) {
        button.hidden=hidden;
        if(pressed===null)button.removeAttribute('aria-pressed');else button.setAttribute('aria-pressed',pressed);
      }
      if(originalEnhanced.present)owner.setAttribute('data-cui-enhanced',originalEnhanced.value);else owner.removeAttribute('data-cui-enhanced');
      // Teardown is not an undo. Equal values cannot identify the last writer,
      // so leave the app-owned target untouched when releasing this owner.
      if(lease.owner===owner){lease.owner=null;for(const retry of [...lease.waiters])retry();}
    }};
  }
  function unchanged(a,b) {
    return a.defaultChoice===b.defaultChoice && a.light===b.light && a.dark===b.dark && a.storage===b.storage && a.targetId===b.targetId &&
      a.buttons.length===b.buttons.length && a.buttons.every((button,index)=>button===b.buttons[index] && a.values[index]===b.values[index]);
  }
  function refresh() {
    if(stopped)return;
    for(const [lease,retry] of pendingLeases)lease.waiters.delete(retry);
    pendingLeases.clear();
    const current=new Set([...(root.matches?.(SELECTOR)?[root]:[]),...root.querySelectorAll(SELECTOR)]);
    for(const [owner,state] of owners) {
      if(!current.has(owner)) {owners.delete(owner);state.stop();}
    }
    for(const owner of current) {
      const config=configuration(owner),previous=owners.get(owner);
      const target=config.targetId ? document.getElementById(config.targetId) : document.documentElement;
      if(previous && unchanged(previous.config,config) && previous.target===target)continue;
      const selection=previous?.selected();owners.delete(owner);previous?.stop();
      const state=mount(owner,config,selection);if(state)owners.set(owner,state);
    }
  }
  const observer=view?.MutationObserver ? new view.MutationObserver(refresh) : null;
  refresh();
  observer?.observe(root,{subtree:true,childList:true,attributes:true,attributeFilter:['data-cui-component','data-cui-theme-value','data-cui-theme-default','data-cui-theme-system-light','data-cui-theme-system-dark','data-cui-theme-storage-key','data-cui-theme-target-id','id']});
  function stop(){
    if(stopped)return;stopped=true;observer?.disconnect();
    for(const state of [...owners.values()].reverse())state.stop();owners.clear();installations.delete(root);
  }
  installations.set(root,stop);return stop;
}
