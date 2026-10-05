import { install as installCalendars, parseCalendarDate } from '../date-calendar/interaction.js';

/** @typedef {string & {readonly __isoDate: unique symbol}} ISODate */
/**
 * App-facing browser draft output. Dates are strict Gregorian ISO strings or
 * null; range ordering and business rules remain the app's responsibility.
 * @typedef {{mode: 'single'|'range', start: ISODate|null, end: ISODate|null, complete: boolean}} PickerSelectionDetail
 */
const ROOT = '[data-cui-date-picker]';
const owners = new WeakMap();
const services = new WeakMap();
function nearest(el, selector) { return el?.closest?.(selector) || null; }
function own(owner, node) { return !!node && (node === owner || owner.contains(node)) && nearest(node, ROOT) === owner; }
function parseISO(value) { return !!parseCalendarDate(value); }

function makePicker(owner) {
  let disposed = false, dispatchingSelection = false, controller, calendar, dropdown, summary, panel, starts, ends, inputList = [], form, formHandler, pendingReset = 0;
  const doc = owner.ownerDocument, view = doc?.defaultView;
  if (!view?.AbortController || !view?.Event || !view?.CustomEvent) return { dispose() {} };
  let abort = new view.AbortController();
  let releaseCalendar = () => {};
  let panelPositionState = null;
  function restorePanelPosition() {
    if (!panelPositionState) return;
    const { node, styles, placement } = panelPositionState;
    for (const [name, state] of styles) {
      if (node.style.getPropertyValue(name) !== state.ownedValue || node.style.getPropertyPriority(name) !== state.ownedPriority) continue;
      if (state.value) node.style.setProperty(name, state.value, state.priority);
      else node.style.removeProperty(name);
    }
    if (node.dataset.cuiDatePanelPlacement === placement.ownedValue) {
      if (placement.value === undefined) delete node.dataset.cuiDatePanelPlacement;
      else node.dataset.cuiDatePanelPlacement = placement.value;
    }
    panelPositionState = null;
  }
  function writePanelPosition(node, values, placementValue) {
    if (!panelPositionState || panelPositionState.node !== node) {
      restorePanelPosition();
      const styles = new Map();
      for (const name of Object.keys(values)) styles.set(name, { value: node.style.getPropertyValue(name), priority: node.style.getPropertyPriority(name), ownedValue: '', ownedPriority: '' });
      panelPositionState = { node, styles, placement: { value: node.dataset.cuiDatePanelPlacement, ownedValue: '' } };
    }
    for (const [name, value] of Object.entries(values)) {
      node.style.setProperty(name, value);
      const state = panelPositionState.styles.get(name); state.ownedValue = value; state.ownedPriority = node.style.getPropertyPriority(name);
    }
    node.dataset.cuiDatePanelPlacement = placementValue;
    panelPositionState.placement.ownedValue = placementValue;
  }

  function discover() {
    const nextCalendar = [...owner.querySelectorAll('[data-cui-date-calendar]')].find((el) => own(owner, el));
    const nextDropdown = [...owner.querySelectorAll('[data-cui-date-dropdown]')].find((el) => own(owner, el));
    const nextSummary = nextDropdown?.querySelector('summary') || null;
    const nextPanel = nextDropdown?.querySelector('[data-cui-date-panel]') || null;
    const inputs = [...owner.querySelectorAll('input[type="date"]')].filter((el) => own(owner, el));
    const nextStarts = inputs.find((el) => el.hasAttribute('data-cui-date-start')) || inputs[0] || null;
    const nextEnds = inputs.find((el) => el.hasAttribute('data-cui-date-end')) || inputs[1] || null;
    const nextForm = nextStarts?.form || nextStarts?.closest('form') || null;
    const changed = nextCalendar !== calendar || nextDropdown !== dropdown || nextSummary !== summary || nextPanel !== panel || inputs.some((el, i) => inputList[i] !== el) || inputs.length !== inputList.length || nextForm !== form;
    if (!changed) return false;
    if (nextPanel !== panel) restorePanelPosition();
    abort.abort(); abort = new view.AbortController();
    releaseCalendar(); releaseCalendar = () => {};
    calendar = nextCalendar; dropdown = nextDropdown; summary = nextSummary; panel = nextPanel; starts = nextStarts; ends = nextEnds; inputList = inputs; form = nextForm;
    if (calendar) releaseCalendar = installCalendars(calendar);
    for (const input of inputList) {
      input.addEventListener('input', onInputChange, { signal: abort.signal });
      input.addEventListener('change', onInputChange, { signal: abort.signal });
    }
    calendar?.addEventListener('cui:date-calendar-changed', onCalendarChange, { signal: abort.signal });
    owner.addEventListener('keydown', onKeydown, { signal: abort.signal });
    dropdown?.addEventListener('toggle', onToggle, { signal: abort.signal });
    doc.addEventListener('click', onDocumentClick, { signal: abort.signal, capture: true });
    view.addEventListener('resize', place, { signal: abort.signal });
    view.addEventListener('scroll', place, { signal: abort.signal, capture: true, passive: true });
    view.visualViewport?.addEventListener('resize', place, { signal: abort.signal });
    view.visualViewport?.addEventListener('scroll', place, { signal: abort.signal });
    if (form) { formHandler = onReset; form.addEventListener('reset', formHandler, { signal: abort.signal }); }
    owner.dataset.cuiDateReady = '';
    return true;
  }
  function refreshFromInputs() {
    if (!calendar) return;
    const start = starts?.value || '', end = ends?.value || '';
    calendar.dataset.cuiDateSelectionStart = start;
    calendar.dataset.cuiDateSelectionEnd = end;
    if (parseISO(start)) calendar.dataset.cuiDateMonth = `${start.slice(0, 7)}-01`;
    calendar.dispatchEvent(new view.Event('cui:date-picker-refresh'));
  }
  function emitSelection(startValue, endValue) {
    const mode = owner.dataset.cuiDateMode === 'range' ? 'range' : 'single';
    const start = parseCalendarDate(startValue) ? startValue : null;
    const end = mode === 'range' && parseCalendarDate(endValue) ? endValue : null;
    /** @type {PickerSelectionDetail} */
    const detail = { mode, start, end, complete: mode === 'single' ? start !== null : start !== null && end !== null };
    owner.dispatchEvent(new view.CustomEvent('cui:date-picker-changed', { bubbles: true, detail }));
  }
  function onInputChange(event) {
    if (dispatchingSelection || !own(owner, event.target)) return;
    refreshFromInputs();
    emitSelection(starts?.value || '', ends?.value || '');
  }
  function onCalendarChange(event) {
    if (event.target !== calendar || !own(owner, calendar)) return;
    const detail = event.detail;
    const mode = owner.dataset.cuiDateMode === 'range' ? 'range' : 'single';
    const start = typeof detail?.start === 'string' && parseCalendarDate(detail.start) ? detail.start : null;
    const end = typeof detail?.end === 'string' && parseCalendarDate(detail.end) ? detail.end : null;
    const complete = mode === 'single' ? start !== null && end === null : start !== null && end !== null;
    const unavailable = new Set([...calendar.querySelectorAll('[data-cui-date-unavailable]')].map((node) => node.getAttribute('value') || node.value || ''));
    const min = calendar.dataset.cuiDateMinimum || '', max = calendar.dataset.cuiDateMaximum || '';
    const violatesConstraints = [start, end].filter(Boolean).some((date) => (min && date < min) || (max && date > max) || unavailable.has(date));
    if (detail?.mode !== mode || detail?.complete !== complete || !start || violatesConstraints || starts?.disabled || (mode === 'range' && ends?.disabled) || (mode === 'single' && end !== null) || (mode === 'range' && end !== null && end < start)) return;
    // Batch the values first so observers of either native event see the completed range.
    const changed = [];
    if (starts && starts.value !== start) { starts.value = start; changed.push(starts); }
    if (ends && ends.value !== end) { ends.value = end; changed.push(ends); }
    dispatchingSelection = true;
    try {
      for (const input of changed) {
        input.dispatchEvent(new view.Event('input', { bubbles: true }));
        input.dispatchEvent(new view.Event('change', { bubbles: true }));
      }
    } finally { dispatchingSelection = false; }
    emitSelection(start, end);
    if (detail.complete && dropdown?.open) { dropdown.open = false; summary?.focus(); }
  }
  function onKeydown(event) {
    if (event.key === 'Escape' && dropdown?.open && own(owner, event.target)) { event.preventDefault(); dropdown.open = false; summary?.focus(); }
  }
  function onDocumentClick(event) {
    if (dropdown?.open && !dropdown.contains(event.target) && own(owner, dropdown)) { dropdown.open = false; }
  }
  function onToggle() {
    if (!dropdown?.open) return;
    place();
    const selected = calendar?.querySelector('[data-cui-date-day][tabindex="0"]');
    selected?.focus();
  }
  function place() {
    if (!dropdown?.open || !panel || !summary || !own(owner, dropdown)) return;
    const rect = summary.getBoundingClientRect(), size = panel.getBoundingClientRect(), gap = 8;
    const viewport = view.visualViewport, viewportLeft = viewport?.offsetLeft || 0, viewportTop = viewport?.offsetTop || 0;
    const width = Math.max(0, viewport?.width || view.innerWidth || doc.documentElement.clientWidth), height = Math.max(0, viewport?.height || view.innerHeight || doc.documentElement.clientHeight);
    const rtl = view.getComputedStyle(owner).direction === 'rtl';
    const minLeft = viewportLeft + gap, maxLeft = Math.max(minLeft, viewportLeft + width - size.width - gap), preferred = rtl ? rect.right - size.width : rect.left;
    const left = Math.max(minLeft, Math.min(preferred, maxLeft));
    const below = Math.max(0, viewportTop + height - rect.bottom - gap), above = Math.max(0, rect.top - viewportTop - gap);
    const up = size.height > below && above > below, available = Math.max(0, (up ? above : below));
    const placedHeight = Math.min(size.height, available);
    const top = up ? Math.max(viewportTop + gap, rect.top - placedHeight) : Math.min(Math.max(viewportTop + gap, rect.bottom), Math.max(viewportTop + gap, viewportTop + height - gap - placedHeight));
    writePanelPosition(panel, {
      '--cui-date-picker-left': `${left}px`,
      '--cui-date-picker-top': `${top}px`,
      'max-inline-size': `${Math.max(0, width - 2 * gap)}px`,
      'max-block-size': `${Math.max(0, available)}px`,
    }, up ? 'above' : 'below');
  }
  function onReset() {
    if (pendingReset) view.clearTimeout(pendingReset);
    pendingReset = view.setTimeout(() => { pendingReset = 0; if (!disposed) refreshFromInputs(); }, 0);
  }

  discover(); refreshFromInputs();
  return {
    update() { if (disposed) return; if (discover()) refreshFromInputs(); },
    dispose() {
      if (disposed) return; disposed = true; abort.abort(); releaseCalendar(); restorePanelPosition();
      if (pendingReset) view.clearTimeout(pendingReset);
      if (owner.dataset.cuiDateReady !== undefined) delete owner.dataset.cuiDateReady;
    },
  };
}
function acquirePicker(owner) {
  let entry = owners.get(owner);
  if (!entry) { entry = { refs: 0, controller: makePicker(owner) }; owners.set(owner, entry); }
  entry.refs++;
  let released = false;
  return () => { if (released) return; released = true; if (--entry.refs === 0) { entry.controller.dispose(); owners.delete(owner); } };
}
function inScope(scope, element) { return (element === scope || scope.contains?.(element)) && nearest(element, ROOT) === element; }
function makeService(scope) {
  const doc = scope.nodeType === 9 ? scope : scope.ownerDocument, view = doc?.defaultView;
  let observer, stopped = false; const active = new Map();
  function scan() {
    if (stopped) return;
    const candidates = [];
    if (scope.matches?.(ROOT)) candidates.push(scope);
    for (const owner of scope.querySelectorAll?.(ROOT) || []) candidates.push(owner);
    const desired = new Set(candidates.filter((owner) => inScope(scope, owner)));
    for (const owner of desired) { if (!active.has(owner)) active.set(owner, acquirePicker(owner)); else owners.get(owner)?.controller.update(); }
    for (const [owner, release] of active) if (!desired.has(owner)) { release(); active.delete(owner); }
  }
  scan();
  if (view?.MutationObserver) { observer = new view.MutationObserver(scan); observer.observe(scope, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-cui-date-start','data-cui-date-end','data-cui-date-mode','type'] }); }
  return { refs: 1, release() { if (--this.refs > 0) return; stopped = true; observer?.disconnect(); for (const release of active.values()) release(); active.clear(); services.delete(scope); } };
}
export function install(root = globalThis.document) {
  if (!root) return () => {};
  let service = services.get(root); if (!service) { service = makeService(root); services.set(root, service); } else service.refs++;
  let released = false; return () => { if (released) return; released = true; service.release(); };
}
