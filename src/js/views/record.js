import { icon, card, row, seg, select, switchBtn, meter, screenArt, livePreview, esc, fmtTime } from '../ui.js';
import { settings, session, devices, preview, thumbs } from '../state.js';
import { inTauri } from '../bridge.js';

const MODES = [
  { id: 'display', ic: 'monitor', title: 'Display', sub: 'Whole screen' },
  { id: 'window', ic: 'app-window', title: 'Window', sub: 'One app' },
  { id: 'region', ic: 'crop', title: 'Region', sub: 'Drag an area' },
  { id: 'camera', ic: 'webcam', title: 'Camera', sub: 'Webcam only' },
];

/** A tile's thumbnail: the real snapshot once it has loaded, mock art until then. */
function tileArt(key, i) {
  const badge = key === 'window' ? '' : `${i + 1}`;
  const src = thumbs[`${key}:${i}`];
  if (!src) return screenArt({ badge });
  return `<div class="screen screen-thumb"><img class="thumb-frame" src="${src}" alt="" />${badge ? `<span class="badge">${badge}</span>` : ''}</div>`;
}

function sourceList() {
  const pick = (key, list, current) => `<div class="sources">${list.map((s, i) => `
    <button class="source" aria-pressed="${i === current}" data-set="${key}" data-val="${i}" data-num>
      ${tileArt(key, i)}
      <b>${esc(s.name)}</b><small>${esc(s.detail)}</small>
    </button>`).join('')}</div>`;
  if (settings.mode === 'display') return pick('display', devices.displays, settings.display);
  if (settings.mode === 'window') return pick('window', devices.windows, settings.window);
  if (settings.mode === 'region') {
    return row({ ic: 'crop', title: 'Shape', sub: 'Drag on screen after pressing Record', ctl: seg('aspect', ['Free', '16:9', '4:3', '1:1', '9:16'], settings.aspect) });
  }
  return row({ ic: 'webcam', title: 'Camera', ctl: select('cameraDevice', devices.cameras, settings.cameraDevice, 'Camera') });
}

function previewBadge() {
  if (settings.mode === 'display') return devices.displays[settings.display]?.name ?? 'No display found';
  if (settings.mode === 'window') return devices.windows[settings.window]?.name ?? 'No window found';
  if (settings.mode === 'region') return `Region ${settings.aspect}`;
  return devices.cameras[settings.cameraDevice] ?? 'No camera found';
}

/** The real capture when one is streaming (display/window, in Tauri); the mock art otherwise. */
function livePreviewOrArt(camOn) {
  const camProps = { cam: camOn || settings.mode === 'camera' ? settings.cameraShape : '', live: liveOverlay() };
  const canStream = inTauri && (settings.mode === 'display' || settings.mode === 'window');
  if (canStream) return livePreview({ src: preview.src, badge: previewBadge(), ...camProps });
  return screenArt({ badge: previewBadge(), region: settings.mode === 'region', ...camProps });
}

function liveOverlay() {
  if (session.counting) {
    return `<div class="live"><div><div class="time" data-count>${session.counting}</div><small>Recording starts</small></div></div>`;
  }
  if (session.recording) {
    return `<div class="live"><div><div class="time" data-timer>${fmtTime(session.seconds)}</div><small>${session.paused ? 'Paused' : 'Recording'}</small></div></div>`;
  }
  return '';
}

