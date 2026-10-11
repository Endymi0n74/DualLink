import { render, refreshAdapters, updateMonitorState, loadSettings, loadAppVersion } from './ui.js';
import { listen } from '@tauri-apps/api/event';

async function init() {
  await loadSettings();
  await loadAppVersion();
  render();
  await refreshAdapters();

  listen('monitoring-update', (event) => {
    updateMonitorState(event.payload);
  });
}

init();
