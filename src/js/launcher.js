// The hotkey-launched quick-start pill: pick a source, toggle mic/camera/countdown, hit record.
// Runs as its own Tauri window; settings and the session state are the same shared modules the
// main window uses, read and written through localStorage like any other window of this app.

import { icon } from './ui.js';
import { invoke, inTauri, windowControl, resizeWindow } from './bridge.js';
import { settings, set, toggle, devices, setDevices, thumbs, session, subscribe } from './state.js';
import { start, cancelCountdown, connectBackend } from './session.js';

const $ = (id) => document.getElementById(id);
const sourcePopover = $('source-popover');
const timerPopover = $('timer-popover');
const morePopover = $('more-popover');
const COUNTDOWNS = ['Off', '3', '5', '10'];

function sourceLabel() {
  if (settings.mode === 'window') return devices.windows[settings.window]?.name ?? 'Window';
  return devices.displays[settings.display]?.name ?? 'Screen';
}

function paintBar() {
  $('source-icon').innerHTML = icon(settings.mode === 'window' ? 'app-window' : 'monitor');
  $('source-label').textContent = sourceLabel();
  $('mic-btn').innerHTML = icon(settings.mic ? 'mic' : 'mic-off');
  $('mic-btn').classList.toggle('off', !settings.mic);
  $('system-btn').innerHTML = icon(settings.system ? 'volume-2' : 'volume-x');
  $('system-btn').classList.toggle('off', !settings.system);
  $('cam-btn').innerHTML = icon(settings.camera ? 'webcam' : 'camera-off');
  $('cam-btn').classList.toggle('off', !settings.camera);
  $('timer-btn').innerHTML = icon('timer');
  $('timer-btn').classList.toggle('active', settings.countdown !== '0');
  $('timer-btn').setAttribute('aria-label', settings.countdown === '0' ? 'Countdown: off' : `Countdown: ${settings.countdown}s`);
  $('record-btn').innerHTML = icon('circle-dot');
  $('more-btn').innerHTML = icon('more-vertical');
  $('close-btn').innerHTML = icon('x');
}

const BAR_H = 56;
const PAD = 18;
const WIN_W = 480;
function resize() {
  if (!sourcePopover.hidden) {
    const popoverH = Math.max(sourcePopover.scrollHeight || sourcePopover.offsetHeight || 260, 260);
    const h = Math.min(PAD + BAR_H + 10 + popoverH + PAD, 480);
    resizeWindow(WIN_W, Math.round(h));
  } else if (!timerPopover.hidden) {
    resizeWindow(WIN_W, 210);
  } else if (!morePopover.hidden) {
    resizeWindow(WIN_W, 200);
  } else {
    resizeWindow(WIN_W, 92);
  }
}

function closePopovers() {
  sourcePopover.hidden = true;
  timerPopover.hidden = true;
  morePopover.hidden = true;
  $('source-btn').setAttribute('aria-expanded', 'false');
  resize();
}

function sourceRow(mode, i, s) {
  const src = thumbs[`${mode}:${i}`];
  return `<button class="src-row" data-mode="${mode}" data-i="${i}">
    <span class="thumb">${src ? `<img src="${src}" alt="" />` : ''}</span>
    <span class="meta"><b>${s.name}</b><small>${s.detail}</small></span>
  </button>`;
}

function bindSourceRows() {
  sourcePopover.querySelectorAll('.src-row').forEach((btn) => {
    btn.addEventListener('click', () => {
      const mode = btn.dataset.mode;
      set('mode', mode);
      set(mode, Number(btn.dataset.i));
      paintBar();
      closePopovers();
    });
  });
}

