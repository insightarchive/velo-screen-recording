// Thin wrapper over the Rust side. In a plain browser (design preview, tests) it falls back to
// a mock so the UI can be exercised without Tauri.

const tauri = window.__TAURI__;

export const inTauri = Boolean(tauri?.core?.invoke);

const mock = {
  list_sources: async () => null,
  capture_thumbnail: async () => null,
  start_preview: async () => {},
  stop_preview: async () => {},
  start_recording: async () => {},
  start_countdown: async () => {},
  stop_recording: async () => ({ path: '', seconds: 0, sizeMb: 0 }),
  discard_recording: async () => {},
  show_main_window: async () => {},
  pause_recording: async () => {},
  resume_recording: async () => {},
  reveal_in_folder: async () => {},
  set_close_to_tray: async () => {},
  video_thumbnail: async () => null,
  export_video: async () => ({ path: '', projectPath: null, seconds: 0, sizeMb: 0, screen: { w: 0, h: 0 } }),
  show_countdown: async () => {},
  close_countdown: async () => {},
  cancel_countdown: async () => {},
  resize_window: async () => {},
};

export async function invoke(cmd, args) {
  if (inTauri) return tauri.core.invoke(cmd, args);
  return mock[cmd]?.(args);
}

/** Subscribe to a backend event. Returns a promise of an unsubscribe function. */
export async function listen(event, handler) {
  if (!inTauri) return () => {};
  return tauri.event.listen(event, (e) => handler(e.payload));
}

/** Broadcasts an event to every window of this app (not just the backend). */
export function broadcast(event, payload) {
  if (!inTauri) return;
  tauri.event.emit(event, payload);
}

export function windowControl(action) {
  if (!inTauri) return;
  tauri.window.getCurrentWindow()[action]();
}

/** A local file path as a URL the webview is allowed to load (video src, etc). */
export function fileSrc(path) {
  if (!inTauri || !path) return '';
  return tauri.core.convertFileSrc(path);
}

/** Resizes the current window (logical pixels). Used by the launcher to hug its open popover. */
export async function resizeWindow(width, height) {
  if (!inTauri) return;
  try {
    const win = tauri.window.getCurrentWindow();
    await invoke('resize_window', { label: win.label, width, height });
  } catch (_) {
    try {
      const Size = tauri.dpi?.LogicalSize || tauri.window?.LogicalSize;
      if (Size) {
        await tauri.window.getCurrentWindow().setSize(new Size(width, height));
      } else {
        await tauri.window.getCurrentWindow().setSize({ type: 'Logical', width, height });
      }
    } catch (e) {
      console.error('resizeWindow error:', e);
    }
  }
}
