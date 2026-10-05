const SELECTOR = '[data-cui-component="tooltip"]';
const placements = { top: ["top", "bottom", "right", "left"], right: ["right", "left", "bottom", "top"], bottom: ["bottom", "top", "right", "left"], left: ["left", "right", "bottom", "top"] };
function point(side, t, b) { return side === "top" ? [t.left + (t.width-b.width)/2, t.top-b.height-8] : side === "bottom" ? [t.left+(t.width-b.width)/2,t.bottom+8] : side === "right" ? [t.right+8,t.top+(t.height-b.height)/2] : [t.left-b.width-8,t.top+(t.height-b.height)/2]; }
/** Install keyboard, pointer, outside-dismissal, and collision-aware tooltip behavior. */
export function install(root = document) {
  const doc = root.ownerDocument ?? (root.nodeType === 9 ? root : globalThis.document);
  const view = doc?.defaultView ?? globalThis.window;
  const ElementType = view?.Element ?? globalThis.Element;
  const NodeType = view?.Node ?? globalThis.Node;
  const AbortControllerType = view?.AbortController ?? globalThis.AbortController;
  const controller = new AbortControllerType(); let active = null;
  const containsRoot = (node) => node === root || root.contains?.(node);
  const enhanced = new Set();
  function enhance(scope) {
    for (const tip of [...(scope.matches?.(SELECTOR) ? [scope] : []), ...(scope.querySelectorAll?.(SELECTOR) ?? [])]) {
      if (!containsRoot(tip) || !tip.querySelector('[data-cui-tooltip-trigger]') || !tip.querySelector('[data-cui-tooltip-bubble]')) continue;
      tip.setAttribute('data-cui-tooltip-enhanced', ''); enhanced.add(tip);
    }
  }
  enhance(root);
  const position = (tip) => {
    const trigger = tip.querySelector("[data-cui-tooltip-trigger]"); const bubble = tip.querySelector("[data-cui-tooltip-bubble]"); if (!trigger || !bubble) return;
    const t = trigger.getBoundingClientRect(); bubble.style.left = "0px"; bubble.style.top = "0px"; const b = bubble.getBoundingClientRect();
    const width = view?.innerWidth ?? 0; const height = view?.innerHeight ?? 0;
    const preferred = placements[tip.dataset.cuiTooltipPlacement] ? tip.dataset.cuiTooltipPlacement : "top";
    const candidates = placements[preferred]; const score = (p) => { const [x,y]=point(p,t,b); return Math.max(0,8-x)+Math.max(0,8-y)+Math.max(0,x+b.width-width+8)+Math.max(0,y+b.height-height+8); };
    const side = candidates.reduce((best,p)=>score(p)<score(best)?p:best); let [x,y]=point(side,t,b); x=Math.max(8,Math.min(x,width-b.width-8)); y=Math.max(8,Math.min(y,height-b.height-8));
    bubble.style.left=`${Math.round(x)}px`; bubble.style.top=`${Math.round(y)}px`; tip.dataset.cuiTooltipResolved=side;
  };
  const open = (tip) => { active=tip; tip.removeAttribute("data-cui-tooltip-dismissed"); tip.dataset.cuiTooltipOpen=""; position(tip); };
  const close = (tip, dismissed=false) => { if (dismissed) tip.dataset.cuiTooltipDismissed=""; tip.removeAttribute("data-cui-tooltip-open"); if(active===tip) active=null; };
  const release = (tip) => {
    close(tip); tip.removeAttribute('data-cui-tooltip-enhanced'); tip.removeAttribute('data-cui-tooltip-dismissed'); tip.removeAttribute('data-cui-tooltip-resolved');
    const bubble = tip.querySelector('[data-cui-tooltip-bubble]'); bubble?.style.removeProperty('left'); bubble?.style.removeProperty('top');
    enhanced.delete(tip);
  };
  const element = (target) => ElementType && target instanceof ElementType ? target : null;
  const node = (target) => NodeType && target instanceof NodeType ? target : null;
  root.addEventListener("pointerover",(e)=>{const trigger=element(e.target)?.closest("[data-cui-tooltip-trigger]"); if(trigger){const tip=trigger.closest(SELECTOR); if(tip && containsRoot(tip))open(tip);}},{signal:controller.signal});
  root.addEventListener("pointerout",(e)=>{const tip=element(e.target)?.closest(SELECTOR); if(tip && !(node(e.relatedTarget) && tip.contains(e.relatedTarget)))close(tip);},{signal:controller.signal});
  root.addEventListener("focusin",(e)=>{const trigger=element(e.target)?.closest("[data-cui-tooltip-trigger]"); const tip=trigger?.closest(SELECTOR); if(tip && containsRoot(tip))open(tip);},{signal:controller.signal});
  root.addEventListener("focusout",(e)=>{const tip=element(e.target)?.closest(SELECTOR); if(tip && !(node(e.relatedTarget) && tip.contains(e.relatedTarget)))close(tip);},{signal:controller.signal});
  root.addEventListener("keydown",(e)=>{if(e.key==="Escape"&&active){e.preventDefault(); close(active,true);}},{signal:controller.signal});
  root.addEventListener("pointerdown",(e)=>{if(active && node(e.target) && !active.contains(e.target))close(active,true);},{signal:controller.signal});
  const reposition=()=>{if(active && containsRoot(active))position(active);};
  view?.addEventListener("resize",reposition,{signal:controller.signal}); doc?.addEventListener("scroll",reposition,{capture:true,signal:controller.signal});
  const Observer = view?.MutationObserver ?? globalThis.MutationObserver;
  const observer = typeof Observer === "function" ? new Observer((records) => {
    for (const record of records) {
      for (const added of record.addedNodes ?? []) enhance(added);
      for (const removed of record.removedNodes ?? []) {
        if (containsRoot(removed)) continue;
        if (active && (removed === active || removed.contains?.(active))) close(active);
        for (const tip of [...enhanced]) if (removed === tip || removed.contains?.(tip)) {
          release(tip);
        }
      }
    }
  }) : null;
  observer?.observe(root, { childList: true, subtree: true });
  return ()=>{controller.abort(); observer?.disconnect(); if(active)close(active); for(const tip of [...enhanced]) release(tip);};
}
