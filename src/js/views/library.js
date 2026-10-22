import { icon, esc, fmtShort, fmtSize, screenArt, select } from '../ui.js';
import { library, ui, videoThumbs } from '../state.js';

function thumbArt(r) {
  const src = videoThumbs[r.path];
  return src ? `<div class="screen screen-thumb"><img class="thumb-frame" src="${src}" alt="" /></div>` : screenArt({});
}

const SORTS = ['Newest', 'Oldest', 'Longest', 'Largest'];

function sorted(list) {
  const by = {
    Newest: (a, b) => b.createdAt - a.createdAt,
    Oldest: (a, b) => a.createdAt - b.createdAt,
    Longest: (a, b) => b.seconds - a.seconds,
    Largest: (a, b) => b.size - a.size,
  }[ui.sort];
  return [...list].sort(by);
}

const when = (t) => new Date(t).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) +
  ', ' + new Date(t).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });

export function view() {
  const q = ui.search.trim().toLowerCase();
  const list = sorted(library.filter((r) => r.name.toLowerCase().includes(q)));

  const body = list.length
    ? `<div class="grid library">${list.map((r) => `
        <article class="card rec-card">
          <div style="position:relative">${thumbArt(r)}<span class="dur">${fmtShort(r.seconds)}</span></div>
          <div><h3 title="${esc(r.name)}">${esc(r.name)}</h3><div class="sub">${when(r.createdAt)} &middot; ${fmtSize(r.size)}</div></div>
          <div class="acts">
            <button class="btn sm primary" data-act="edit" data-id="${r.id}">${icon('pencil')}Edit</button>
            <button class="btn sm" data-act="reveal" data-id="${r.id}">${icon('folder-open')}Show</button>
            <button class="btn sm" data-act="delete" data-id="${r.id}" aria-label="Delete">${icon('trash-2')}</button>
          </div>
        </article>`).join('')}</div>`
    : `<section class="card"><div class="empty">${icon('film')}
        <b>${library.length ? 'No recordings match your search' : 'No recordings yet'}</b>
        <span>${library.length ? 'Try a different name.' : 'Your finished recordings will show up here.'}</span>
        ${library.length ? '' : `<button class="btn primary" data-go="record">${icon('circle-dot')}Start recording</button>`}
      </div></section>`;

  return `
    <div class="page-head">
      <div class="page-title">
        <span class="chip-lg">${icon('library')}</span>
        <div><h1>Library</h1><p>${library.length} recording${library.length === 1 ? '' : 's'} saved on this PC.</p></div>
      </div>
    </div>
    <div class="toolbar">
      <label class="search">${icon('search')}<input class="field" type="search" placeholder="Search recordings" value="${esc(ui.search)}" aria-label="Search recordings" data-search /></label>
      ${select('sort', SORTS, SORTS.indexOf(ui.sort), 'Sort by')}
    </div>
    ${body}`;
}
