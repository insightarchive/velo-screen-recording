import { ICONS } from './icons.js';

/** Inline Lucide icon as an SVG string. */
export function icon(name, cls = '') {
  const body = ICONS[name];
  if (!body) throw new Error(`Unknown icon: ${name}`);
  return `<svg class="icon ${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
}

/** Fill every static [data-icon] placeholder (used for the app shell). */
export function hydrate(root = document) {
  root.querySelectorAll('[data-icon]').forEach((el) => {
    const label = el.dataset.label;
    el.innerHTML = label !== undefined
      ? `${icon(el.dataset.icon)}<span class="label">${label}</span>${el.dataset.count ? `<span class="count">${el.dataset.count}</span>` : ''}`
      : icon(el.dataset.icon);
    el.removeAttribute('data-icon');
  });
}

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

export const fmtTime = (s) => [Math.floor(s / 3600), Math.floor(s / 60) % 60, s % 60].map((n) => String(n).padStart(2, '0')).join(':');
export const fmtShort = (s) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
export const fmtSize = (mb) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(mb < 10 ? 1 : 0)} MB`);

// ---- Form controls. State changes flow through data-* attributes handled in app.js. ----

export const switchBtn = (key, on, label) =>
  `<button class="switch" role="switch" aria-label="${esc(label)}" aria-checked="${on}" data-toggle="${key}"></button>`;

export const seg = (key, options, value) =>
  `<div class="seg" role="group">${options.map((o) =>
    `<button aria-pressed="${o === value}" data-set="${key}" data-val="${esc(o)}">${esc(o)}</button>`).join('')}</div>`;

export const select = (key, options, index, label) =>
  `<select class="field" aria-label="${esc(label)}" data-select="${key}">${options.map((o, i) =>
    `<option value="${i}" ${i === index ? 'selected' : ''}>${esc(o)}</option>`).join('')}</select>`;

export const range = (key, min, max, value, label) =>
  `<input class="range" type="range" min="${min}" max="${max}" value="${value}" aria-label="${esc(label)}" data-range="${key}" />`;

export const row = ({ ic, title, sub = '', ctl = '' }) => `
  <div class="row">
    <div class="meta">${ic ? `<span class="chip">${icon(ic)}</span>` : ''}<div><b>${title}</b>${sub ? `<small>${sub}</small>` : ''}</div></div>
    <div class="ctl">${ctl}</div>
  </div>`;

export const card = ({ ic, title, hint = '', body, extra = '' }) => `
  <section class="card">
    <div class="card-head">
      <div class="card-title">${ic ? `<span class="chip">${icon(ic)}</span>` : ''}${title}</div>
      ${hint ? `<span class="card-hint">${hint}</span>` : ''}${extra}
    </div>
    ${body}
  </section>`;

export function meter(id, active) {
  return `<div class="meter ${active ? '' : 'off'}" data-meter="${id}" aria-hidden="true">${'<i></i>'.repeat(16)}</div>`;
}

/** A mock screen, used where no real capture is available yet (other sources, camera mode, the browser preview). */
export function screenArt({ badge = '', region = false, cam = '', live = '' } = {}) {
  return `<div class="screen">
    <div class="win w1"></div><div class="win w2"></div><div class="taskbar"></div>
    ${region ? '<div class="region"></div>' : ''}
    ${badge ? `<span class="badge">${esc(badge)}</span>` : ''}
    ${cam ? `<div class="cam ${cam === 'Square' ? 'square' : ''}"></div>` : ''}
    ${live}
  </div>`;
}

/** The real, live screen/window capture, streamed frame by frame from the Rust side. */
export function livePreview({ src = '', badge = '', cam = '', live = '' } = {}) {
  return `<div class="screen screen-live">
    ${src ? `<img class="preview-frame" id="preview-frame" src="${src}" alt="" />` : `<div class="screen-wait">${icon('monitor')}<span>Starting preview…</span></div>`}
    ${badge ? `<span class="badge">${esc(badge)}</span>` : ''}
    ${cam ? `<div class="cam ${cam === 'Square' ? 'square' : ''}"></div>` : ''}
    ${live}
  </div>`;
}
