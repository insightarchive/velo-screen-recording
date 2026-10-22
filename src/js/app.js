import { icon, hydrate, fmtTime } from './ui.js';
import { inTauri, invoke, listen, windowControl } from './bridge.js';
import { settings, set, toggle, resetSettings, session, library, removeRecording, editor, ui, subscribe, devices, setDevices, preview, thumbs, videoThumbs } from './state.js';
import * as sess from './session.js';
import { connectBackend } from './session.js';
import { view as recordView } from './views/record.js';
import { view as libraryView } from './views/library.js';
import { view as editorView, paintCanvas, bindVideo } from './views/editor.js';
import { view as settingsView } from './views/settings.js';

const ROUTES = {
  record: { title: 'Record', view: recordView },
  library: { title: 'Library', view: libraryView },
  editor: { title: 'Editor', view: editorView },
  settings: { title: 'Settings', view: settingsView },
};
const EDITOR_KEYS = new Set(['bg', 'pad', 'radius', 'shadow', 'cursorSize', 'ring', 'smooth', 'format', 'res']);

const $ = (id) => document.getElementById(id);
const root = document.documentElement;
const page = $('page');
let route = 'record';

// ---------- Rendering ----------
function paintShell() {
  $('page-name').textContent = ROUTES[route].title;
  document.querySelectorAll('[data-route]').forEach((b) => {
    if (b.dataset.route === route) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
  });
  const lib = $('nav-library').querySelector('.count');
  if (lib) lib.textContent = library.length || '';
  $('save-path').textContent = settings.folder;
  $('rec-pill').innerHTML = session.recording
    ? `<span class="pill rec"><span class="dot"></span><span data-timer>${fmtTime(session.seconds)}</span>${session.paused ? ' &middot; Paused' : ''}</span>`
    : '';
  $('theme-btn').innerHTML = icon(root.dataset.theme === 'dark' ? 'sun' : 'moon');
}

function render() {
  const active = document.activeElement;
  const wasSearch = active?.matches?.('[data-search]');
  const caret = wasSearch ? active.selectionStart : 0;
  page.innerHTML = ROUTES[route].view();
  paintShell();
  if (wasSearch) { const el = page.querySelector('[data-search]'); el?.focus(); el?.setSelectionRange(caret, caret); }
  syncPreview();
  if (route === 'library') loadVideoThumbs();
  if (route === 'editor') { bindVideo(); loadEditorProject(); }
}

// ---------- Editor project loading (Tauri only) ----------
async function loadEditorProject() {
  if (!inTauri) return;
  const rec = library.find((r) => r.id === ui.editorId) ?? library[0];
  if (!rec || editor.projectFor === rec.id) return;
  if (!rec.projectPath) { editor.project = null; editor.projectFor = rec.id; return; }
  try {
    editor.project = await invoke('load_project', { path: rec.projectPath });
    editor.projectFor = rec.id;
    render();
  } catch (_) { editor.project = null; editor.projectFor = rec.id; }
}

// ---------- Library video thumbnails (Tauri only) ----------
const fetchingThumbs = new Set();
function loadVideoThumbs() {
  if (!inTauri) return;
  for (const r of library) {
    if (!r.path || videoThumbs[r.path] || fetchingThumbs.has(r.path)) continue;
    fetchingThumbs.add(r.path);
    invoke('video_thumbnail', { path: r.path })
      .then((dataUrl) => { if (dataUrl) { videoThumbs[r.path] = dataUrl; render(); } })
      .catch(() => {})
      .finally(() => fetchingThumbs.delete(r.path));
  }
}