function startCard() {
  if (session.saving) {
    return card({ ic: 'film', title: 'Saving', body: `<div class="empty">${icon('film')}<b>Finishing your video</b><span>This takes a moment.</span></div>` });
  }
  if (session.counting) {
    return card({ ic: 'timer', title: 'Get ready', body: `<div class="record-cta"><button class="btn lg block" data-act="cancel">${icon('x')}Cancel</button></div>` });
  }
  if (session.recording) {
    return card({
      ic: 'circle-dot', title: 'Recording',
      body: `<div class="record-cta">
        <div class="live-ctl">
          <button class="btn lg" data-act="pause">${icon(session.paused ? 'play' : 'pause')}${session.paused ? 'Resume' : 'Pause'}</button>
          <button class="btn danger lg" data-act="stop">${icon('square')}Stop</button>
        </div>
        <div class="hint">Press <span class="kbd">Alt</span> + <span class="kbd">Shift</span> + <span class="kbd">R</span> to stop</div>
      </div>`,
    });
  }
  return card({
    ic: 'circle-dot', title: 'Ready to record', hint: `${settings.quality} &middot; ${settings.fps} fps`,
    body: `
      ${row({ ic: 'sparkles', title: 'Mode', sub: settings.recordMode === 'Studio' ? 'Keeps zoom and effects editable' : 'Saves a finished video right away', ctl: seg('recordMode', ['Studio', 'Quick'], settings.recordMode) })}
      ${row({ ic: 'timer', title: 'Countdown', ctl: seg('countdown', ['Off', '3', '5', '10'], settings.countdown === '0' ? 'Off' : settings.countdown) })}
      <div class="record-cta" style="margin-top:16px">
        <button class="btn danger lg block" data-act="start">${icon('circle-dot')}Start recording</button>
        <div class="hint">or press <span class="kbd">Alt</span> + <span class="kbd">Shift</span> + <span class="kbd">R</span></div>
      </div>`,
  });
}

export function view() {
  const busy = session.recording || session.counting || session.saving;
  const camOn = settings.camera && settings.mode !== 'camera';

  const modeTiles = `<div class="tiles">${MODES.map((m) => `
    <button class="tile" aria-pressed="${settings.mode === m.id}" data-set="mode" data-val="${m.id}" ${busy ? 'disabled' : ''}>
      <span class="chip">${icon(m.ic)}</span><span><b>${m.title}</b><small>${m.sub}</small></span>
    </button>`).join('')}</div>`;

  const inputs = card({
    ic: 'mic', title: 'Inputs',
    body: [
      row({ ic: settings.mic ? 'mic' : 'mic-off', title: 'Microphone', sub: esc(devices.mics[settings.micDevice]), ctl: `${meter('mic', settings.mic)}${switchBtn('mic', settings.mic, 'Microphone')}` }),
      row({ ic: settings.system ? 'volume-2' : 'volume-x', title: 'System sound', sub: 'Everything your PC plays', ctl: `${meter('system', settings.system)}${switchBtn('system', settings.system, 'System sound')}` }),
      row({ ic: settings.camera ? 'webcam' : 'camera-off', title: 'Camera', sub: esc(devices.cameras[settings.cameraDevice]), ctl: switchBtn('camera', settings.camera, 'Camera') }),
      settings.camera ? row({ title: 'Camera shape', ctl: seg('cameraShape', ['Circle', 'Square'], settings.cameraShape) }) : '',
    ].join(''),
  });

  const effects = card({
    ic: 'wand-sparkles', title: 'Effects', hint: 'Applied when you edit',
    body: [
      row({ ic: 'mouse-pointer-click', title: 'Show cursor', ctl: switchBtn('cursor', settings.cursor, 'Show cursor') }),
      row({ ic: 'circle-dot', title: 'Highlight clicks', sub: 'A soft ring where you click', ctl: switchBtn('clicks', settings.clicks, 'Highlight clicks') }),
      row({ ic: 'zoom-in', title: 'Auto zoom', sub: 'Zooms in on what you click', ctl: switchBtn('autoZoom', settings.autoZoom, 'Auto zoom') }),
    ].join(''),
  });

  return `
    <div class="page-head">
      <div class="page-title">
        <span class="chip-lg">${icon('video')}</span>
        <div><h1>Record</h1><p>Pick what to capture, check your sound, and start.</p></div>
      </div>
    </div>
    <div class="grid record">
      <div class="stack">
        ${modeTiles}
        ${card({
          ic: 'monitor', title: 'Preview', hint: esc(previewBadge()),
          body: `${livePreviewOrArt(camOn)}${busy ? '' : sourceList()}`,
        })}
      </div>
      <div class="stack">
        ${startCard()}
        ${inputs}
        ${busy ? '' : effects}
      </div>
    </div>`;
}
