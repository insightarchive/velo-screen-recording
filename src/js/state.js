// Single source of truth for settings, the live recording session and the library.
// Settings and the library persist to localStorage; the session lives in memory.

const SETTINGS_KEY = 'velo-settings-v2';
const LIBRARY_KEY = 'velo-library-v2';

const defaults = {
  theme: 'light',
  mode: 'display',
  display: 0,
  window: 0,
  aspect: 'Free',
  mic: true,
  micDevice: 0,
  system: true,
  camera: false,
  cameraDevice: 0,
  cameraShape: 'Circle',
  cursor: true,
  clicks: true,
  autoZoom: true,
  recordMode: 'Studio',
  countdown: '3',
  quality: '1080p',
  fps: '60',
  bitrate: 'Balanced',
  encoder: 'Hardware (GPU)',
  hideWindow: true,
  border: true,
  noise: true,
  micGain: 80,
  startup: false,
  tray: true,
  onClose: 'Hide to tray',
  updates: true,
  openAfter: true,
  copyAfter: false,
  folder: 'Videos\\Velo',
  pattern: 'Velo {date} {time}',
};

function read(key, fallback) {
  try { return JSON.parse(localStorage.getItem(key)) ?? fallback; } catch (_) { return fallback; }
}
function write(key, value) {
  try { localStorage.setItem(key, JSON.stringify(value)); } catch (_) { /* storage can be unavailable */ }
}

export const settings = { ...defaults, ...read(SETTINGS_KEY, {}) };
export function set(key, value) { settings[key] = value; write(SETTINGS_KEY, settings); }
export function resetSettings() { Object.assign(settings, defaults); write(SETTINGS_KEY, settings); }
export function toggle(key) { set(key, !settings[key]); }

// Replaced by real devices from the Rust side (list_sources) when running in Tauri.
export function setDevices(next) {
  if (next) Object.assign(devices, next);
}

export const devices = {
  displays: [
    { name: 'Display 1', detail: '1920 x 1080 (main)' },
    { name: 'Display 2', detail: '2560 x 1440' },
  ],
  windows: [
    { name: 'Google Chrome', detail: 'New tab' },
    { name: 'Visual Studio Code', detail: 'app.js' },
    { name: 'File Explorer', detail: 'Downloads' },
  ],
  mics: ['Default microphone', 'Headset microphone', 'USB microphone'],
  cameras: ['Integrated camera', 'USB webcam'],
  speakers: ['Default speakers', 'Headphones'],
};

export const session = { recording: false, paused: false, saving: false, seconds: 0, counting: 0 };

/** Latest live preview frame from the Rust side (empty until the first one arrives). */
export const preview = { src: '' };

/** One-shot snapshot per source, keyed `${mode}:${index}`, for the picker list in Record. */
export const thumbs = {};

/** Thumbnail per recorded video, keyed by its file path, for Library and the Editor. */
export const videoThumbs = {};

export const library = read(LIBRARY_KEY, []);
export function addRecording(rec) { library.unshift(rec); write(LIBRARY_KEY, library); }
export function removeRecording(id) {
  const i = library.findIndex((r) => r.id === id);
  if (i >= 0) library.splice(i, 1);
  write(LIBRARY_KEY, library);
}

export const ui = { settingsTab: 'General', search: '', sort: 'Newest', editorId: null };

export const editor = {
  bg: 0, pad: 64, radius: 16, shadow: true,
  trim: [0, 100], zooms: [{ start: 22, end: 38, scale: 2 }, { start: 60, end: 72, scale: 1.6 }],
  cursorSize: 100, ring: true, smooth: 'Medium', format: 'MP4', res: '1080p',
  // The loaded .velo.json project for the recording currently open (null until `load_project` returns).
  project: null, projectFor: null,
};

export const BACKGROUNDS = [
  'linear-gradient(135deg, #6366f1, #ec4899)',
  'linear-gradient(135deg, #0ea5e9, #6366f1)',
  'linear-gradient(135deg, #f59e0b, #ef4444)',
  'linear-gradient(135deg, #10b981, #0ea5e9)',
  'linear-gradient(135deg, #1e1b34, #3b2a73)',
  'linear-gradient(135deg, #e9e5f7, #d6cff0)',
  'linear-gradient(135deg, #111827, #374151)',
  'linear-gradient(135deg, #fde68a, #fca5a5)',
];

const listeners = new Set();
export const subscribe = (fn) => listeners.add(fn);
export const emit = (kind, payload) => listeners.forEach((fn) => fn(kind, payload));
