import { invoke, inTauri, listen, broadcast } from './bridge.js';
import { settings, session, addRecording, emit } from './state.js';

let timer = null;
let countdownTimer = null;

// The countdown runs as a full-screen overlay (its own window) so it's visible over whatever
// you're about to record, no matter which window started it.
function showCountdown(n) {
  if (!inTauri) return;
  invoke('show_countdown');
  broadcast('countdown-tick', { n });
}
function tickCountdown(n) {
  broadcast('countdown-tick', { n });
}
function cancelCountdownOverlay() {
  if (inTauri) invoke('cancel_countdown');
}

const stamp = () => {
  const d = new Date();
  const p = (n) => String(n).padStart(2, '0');
  return { date: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`, time: `${p(d.getHours())}-${p(d.getMinutes())}-${p(d.getSeconds())}` };
};

/** What the Rust engine needs to know, taken from the current settings. */
function config() {
  return {
    mode: settings.mode,
    display: settings.display,
    window: settings.window,
    system: settings.system,
    mic: settings.mic,
    micDevice: settings.micDevice,
    micGain: settings.micGain,
    camera: settings.camera,
    cursor: settings.cursor,
    fps: Number(settings.fps),
    bitrate: settings.bitrate,
    studio: settings.recordMode === 'Studio',
    hideWindow: settings.hideWindow,
    folder: settings.folder,
    pattern: settings.pattern,
  };
}

async function begin() {
  try {
    await invoke('start_recording', { config: config() });
  } catch (e) {
    emit('error', String(e));
    return;
  }
  // In Tauri the backend reports state and time through events; the browser preview simulates them.
  if (!inTauri) {
    Object.assign(session, { recording: true, paused: false, seconds: 0, counting: 0 });
    timer = setInterval(() => {
      if (!session.paused) { session.seconds += 1; emit('tick'); }
    }, 1000);
    emit('state');
  }
}

export async function start() {
  if (session.recording || session.counting || session.saving) return;
  const n = Number(settings.countdown) || 0;
  if (inTauri) {
    try {
      await invoke('start_countdown', { config: config(), seconds: n });
    } catch (e) {
      emit('error', String(e));
    }
    return;
  }
  if (!n) { begin(); return; }
  session.counting = n;
  emit('state');
  showCountdown(n);
  countdownTimer = setInterval(() => {
    session.counting -= 1;
    if (session.counting <= 0) {
      clearInterval(countdownTimer);
      session.counting = 0;
      begin();
    } else {
      emit('state');
      tickCountdown(session.counting);
    }
  }, 1000);
}

export function cancelCountdown() {
  if (inTauri) {
    invoke('cancel_countdown');
  } else {
    clearInterval(countdownTimer);
    session.counting = 0;
    cancelCountdownOverlay();
    emit('state');
  }
}

export async function togglePause() {
  const pausing = !session.paused;
  await invoke(pausing ? 'pause_recording' : 'resume_recording');
  if (!inTauri) { session.paused = pausing; emit('state'); }
}

export async function stop() {
  if (inTauri) {
    try { await invoke('stop_recording'); } catch (e) { emit('error', String(e)); }
    return; // the 'stopped' event updates the session and library
  }
  clearInterval(timer);
  const { date, time } = stamp();
  const seconds = session.seconds;
  finished({
    path: '',
    seconds,
    sizeMb: Math.max(1, seconds * 0.9),
    name: settings.pattern.replace('{date}', date).replace('{time}', time),
  });
}

function finished(done) {
  addRecording({
    id: String(Date.now()),
    name: done.name ?? (done.path.split(/[\\/]/).pop() || 'Recording').replace(/\.mp4$/i, ''),
    path: done.path,
    projectPath: done.projectPath ?? null,
    seconds: done.seconds,
    size: done.sizeMb,
    mode: settings.mode,
    createdAt: Date.now(),
  });
  Object.assign(session, { recording: false, paused: false, saving: false, seconds: 0 });
  emit('state');
}

/** Wire the backend's events into the session. Call once at startup. */
export async function connectBackend() {
  await listen('countdown-tick', ({ n }) => {
    session.counting = n;
    emit('state');
  });
  await listen('rec-state', ({ state, data }) => {
    if (state === 'recording') {
      Object.assign(session, { recording: true, paused: false, saving: false, counting: 0 });
    } else if (state === 'paused') {
      session.paused = true;
    } else if (state === 'saving') {
      Object.assign(session, { recording: false, saving: true });
    } else if (state === 'stopped') {
      finished(data);
      return;
    } else if (state === 'discarded') {
      Object.assign(session, { recording: false, paused: false, saving: false, seconds: 0 });
    } else if (state === 'failed') {
      Object.assign(session, { recording: false, paused: false, saving: false, seconds: 0 });
      emit('error', data?.message ?? 'The recording failed.');
    }
    emit('state');
  });
  await listen('rec-tick', ({ ms, paused }) => {
    session.seconds = Math.floor(ms / 1000);
    session.paused = paused;
    emit('tick');
  });
  await listen('hotkey-start', () => start());
}
