/** @typedef {string & {readonly __isoDate: unique symbol}} ISODate */
/**
 * Stable, bounded selection output. `start` is always a strict Gregorian ISO date;
 * a range's `end` is null until its second endpoint is chosen. This is browser
 * draft data only; the consuming app remains authoritative for validation.
 * @typedef {{mode: 'single'|'range', start: ISODate, end: ISODate|null, complete: boolean}} CalendarSelectionDetail
 */
const ROOT = '[data-cui-date-calendar][data-cui-date-interactive="true"]';
const ISO = /^(\d{4})-(\d{2})-(\d{2})$/;
const services = new WeakMap();
const calendars = new WeakMap();

function parseDate(value) {
  const match = ISO.exec(value || '');
  if (!match || Number(match[1]) < 1) return null;
  const date = new Date(0);
  date.setUTCHours(0, 0, 0, 0);
  date.setUTCFullYear(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return date.getUTCFullYear() === Number(match[1]) && date.getUTCMonth() === Number(match[2]) - 1 && date.getUTCDate() === Number(match[3]) ? date : null;
}
function iso(date) { return `${String(date.getUTCFullYear()).padStart(4, '0')}-${String(date.getUTCMonth() + 1).padStart(2, '0')}-${String(date.getUTCDate()).padStart(2, '0')}`; }
function shiftMonth(date, amount) {
  const year = date.getUTCFullYear(), index = year * 12 + date.getUTCMonth() + amount;
  if (index < 12 || index > 9999 * 12 + 11) return null;
  const result = new Date(0); result.setUTCHours(0, 0, 0, 0); result.setUTCFullYear(Math.floor(index / 12), index % 12, 1); return result;
}
function shiftDay(date, amount) {
  const result = new Date(date); result.setUTCDate(result.getUTCDate() + amount);
  return result.getUTCFullYear() < 1 || result.getUTCFullYear() > 9999 ? null : result;
}
function monthEnd(date) { const next = shiftMonth(date, 1); return next ? shiftDay(next, -1) : parseDate('9999-12-31'); }
function jsonLabels(raw, count) {
  try { const parsed = JSON.parse(raw || ''); return Array.isArray(parsed) && parsed.length === count && parsed.every((item) => typeof item === 'string' && item.trim() && !/[\u0000-\u001f\u007f]/.test(item)) ? parsed : null; } catch { return null; }
}
function nearest(el, selector) { return el?.closest?.(selector) || null; }
function belongs(root, node) { return !!node && (node === root || (root.contains(node) && nearest(node, ROOT) === root)); }
function config(root) {
  const month = parseDate(root.dataset.cuiDateMonth);
  const first = Number(root.dataset.cuiDateFirstDay);
  const months = jsonLabels(root.dataset.cuiDateMonthLabels, 12);
  const short = jsonLabels(root.dataset.cuiDateWeekdayShort, 7);
  const full = jsonLabels(root.dataset.cuiDateWeekdayFull, 7);
  if (!month || month.getUTCDate() !== 1 || !Number.isInteger(first) || first < 0 || first > 6 || !months || !short || !full) return null;
  return { month, first, months, short, full };
}
function makeCalendar(root) {
  let disposed = false, configKey = '', renderedKey = '', grid, label, previous, next, bound = null;
  const doc = root.ownerDocument, view = doc?.defaultView;
  if (!view?.AbortController || !view?.CustomEvent) return { update() {}, dispose() { disposed = true; } };
  let controller = new view.AbortController();

  function enabled(date) {
    if (!date || root.dataset.cuiDateDisabled === 'true') return false;
    const value = iso(date), min = root.dataset.cuiDateMinimum || '', max = root.dataset.cuiDateMaximum || '';
    return !(min && value < min) && !(max && value > max) && !unavailable().has(value);
  }
  function unavailable() { return new Set([...root.querySelectorAll('[data-cui-date-unavailable]')].filter((el) => nearest(el, ROOT) === root).map((el) => el.getAttribute('value') || el.value || '')); }
  function monthHasEnabled(date) {
    if (!date || root.dataset.cuiDateDisabled === 'true') return false;
    const first = iso(date), last = iso(monthEnd(date)), min = root.dataset.cuiDateMinimum || '', max = root.dataset.cuiDateMaximum || '';
    if ((max && max < first) || (min && min > last)) return false;
    const blocked = unavailable(), low = min && min > first ? min : first, high = max && max < last ? max : last;
    let cursor = parseDate(low);
    for (let i = 0; cursor && iso(cursor) <= high && i < 32; i++, cursor = shiftDay(cursor, 1)) if (enabled(cursor)) return true;
    return false;
  }
  function currentRefs() {
    const newGrid = root.querySelector('[data-cui-date-grid-body]'), newLabel = root.querySelector('[data-cui-date-month-label]');
    const newPrevious = root.querySelector('[data-cui-date-previous]'), newNext = root.querySelector('[data-cui-date-next]');
    if (!newGrid || !newLabel || !newPrevious || !newNext) return false;
    if (grid !== newGrid || label !== newLabel || previous !== newPrevious || next !== newNext) {
      controller.abort(); controller = new view.AbortController();
      grid = newGrid; label = newLabel; previous = newPrevious; next = newNext;
      root.addEventListener('click', onClick, { signal: controller.signal });
      root.addEventListener('keydown', onKey, { signal: controller.signal });
      root.addEventListener('cui:date-picker-refresh', onRefresh, { signal: controller.signal });
    }
    return true;
  }
  function chooseFocus(preferred, cells) {
    if (preferred && cells.some((x) => x.date && iso(x.date) === preferred && enabled(x.date))) return preferred;
    const candidates = cells.filter((x) => x.date && enabled(x.date));
    const selected = root.dataset.cuiDateSelectionStart || root.dataset.cuiDateToday || '';
    return candidates.find((x) => iso(x.date) === selected)?.value || candidates.find((x) => x.inMonth)?.value || candidates[0]?.value || '';
  }
  function draw(preferred) {
    if (disposed || !currentRefs()) return;
    const c = config(root); if (!c) return;
    const visible = c.month, value = iso(visible), offset = (visible.getUTCDay() - c.first + 7) % 7;
    label.textContent = `${c.months[visible.getUTCMonth()]} ${visible.getUTCFullYear()}`;
    const cells = [];
    for (let i = 0; i < 42; i++) { const date = shiftDay(visible, i - offset); cells.push({ date, value: date ? iso(date) : '', inMonth: !!date && date.getUTCMonth() === visible.getUTCMonth() && date.getUTCFullYear() === visible.getUTCFullYear() }); }
    const roving = chooseFocus(preferred || bound, cells), fragment = doc.createDocumentFragment();
    const startValue = root.dataset.cuiDateSelectionStart || '', endValue = root.dataset.cuiDateSelectionEnd || '', mode = root.dataset.cuiDateSelectionMode || 'single';
    for (let week = 0; week < 6; week++) {
      const row = doc.createElement('tr');
      for (let day = 0; day < 7; day++) {
        const cell = cells[week * 7 + day], td = doc.createElement('td');
        td.className = 'cui-date-calendar__cell';
        if (!cell.date) { td.setAttribute('aria-hidden', 'true'); row.append(td); continue; }
        const d = cell.date, val = cell.value, unavailableDate = !enabled(d), isStart = val === startValue, isEnd = !!endValue && val === endValue;
        const selected = isStart || isEnd, inRange = mode === 'range' && !!startValue && !!endValue && val >= startValue && val <= endValue;
        for (const [name, on] of [['outside', !cell.inMonth], ['today', val === root.dataset.cuiDateToday], ['selected', selected], ['range-start', isStart], ['range-end', isEnd], ['in-range', inRange], ['unavailable', unavailableDate]]) if (on) td.classList.add(`cui-date-calendar__cell--${name}`);
        td.setAttribute('role', 'gridcell'); td.setAttribute('aria-selected', String(inRange || selected));
        if (val === root.dataset.cuiDateToday) td.setAttribute('aria-current', 'date');
        const button = doc.createElement('button'); button.type = 'button'; button.className = 'cui-date-calendar__day'; button.dataset.cuiDateDay = ''; button.dataset.cuiDateValue = val;
        button.textContent = String(d.getUTCDate()); button.disabled = unavailableDate; button.tabIndex = val === roving && !unavailableDate ? 0 : -1;
        button.setAttribute('aria-label', `${c.full[d.getUTCDay()]} ${d.getUTCDate()} ${c.months[d.getUTCMonth()]} ${d.getUTCFullYear()}`); td.append(button); row.append(td);
      }
      fragment.append(row);
    }
    grid.replaceChildren(fragment); bound = roving || '';
    const prevMonth = shiftMonth(visible, -1), nextMonth = shiftMonth(visible, 1);
    previous.disabled = !monthHasEnabled(prevMonth); next.disabled = !monthHasEnabled(nextMonth);
    configKey = value;
    renderedKey = [root.dataset.cuiDateSelectionStart, root.dataset.cuiDateSelectionEnd, root.dataset.cuiDateSelectionMode, root.dataset.cuiDateDisabled, root.dataset.cuiDateMinimum, root.dataset.cuiDateMaximum, [...unavailable()].sort().join(',')].join('|');
  }
  function onRefresh() { draw(); }
  function moveMonth(date, amount) {
    const targetMonth = shiftMonth(date, amount); if (!targetMonth) return null;
    const day = date.getUTCDate(), candidate = new Date(targetMonth); candidate.setUTCDate(Math.min(day, monthEnd(targetMonth).getUTCDate())); return candidate;
  }
  function focusDate(date) {
    if (!date || !enabled(date)) return;
    const targetMonth = new Date(date); targetMonth.setUTCDate(1);
    if (iso(config(root)?.month || targetMonth) !== iso(targetMonth)) root.dataset.cuiDateMonth = iso(targetMonth);
    draw(iso(date)); root.querySelector(`[data-cui-date-day][data-cui-date-value="${iso(date)}"]`)?.focus();
  }
  function seek(date, direction) {
    let cursor = date;
    for (let i = 0; cursor && i < 366; i++) { if (enabled(cursor)) return cursor; cursor = shiftDay(cursor, direction); }
    return null;
  }
  function select(date) {
    if (!enabled(date)) return;
    const mode = root.dataset.cuiDateSelectionMode || 'single', selected = iso(date);
    let start = selected, end = '', complete = true;
    if (mode === 'range') {
      const oldStart = parseDate(root.dataset.cuiDateSelectionStart), oldEnd = parseDate(root.dataset.cuiDateSelectionEnd);
      if (oldStart && !oldEnd) { if (date < oldStart) { start = selected; end = iso(oldStart); } else { start = iso(oldStart); end = selected; } }
      else complete = false;
    }
    root.dataset.cuiDateSelectionStart = start; root.dataset.cuiDateSelectionEnd = end;
    if (!config(root)?.month || config(root).month.getUTCMonth() !== date.getUTCMonth() || config(root).month.getUTCFullYear() !== date.getUTCFullYear()) root.dataset.cuiDateMonth = `${selected.slice(0, 7)}-01`;
    draw(selected); root.querySelector(`[data-cui-date-day][data-cui-date-value="${selected}"]`)?.focus();
    /** @type {CalendarSelectionDetail} */
    const detail = { complete, end: end || null, mode, start };
    root.dispatchEvent(new view.CustomEvent('cui:date-calendar-changed', { bubbles: true, detail }));
  }
  function onClick(event) {
    const button = nearest(event.target, 'button'); if (!belongs(root, button)) return;
    if (button === previous || button === next) {
      const c = config(root), amount = button === previous ? -1 : 1, target = c && shiftMonth(c.month, amount);
      if (target && monthHasEnabled(target)) { const active = parseDate(bound) || c.month; const candidate = moveMonth(active, amount); root.dataset.cuiDateMonth = iso(target); draw(candidate && enabled(candidate) ? iso(candidate) : undefined); root.querySelector('[data-cui-date-day][tabindex="0"]')?.focus(); }
      return;
    }
    if (button.matches('[data-cui-date-day]') && !button.disabled) { const date = parseDate(button.dataset.cuiDateValue); if (date) select(date); }
  }
  function onKey(event) {
    const button = nearest(event.target, '[data-cui-date-day]'); if (!belongs(root, button)) return;
    const date = parseDate(button.dataset.cuiDateValue); if (!date) return;
    let target = null, direction = 1;
    switch (event.key) {
      case 'ArrowLeft': direction = view.getComputedStyle(root).direction === 'rtl' ? 1 : -1; target = shiftDay(date, direction); break;
      case 'ArrowRight': direction = view.getComputedStyle(root).direction === 'rtl' ? -1 : 1; target = shiftDay(date, direction); break;
      case 'ArrowUp': direction = -1; target = shiftDay(date, -7); break;
      case 'ArrowDown': target = shiftDay(date, 7); break;
      case 'Home': direction = -1; target = shiftDay(date, -((date.getUTCDay() - (config(root)?.first || 0) + 7) % 7)); break;
      case 'End': target = shiftDay(date, 6 - ((date.getUTCDay() - (config(root)?.first || 0) + 7) % 7)); break;
      case 'PageUp': direction = -1; target = moveMonth(date, event.shiftKey ? -12 : -1); break;
      case 'PageDown': target = moveMonth(date, event.shiftKey ? 12 : 1); break;
      default: return;
    }
    event.preventDefault(); target = seek(target, direction);
    if (target) focusDate(target);
  }
  function update() {
    if (disposed) return;
    const c = config(root); if (!c) { controller.abort(); grid = label = previous = next = null; delete root.dataset.cuiDateCalendarReady; return; }
    const refsChanged = !grid || grid !== root.querySelector('[data-cui-date-grid-body]') || label !== root.querySelector('[data-cui-date-month-label]') || previous !== root.querySelector('[data-cui-date-previous]') || next !== root.querySelector('[data-cui-date-next]');
    const key = [root.dataset.cuiDateSelectionStart, root.dataset.cuiDateSelectionEnd, root.dataset.cuiDateSelectionMode, root.dataset.cuiDateDisabled, root.dataset.cuiDateMinimum, root.dataset.cuiDateMaximum, [...unavailable()].sort().join(',')].join('|');
    const changed = iso(c.month) !== configKey || key !== renderedKey;
    if (refsChanged || changed) draw(refsChanged ? undefined : bound);
  }
  update();
  if (grid && config(root)) root.dataset.cuiDateCalendarReady = '';
  return { update, dispose() { if (disposed) return; disposed = true; controller.abort(); if (root.dataset.cuiDateCalendarReady !== undefined) delete root.dataset.cuiDateCalendarReady; } };
}
function acquireCalendar(root) {
  let record = calendars.get(root);
  if (!record) { record = { refs: 0, controller: makeCalendar(root) }; calendars.set(root, record); }
  record.refs++;
  let released = false;
  return () => { if (released) return; released = true; if (--record.refs === 0) { record.controller.dispose(); calendars.delete(root); } };
}
function isInScope(scope, el) { return (el === scope || scope.contains?.(el)) && nearest(el, ROOT) === el; }
function makeService(scope) {
  const doc = scope.nodeType === 9 ? scope : scope.ownerDocument, view = doc?.defaultView;
  const active = new Map(); let stopped = false, observer;
  function scan() {
    if (stopped) return;
    const candidates = [];
    if (scope.matches?.(ROOT)) candidates.push(scope);
    for (const el of scope.querySelectorAll?.(ROOT) || []) candidates.push(el);
    const desired = new Set(candidates.filter((el) => isInScope(scope, el)));
    for (const el of desired) { if (!active.has(el)) active.set(el, acquireCalendar(el)); else calendars.get(el)?.controller.update(); }
    for (const [el, release] of active) if (!desired.has(el)) { release(); active.delete(el); }
  }
  scan();
  if (view?.MutationObserver) { observer = new view.MutationObserver(scan); observer.observe(scope, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-cui-date-month','data-cui-date-first-day','data-cui-date-month-labels','data-cui-date-weekday-short','data-cui-date-weekday-full','data-cui-date-selection-start','data-cui-date-selection-end','data-cui-date-selection-mode','data-cui-date-disabled','data-cui-date-minimum','data-cui-date-maximum'] }); }
  return { refs: 1, release() { if (--this.refs > 0) return; stopped = true; observer?.disconnect(); for (const release of active.values()) release(); active.clear(); services.delete(scope); } };
}
export function install(root = globalThis.document) {
  if (!root) return () => {};
  let service = services.get(root); if (!service) { service = makeService(root); services.set(root, service); } else service.refs++;
  let released = false; return () => { if (released) return; released = true; service.release(); };
}
export { parseDate as parseCalendarDate };