// ---------- Live source preview (Tauri only) ----------
// Streams the real selected display/window into the Preview panel instead of mock art.
let previewKey = null;
async function syncPreview() {
  if (!inTauri) return;
  const busy = session.recording || session.counting || session.saving;
  const canStream = !busy && route === 'record' && (settings.mode === 'display' || settings.mode === 'window');
  const index = settings.mode === 'window' ? settings.window : settings.display;
  const key = canStream ? `${settings.mode}:${index}` : null;
  if (key === previewKey) return;
  const hadPreview = previewKey !== null;
  previewKey = key;
  if (hadPreview) { preview.src = ''; await invoke('stop_preview'); }
  if (key) {
    try { await invoke('start_preview', { mode: settings.mode, index }); } catch (_) { previewKey = null; }
  }
}

function go(name) {
  route = ROUTES[name] ? name : 'record';
  if (location.hash !== `#/${route}`) history.replaceState(null, '', `#/${route}`);
  render();
  $('page').parentElement.scrollTop = 0;
}

function applyTheme(theme) {
  root.dataset.theme = theme;
  set('theme', theme);
}

let toastTimer;
function toast(msg) {
  const el = $('toast');
  el.textContent = msg;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2600);
}
const SOON = 'This works once the recording engine is connected.';

// ---------- Input handling ----------
const assign = (key, value) => (EDITOR_KEYS.has(key) ? (editor[key] = value) : set(key, value));

const ACTIONS = {
  start: () => sess.start(),
  stop: () => sess.stop(),
  pause: () => sess.togglePause(),
  cancel: () => sess.cancelCountdown(),
  edit: (el) => { ui.editorId = el.dataset.id; go('editor'); },
  delete: (el) => { removeRecording(el.dataset.id); render(); },
  reveal: (el) => { const r = library.find((x) => x.id === el.dataset.id); invoke('reveal_in_folder', { path: r?.path }); toast(SOON); },
  'reveal-folder': () => { invoke('reveal_in_folder', { path: settings.folder }); toast(SOON); },
  reset: () => { resetSettings(); applyTheme(settings.theme); render(); },
  play: () => { const v = $('editor-video'); if (v) { v.paused ? v.play() : v.pause(); } },
  export: () => toast(SOON),
  browse: () => toast(SOON),
};

document.addEventListener('click', (e) => {
  const el = e.target.closest('button');
  if (!el || el.disabled) return;
  if (el.dataset.route || el.dataset.go) return go(el.dataset.route || el.dataset.go);
  if (el.dataset.tab) { ui.settingsTab = el.dataset.tab; return render(); }
  if (el.dataset.act) { ACTIONS[el.dataset.act]?.(el); return; }
  if (el.dataset.toggle) {
    const key = el.dataset.toggle;
    if (EDITOR_KEYS.has(key)) editor[key] = !editor[key]; else toggle(key);
    return render();
  }
  if (el.dataset.set) {
    const key = el.dataset.set;
    let val = el.dataset.val;
    if (el.hasAttribute('data-num')) val = Number(val);
    if (key === 'countdown') val = val === 'Off' ? '0' : val;
    if (key === 'theme') { val = val.toLowerCase(); applyTheme(val); return render(); }
    assign(key, val);
    if (key === 'onClose') invoke('set_close_to_tray', { enabled: val === 'Hide to tray' });
    if (key === 'mode' && val === 'window') refreshSources();
    render();
  }
});

document.addEventListener('change', (e) => {
  const el = e.target;
  if (el.dataset.select) {
    if (el.dataset.select === 'sort') ui.sort = el.selectedOptions[0].text;
    else assign(el.dataset.select, el.dataset.select === 'encoder' ? el.selectedOptions[0].text : Number(el.value));
    render();
  }
});

