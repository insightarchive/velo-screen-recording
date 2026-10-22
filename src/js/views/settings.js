import { icon, card, row, seg, select, switchBtn, range, esc } from '../ui.js';
import { settings, devices, ui } from '../state.js';

const TABS = [
  ['General', 'sliders-horizontal'],
  ['Recording', 'video'],
  ['Audio', 'mic'],
  ['Hotkeys', 'keyboard'],
  ['Output', 'folder-open'],
];

const sw = (key, label) => switchBtn(key, settings[key], label);

const kbd = (combo) => combo.split('+').map((k) => `<span class="kbd">${k}</span>`).join(' + ');

const PANELS = {
  General: () => card({ ic: 'sliders-horizontal', title: 'Starting and closing', extra: `<button class="btn sm" data-act="reset">${icon('rotate-ccw')}Reset to default</button>`, body: [
    row({ title: 'Start with Windows', sub: 'So recording is one shortcut away.', ctl: sw('startup', 'Start with Windows') }),
    row({ title: 'Closing the window', sub: 'Keep running in the tray so hotkeys keep working.', ctl: seg('onClose', ['Hide to tray', 'Quit'], settings.onClose) }),
    row({ title: 'Theme', ctl: seg('theme', ['Light', 'Dark'], settings.theme === 'dark' ? 'Dark' : 'Light') }),
    row({ title: 'Check for updates', sub: 'Quietly, in the background.', ctl: sw('updates', 'Check for updates') }),
  ].join('') }),
  Recording: () => card({ ic: 'video', title: 'Quality and behaviour', body: [
    row({ title: 'Resolution', sub: 'Output size of the video.', ctl: seg('quality', ['720p', '1080p', '1440p', '4K'], settings.quality) }),
    row({ title: 'Frame rate', sub: '60 is smoother for motion and cursor.', ctl: seg('fps', ['30', '60'], settings.fps) }),
    row({ title: 'Bitrate', sub: 'Higher means sharper and larger files.', ctl: seg('bitrate', ['Low', 'Balanced', 'High'], settings.bitrate) }),
    row({ title: 'Encoder', sub: 'GPU encoding keeps the PC responsive.', ctl: select('encoder', ['Hardware (GPU)', 'Software (CPU)'], ['Hardware (GPU)', 'Software (CPU)'].indexOf(settings.encoder), 'Encoder') }),
    row({ title: 'Hide this window while recording', ctl: sw('hideWindow', 'Hide window') }),
    row({ title: 'Show a border around the recorded area', ctl: sw('border', 'Show border') }),
  ].join('') }),
  Audio: () => card({ ic: 'mic', title: 'Sound', body: [
    row({ title: 'Microphone', ctl: select('micDevice', devices.mics, settings.micDevice, 'Microphone') }),
    row({ title: 'Microphone volume', ctl: `<div style="width:160px">${range('micGain', 0, 100, settings.micGain, 'Microphone volume')}</div><span class="muted" data-out="micGain">${settings.micGain}%</span>` }),
    row({ title: 'Reduce background noise', sub: 'Cleans up fans and keyboard clicks.', ctl: sw('noise', 'Reduce noise') }),
    row({ title: 'System sound from', ctl: select('speaker', devices.speakers, 0, 'Speakers') }),
  ].join('') }),
  Hotkeys: () => card({ ic: 'keyboard', title: 'Global shortcuts', hint: 'Work from any app', body: [
    ['Start or stop recording', 'Alt+Shift+R'],
    ['Pause or resume', 'Alt+Shift+P'],
    ['Record a region', 'Alt+Shift+A'],
    ['Open the window', 'Ctrl+Shift+V'],
  ].map(([t, k]) => row({ title: t, ctl: kbd(k) })).join('') }),
  Output: () => card({ ic: 'folder-open', title: 'Saving', body: [
    row({ title: 'Save folder', sub: 'Where finished videos go.', ctl: `<input class="field" readonly value="${esc(settings.folder)}" aria-label="Save folder" /><button class="btn sm" data-act="browse">Browse</button>` }),
    row({ title: 'File name', sub: 'Use {date} and {time}.', ctl: `<input class="field" value="${esc(settings.pattern)}" aria-label="File name" data-text="pattern" />` }),
    row({ title: 'Open the folder after saving', ctl: sw('openAfter', 'Open folder') }),
    row({ title: 'Copy the file to the clipboard', ctl: sw('copyAfter', 'Copy file') }),
  ].join('') }),
};

export function view() {
  return `
    <div class="page-head">
      <div class="page-title">
        <span class="chip-lg">${icon('settings')}</span>
        <div><h1>Settings</h1><p>How Velo records, saves, and responds to your shortcuts.</p></div>
      </div>
    </div>
    <div class="tabs" role="tablist">${TABS.map(([name, ic]) =>
      `<button class="tab" role="tab" aria-selected="${ui.settingsTab === name}" data-tab="${name}">${icon(ic)}${name}</button>`).join('')}</div>
    ${PANELS[ui.settingsTab]()}`;
}
