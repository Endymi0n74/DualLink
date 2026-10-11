import { render, refreshAdapters, updateMonitorState, loadSettings, loadAppVersion, showTrayHint } from './ui.js';
import { listen } from '@tauri-apps/api/event';

async function init() {
  await loadSettings();
  await loadAppVersion();
  render();
  await refreshAdapters();

  listen('monitoring-update', (event) => {
    updateMonitorState(event.payload);
  });

  // Le bouton X cache la fenêtre au lieu de quitter: sans info, l'utilisateur
  // cherche l'application dans la barre des tâches et la croit plantée.
  listen('hide-to-tray-notice', () => {
    showTrayHint();
  });
}

init();
