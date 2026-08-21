import { render, refreshAdapters, updateMonitorState, loadSettings } from './ui.js';
import { listen } from '@tauri-apps/api/event';

async function init() {
  await loadSettings();
  render();
  await refreshAdapters();

  listen('monitoring-update', (event) => {
    updateMonitorState(event.payload);
  });
}

init();