async function loadThumbs() {
  const jobs = [...devices.displays.map((_, i) => ['display', i]), ...devices.windows.map((_, i) => ['window', i])];
  for (const [mode, index] of jobs) {
    if (thumbs[`${mode}:${index}`]) continue;
    try {
      const dataUrl = await invoke('capture_thumbnail', { mode, index });
      if (!dataUrl) continue;
      thumbs[`${mode}:${index}`] = dataUrl;
      const cell = sourcePopover.querySelector(`.src-row[data-mode="${mode}"][data-i="${index}"] .thumb`);
      if (cell) cell.innerHTML = `<img src="${dataUrl}" alt="" />`;
    } catch (_) { /* the source may have closed since list_sources ran */ }
  }
}

async function openSourcePopover() {
  timerPopover.hidden = true;
  morePopover.hidden = true;
  if (!sourcePopover.hidden) { closePopovers(); return; }
  sourcePopover.hidden = false;
  $('source-btn').setAttribute('aria-expanded', 'true');
  const found = await invoke('list_sources');
  if (found) setDevices(found);
  $('screens-group').hidden = devices.displays.length === 0;
  $('windows-group').hidden = devices.windows.length === 0;
  $('screens-list').innerHTML = devices.displays.map((s, i) => sourceRow('display', i, s)).join('');
  $('windows-list').innerHTML = devices.windows.map((s, i) => sourceRow('window', i, s)).join('');
  bindSourceRows();
  requestAnimationFrame(() => {
    resize();
    loadThumbs();
  });
}

function renderTimerChoices() {
  $('timer-choices').innerHTML = COUNTDOWNS.map((c) => {
    const val = c === 'Off' ? '0' : c;
    return `<button class="choice" aria-pressed="${settings.countdown === val}" data-val="${val}">${c}</button>`;
  }).join('');
  $('timer-choices').querySelectorAll('.choice').forEach((b) => {
    b.addEventListener('click', () => { set('countdown', b.dataset.val); paintBar(); closePopovers(); });
  });
}

function openTimerPopover() {
  sourcePopover.hidden = true;
  morePopover.hidden = true;
  if (!timerPopover.hidden) { closePopovers(); return; }
  renderTimerChoices();
  timerPopover.hidden = false;
  requestAnimationFrame(() => resize());
}

function openMorePopover() {
  sourcePopover.hidden = true;
  timerPopover.hidden = true;
  if (!morePopover.hidden) { closePopovers(); return; }
  morePopover.hidden = false;
  requestAnimationFrame(() => resize());
}

$('source-btn').addEventListener('click', openSourcePopover);
$('mic-btn').addEventListener('click', () => { toggle('mic'); paintBar(); });
$('system-btn').addEventListener('click', () => { toggle('system'); paintBar(); });
$('cam-btn').addEventListener('click', () => { toggle('camera'); paintBar(); });
$('timer-btn').addEventListener('click', openTimerPopover);
$('more-btn').addEventListener('click', openMorePopover);
$('menu-folder').addEventListener('click', () => { invoke('reveal_in_folder', { path: settings.folder }); closePopovers(); });
$('menu-main').addEventListener('click', () => { invoke('show_main_window'); closePopovers(); });
$('close-btn').addEventListener('click', () => windowControl('close'));
$('record-btn').addEventListener('click', () => {
  if (session.counting || session.recording) return;
  closePopovers();
  start();
});

document.addEventListener('click', (e) => {
  if (!sourcePopover.hidden && !e.target.closest('#source-btn, #source-popover')) closePopovers();
  else if (!timerPopover.hidden && !e.target.closest('#timer-btn, #timer-popover')) closePopovers();
  else if (!morePopover.hidden && !e.target.closest('#more-btn, #more-popover')) closePopovers();
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    if (!sourcePopover.hidden || !timerPopover.hidden || !morePopover.hidden) {
      closePopovers();
    } else if (session.counting) {
      cancelCountdown();
    } else {
      windowControl('close');
    }
  }
});

subscribe((kind) => {
  if (kind === 'error') {
    paintBar();
    return;
  }
  if (kind !== 'state' && kind !== 'tick') return;
  $('record-btn').classList.toggle('counting', !!session.counting);
  if (session.recording) windowControl('close');
});

paintBar();
resize();
if (inTauri) connectBackend();
