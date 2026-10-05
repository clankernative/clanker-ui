// Explicit browser conformance simulator: no fetch, upload, persistence, or Native admission.
import * as modal from './components/modal/interaction.js';
import * as drawer from './components/drawer/interaction.js';
import * as popover from './components/popover/interaction.js';
import * as commandMenu from './components/command-menu/interaction.js';
import * as confirmDialog from './components/confirm-dialog/interaction.js';
import * as calendar from './components/date-calendar/interaction.js';
import * as picker from './components/date-picker/interaction.js';
import * as upload from './components/file-upload/interaction.js';
import * as viewport from './components/data-viewport/interaction.js';

const state = { fileSelections: [], chooserCancellations: 0, confirmationIntents: 0, windows: [], navigation: [], dates: [], failures: [], failSelection: false, failWindow: false, failConfirmation: false, failNavigation: false };
const output = document.getElementById('simulator-state');
const render = () => { output.textContent = `Explicit test adapter state (no backend transport):\n${JSON.stringify(state, null, 2)}`; };
const delay = signal => new Promise((resolve, reject) => {
  if (signal?.aborted) { reject(new DOMException('Simulator cancelled', 'AbortError')); return; }
  const timer = setTimeout(() => { signal?.removeEventListener('abort', abort); resolve(); }, 30);
  const abort = () => { clearTimeout(timer); reject(new DOMException('Simulator cancelled', 'AbortError')); };
  signal?.addEventListener('abort', abort, { once: true });
});
// Supply a real browser form solely to test validation and original submitter
// association. Its submit event is intercepted by this explicitly labeled fixture.
const confirmation = document.querySelector('[data-cui-component="confirm-dialog"]');
const original = confirmation?.querySelector('[data-cui-confirm-trigger]');
const formId = original?.getAttribute('form');
if (formId && !document.getElementById(formId)) {
  const form = document.createElement('form'); form.id = formId;
  const label = document.createElement('label'); label.textContent = 'Simulator confirmation reason (required)';
  const input = document.createElement('input'); input.name = 'reason'; input.required = true; input.value = 'Illustrative fixture'; label.append(input);
  form.append(label); confirmation.before(form);
}
if (formId) document.getElementById(formId)?.addEventListener('submit', event => { event.preventDefault(); state.confirmationIntents++; render(); });
const disposers = [];
disposers.push(modal.install(document), drawer.install(document), popover.install(document), commandMenu.install(document), calendar.install(document), picker.install(document));
disposers.push(confirmDialog.install(document, { submit: async request => {
  await delay(request.signal);
  if (state.failConfirmation) { state.failConfirmation = false; state.failures.push('Simulated confirmation adapter failure'); render(); throw new Error('Explicit simulator failure'); }
  request.form.requestSubmit(request.submitter);
} }));
disposers.push(upload.install(document, {
  selection: async request => {
    state.fileSelections.push(request.files.map(file => ({ name: file.name, size: file.size, type: file.type }))); render();
    await delay(request.signal);
    if (state.failSelection) { state.failSelection = false; state.failures.push('Simulated selection handoff failure'); render(); throw new Error('Explicit simulator failure'); }
    render();
  },
  cancel: request => { if (!request.signal.aborted) { state.chooserCancellations++; render(); } },
}));
disposers.push(viewport.install(document, {
  window: async request => {
    state.windows.push({ start: request.start, end: request.end, total: request.total }); render();
    await delay(request.signal);
    if (state.failWindow) { state.failWindow = false; state.failures.push('Simulated window adapter failure'); render(); throw new Error('Explicit simulator failure'); }
    render();
  },
  navigate: async request => {
    await delay(request.signal);
    if (state.failNavigation) { state.failNavigation = false; state.failures.push('Simulated navigation adapter failure'); render(); throw new Error('Explicit simulator failure'); }
    state.navigation.push({ control: request.control?.tagName ?? null, href: request.control?.getAttribute('href') ?? null, window: request.window ?? null }); render();
  },
}));
for (const name of ['cui:date-calendar-changed', 'cui:date-picker-changed']) document.addEventListener(name, event => { state.dates.push({ name, detail: event.detail }); render(); });
globalThis.componentProof = {
  state,
  failNext(kind) { if (!['Selection', 'Window', 'Confirmation', 'Navigation'].includes(kind)) throw new Error('Unknown simulator failure'); state[`fail${kind}`] = true; render(); },
  cleanup() { for (const dispose of disposers.reverse()) dispose(); render(); },
};
render();