document.addEventListener('input', (e) => {
  const el = e.target;
  if (el.dataset.search !== undefined) { ui.search = el.value; return render(); }
  if (el.dataset.text) return set(el.dataset.text, el.value);
  if (el.dataset.range) {
    const key = el.dataset.range;
    assign(key, Number(el.value));
    const out = document.querySelector(`[data-out="${key}"]`);
    if (out) out.textContent = `${el.value}%`;
    if (EDITOR_KEYS.has(key)) paintCanvas();
    return;
  }
  if (el.dataset.trim !== undefined) {
    const i = Number(el.dataset.trim);
    const v = Number(el.value);
    editor.trim[i] = i === 0 ? Math.min(v, editor.trim[1] - 2) : Math.max(v, editor.trim[0] + 2);
    el.value = editor.trim[i];
    const keep = document.querySelector('.track .keep');
    if (keep) { keep.style.left = `${editor.trim[0]}%`; keep.style.width = `${editor.trim[1] - editor.trim[0]}%`; }
  }
});

document.addEventListener('keydown', (e) => {
  if (e.altKey && e.shiftKey && e.code === 'KeyR') {
    e.preventDefault();
    if (session.recording) sess.stop(); else sess.start();
  }
});

$('theme-btn').addEventListener('click', () => { applyTheme(root.dataset.theme === 'dark' ? 'light' : 'dark'); render(); });
document.querySelectorAll('[data-win]').forEach((b) => b.addEventListener('click', () => windowControl(b.dataset.win)));
window.addEventListener('hashchange', () => go(location.hash.replace('#/', '')));

// ---------- Live updates ----------
subscribe((kind, payload) => {
  if (kind === 'error') {
    toast(payload);
  } else if (kind === 'tick') {
    document.querySelectorAll('[data-timer]').forEach((n) => { n.textContent = fmtTime(session.seconds); });
  } else {
    render();
  }
});

// Level meters. In the browser preview these are simulated; Tauri replaces them with real levels.
const levels = { mic: 0, system: 0 };
const real = { mic: 0, system: 0 };
setInterval(() => {
  document.querySelectorAll('[data-meter]').forEach((m) => {
    if (m.classList.contains('off')) { m.querySelectorAll('i').forEach((b) => { b.className = ''; }); return; }
    const id = m.dataset.meter;
    const target = inTauri ? Math.sqrt(real[id]) : Math.random() * (id === 'mic' ? 1 : 0.8);
    levels[id] = inTauri ? target : levels[id] * 0.6 + target * 0.4;
    const on = Math.round(levels[id] * 16 * 1.4);
    m.querySelectorAll('i').forEach((b, i) => { b.className = i < on ? (i > 13 ? 'max' : i > 10 ? 'hot' : 'on') : ''; });
  });
}, 120);

// ---------- Real devices and events (Tauri only) ----------
async function refreshSources() {
  const found = await invoke('list_sources');
  if (!found) return;
  setDevices(found);
  if (settings.display >= devices.displays.length) set('display', 0);
  if (settings.window >= devices.windows.length) set('window', 0);
  if (settings.micDevice >= devices.mics.length) set('micDevice', 0);
  render();
  loadThumbs();
}

/** Snapshots each display and window once, for the picker tiles. Windows can close mid-list; skip those. */
async function loadThumbs() {
  Object.keys(thumbs).forEach((k) => delete thumbs[k]);
  const jobs = [
    ...devices.displays.map((_, i) => ['display', i]),
    ...devices.windows.map((_, i) => ['window', i]),
  ];
  for (const [mode, index] of jobs) {
    try {
      const dataUrl = await invoke('capture_thumbnail', { mode, index });
      if (dataUrl) { thumbs[`${mode}:${index}`] = dataUrl; render(); }
    } catch (_) { /* the source may have closed since list_sources ran */ }
  }
}

async function initBackend() {
  if (!inTauri) return;
  await connectBackend();
  await listen('levels', (l) => { real.mic = l.mic; real.system = l.system; });
  await listen('preview-frame', ({ dataUrl }) => {
    preview.src = dataUrl;
    const img = $('preview-frame');
    if (img) img.src = dataUrl; else render();
  });
  invoke('set_close_to_tray', { enabled: settings.onClose === 'Hide to tray' });
  refreshSources();
}

hydrate();
go(location.hash.replace('#/', '') || 'record');
initBackend();
