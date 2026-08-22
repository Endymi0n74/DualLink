import * as api from './api.js';

let adapters = [];
let currentMode = 'none';
let monitorData = null;
let failoverState = null;
let settingsData = null;
let currentView = 'main'; // 'main' | 'settings' | 'logs' | 'expert'
let logContent = '';
let logFiles = [];
let expertDetails = null;
let selectedLogDate = '';
let logAutoRefresh = null;
const MAX_HISTORY = 10;
let latencyHistory = [];

function getAdapterIcon(desc) {
  const d = desc.toLowerCase();
  if (d.includes('wi-fi') || d.includes('wifi') || d.includes('wireless')) return '📶';
  if (d.includes('ethernet') || d.includes('realtek') || d.includes('intel')) return '🔌';
  if (d.includes('bluetooth')) return '🔵';
  if (d.includes('wan') || d.includes('pppoe') || d.includes('freebox')) return '🌐';
  if (d.includes('usb') || d.includes('cellular') || d.includes('mobile')) return '📱';
  return '⚙️';
}

function latencyClass(ms) {
  if (ms == null) return '';
  if (ms < 50) return 'latency-good';
  if (ms < 100) return 'latency-medium';
  return 'latency-bad';
}

function getMonitorForAdapter(name) {
  if (!monitorData || !monitorData.adapters) return null;
  return monitorData.adapters.find(a => a.name === name);
}

function renderAdapterCard(adapter) {
  const icon = getAdapterIcon(adapter.description);
  const mon = getMonitorForAdapter(adapter.name);
  const isUp = adapter.status === 'Up';
  const connected = mon ? mon.reachable : false;
  const latency = mon ? mon.latency_ms : null;
  const latCls = latencyClass(latency);
  return '<div class="adapter-card ' + (isUp ? '' : 'disabled') + '">'
    + '<div class="adapter-icon">' + icon + '</div>'
    + '<div class="adapter-info">'
    + '<div class="adapter-name">' + adapter.name + '</div>'
    + '<div class="adapter-meta">'
    + '<span class="adapter-speed">⚡ ' + adapter.speed + '</span>'
    + (latency != null ? '<span class="adapter-latency ' + latCls + '">🎯 ' + latency + 'ms</span>' : '')
    + '<span class="adapter-connectivity ' + (connected ? 'connected' : 'disconnected') + '">'
    + (connected ? '✅ Connecté' : '❌ Déconnecté') + '</span>'
    + '</div></div>'
    + '<label class="toggle">'
    + '<input type="checkbox" ' + (isUp ? 'checked' : '') + ' data-adapter="' + adapter.name + '" ' + (isUp && adapters.length <= 1 ? 'disabled' : '') + '>'
    + '<span class="toggle-slider"></span></label>'
    + '</div>';
}

function renderStatus() {
  const n = adapters.filter(a => a.status === 'Up').length;
  const lat = monitorData ? monitorData.overall_latency_ms : null;
  const on = monitorData ? monitorData.internet_reachable : false;
  const lc = monitorData ? monitorData.last_check : 'N/A';
  return '<div class="status-item"><span>🟢</span><span class="status-value">' + (on ? 'En ligne' : 'Hors ligne') + '</span></div>'
    + '<div class="status-item"><span>⚡</span><span class="status-value">' + n + ' carte(s) active(s)</span></div>'
    + (lat != null ? '<div class="status-item"><span>🎯</span><span class="status-value ' + latencyClass(lat) + '">' + lat + 'ms</span></div>' : '')
    + '<div class="status-item"><span>🕐</span><span class="status-value">' + lc + '</span></div>';
}

export function render() {
  const app = document.getElementById('app');
  const on = monitorData ? monitorData.internet_reachable : false;
  const noAdapters = adapters.length === 0;
  const adapterCards = noAdapters ?
    '<div class="empty-state"><div class="empty-state-icon">📡</div>Aucune interface détectée<br>Cliquez sur Actualiser</div>' :
    adapters.map(renderAdapterCard).join('');

  app.innerHTML = '';
  const header = document.createElement('div');
  header.className = 'glass header';
  header.innerHTML = '<div class="header-left"><span class="header-logo">🌐</span><span class="header-title">DualLink</span></div>'
    + '<div class="header-nav">'
    + '<button class="nav-btn ' + (currentView === 'main' ? 'active' : '') + '" id="nav-main"><span>🏠</span> Accueil</button>'
    + '<button class="nav-btn ' + (currentView === 'settings' ? 'active' : '') + '" id="nav-settings"><span>⚙️</span> Settings</button>'
    + '<button class="nav-btn ' + (currentView === 'logs' ? 'active' : '') + '" id="nav-logs"><span>📋</span> Logs</button>'
    + '<button class="nav-btn ' + (currentView === 'expert' ? 'active' : '') + '" id="nav-expert"><span>🔬</span> Expert</button>'
    + '</div>'
    + '<div class="header-status"><span class="status-dot ' + (on ? 'online' : '') + '"></span><span>' + (on ? 'Connecté' : 'Déconnecté') + '</span></div>';
  app.appendChild(header);

  const mode = document.createElement('div');
  if (currentView === 'logs') {
    // ── Logs View ──
    app.appendChild(renderLogsView());
  } else if (currentView === 'settings') {
    // ── Settings View ──
    app.appendChild(renderSettingsView());
  } else if (currentView === 'expert') {
    // ── Expert View ──
    app.appendChild(renderExpertView());
  } else {
    // ── Main View ──
    mode.className = 'glass';
    const foEnabled = failoverState && failoverState.config && failoverState.config.enabled;
    const foActive = failoverState && failoverState.is_failed_over;
    mode.innerHTML = '<div class="section-title">Mode de mutualisation</div>'
      + '<div class="mode-selector">'
      + '<button class="mode-btn ' + (currentMode === 'none' ? 'active' : '') + '" data-mode="none"><span class="mode-btn-icon">🔀</span><div class="mode-btn-text"><span class="mode-btn-label">Individuel</span><span class="mode-btn-desc">Chaque carte gère son trafic</span></div></button>'
      + '<button class="mode-btn ' + (currentMode === 'load-balancing' ? 'active' : '') + '" data-mode="load-balancing"><span class="mode-btn-icon">⚖️</span><div class="mode-btn-text"><span class="mode-btn-label">Load Balancing</span><span class="mode-btn-desc">Répartir le trafic sur les deux</span></div></button>'
      + '<button class="mode-btn ' + (currentMode === 'failover' ? 'active' : '') + '" data-mode="failover"><span class="mode-btn-icon">🛡️</span><div class="mode-btn-text"><span class="mode-btn-label">Failover</span><span class="mode-btn-desc">Backup automatique si chute</span></div></button>'
      + '</div>'
      + (foEnabled ? '<div class="failover-banner ' + (foActive ? 'failover-active' : 'failover-standby') + '">'
        + '<span class="failover-banner-icon">' + (foActive ? '⚡' : '🛡️') + '</span>'
        + '<span class="failover-banner-text">'
        + (foActive
          ? '⚠️ FAILOVER ACTIF — Bascule sur ' + (failoverState.config.secondary_name || 'secondary')
          : '✅ Failover en standby — ' + (failoverState.config.primary_name || 'primary') + ' actif')
        + '</span>'
        + '<span class="failover-banner-count">Switches: ' + (failoverState.failover_count || 0) + '</span>'
        + '<button class="btn btn-sm btn-danger" id="btn-disable-failover">Désactiver</button>'
        + '</div>' : '');
    app.appendChild(mode);

    const cards = document.createElement('div');
    cards.className = 'glass';
    cards.innerHTML = '<div class="section-title">Interfaces réseau</div><div class="adapter-list">' + adapterCards + '</div>';
    app.appendChild(cards);

    const dash = document.createElement('div');
    dash.innerHTML = renderDashboard();
    app.appendChild(dash);

    const status = document.createElement('div');
    status.className = 'glass status-bar';
    status.innerHTML = '<div class="status-bar-left">' + renderStatus() + '</div>';
    app.appendChild(status);

    const actions = document.createElement('div');
    actions.className = 'actions-bar';
    actions.innerHTML = '<button class="btn" id="btn-refresh"><span>🔄</span> Actualiser</button>'
      + '<button class="btn btn-primary" id="btn-apply" ' + (adapters.length < 2 ? 'disabled' : '') + '><span>💾</span> Appliquer le mode</button>';
    app.appendChild(actions);
  }

  const toasts = document.createElement('div');
  toasts.className = 'toast-container';
  toasts.id = 'toasts';
  app.appendChild(toasts);

  bindEvents();
}

function renderDashboard() {
  const canvasId = 'latency-canvas';
  const hist = latencyHistory;
  const validMs = hist.filter(h => h.ms != null).map(h => h.ms);
  const current = validMs.length > 0 ? validMs[validMs.length - 1] : null;
  const avg = validMs.length > 0 ? Math.round(validMs.reduce((a, b) => a + b, 0) / validMs.length) : null;
  const min = validMs.length > 0 ? Math.min(...validMs) : null;
  const max = validMs.length > 0 ? Math.max(...validMs) : null;
  const uptime = hist.length > 0 ? Math.round(hist.filter(h => h.ok).length / hist.length * 100) : 0;

  return '<div class="glass dashboard-card">'
    + '<div class="section-title">📊 Monitoring temps réel</div>'
    + '<div class="dashboard-grid">'
    + '<div class="dashboard-canvas-wrap"><canvas id="' + canvasId + '" width="500" height="120"></canvas></div>'
    + '<div class="dashboard-stats">'
    + '<div class="dash-stat"><span class="dash-stat-label">Actuel</span><span class="dash-stat-value ' + latencyClass(current) + '">' + (current != null ? current + 'ms' : '—') + '</span></div>'
    + '<div class="dash-stat"><span class="dash-stat-label">Moyenne</span><span class="dash-stat-value ' + latencyClass(avg) + '">' + (avg != null ? avg + 'ms' : '—') + '</span></div>'
    + '<div class="dash-stat"><span class="dash-stat-label">Min</span><span class="dash-stat-value ' + latencyClass(min) + '">' + (min != null ? min + 'ms' : '—') + '</span></div>'
    + '<div class="dash-stat"><span class="dash-stat-label">Max</span><span class="dash-stat-value ' + latencyClass(max) + '">' + (max != null ? max + 'ms' : '—') + '</span></div>'
    + '<div class="dash-stat"><span class="dash-stat-label">Uptime</span><span class="dash-stat-value ' + (uptime >= 90 ? 'latency-good' : uptime >= 50 ? 'latency-medium' : 'latency-bad') + '">' + uptime + '%</span></div>'
    + '<div class="dash-stat"><span class="dash-stat-label">Mesures</span><span class="dash-stat-value">' + hist.length + '/' + MAX_HISTORY + '</span></div>'
    + '</div></div></div>';
}

function drawDashboardCanvas() {
  const canvas = document.getElementById('latency-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  const W = canvas.width;
  const H = canvas.height;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = W * dpr;
  canvas.height = H * dpr;
  canvas.style.width = W + 'px';
  canvas.style.height = H + 'px';
  ctx.scale(dpr, dpr);

  ctx.clearRect(0, 0, W, H);

  const hist = latencyHistory;
  const validMs = hist.filter(h => h.ms != null).map(h => h.ms);
  if (validMs.length < 2) {
    ctx.fillStyle = '#555570';
    ctx.font = '13px Segoe UI';
    ctx.textAlign = 'center';
    ctx.fillText('En attente de mesures...', W / 2, H / 2 + 4);
    return;
  }

  const maxVal = Math.max(...validMs) * 1.2 || 100;
  const minVal = 0;
  const range = maxVal - minVal || 1;
  const padX = 8;
  const padY = 10;
  const gW = W - padX * 2;
  const gH = H - padY * 2;

  // Grid lines
  ctx.strokeStyle = 'rgba(255,255,255,.06)';
  ctx.lineWidth = 1;
  for (let i = 0; i <= 4; i++) {
    const y = padY + (gH * i) / 4;
    ctx.beginPath();
    ctx.moveTo(padX, y);
    ctx.lineTo(W - padX, y);
    ctx.stroke();
  }

  // Map history to points (include null gaps)
  const points = [];
  for (let i = 0; i < hist.length; i++) {
    const x = padX + (gW * i) / (MAX_HISTORY - 1);
    if (hist[i].ms != null) {
      const y = padY + gH - ((hist[i].ms - minVal) / range) * gH;
      points.push({ x, y, ok: true });
    } else {
      points.push({ x, y: null, ok: hist[i].ok });
    }
  }

  // Gradient fill under line
  const okPoints = points.filter(p => p.y !== null);
  if (okPoints.length > 1) {
    const grad = ctx.createLinearGradient(0, 0, 0, H);
    grad.addColorStop(0, 'rgba(0,212,255,.25)');
    grad.addColorStop(1, 'rgba(0,212,255,.02)');
    ctx.beginPath();
    ctx.moveTo(okPoints[0].x, H - padY);
    for (const p of okPoints) ctx.lineTo(p.x, p.y);
    ctx.lineTo(okPoints[okPoints.length - 1].x, H - padY);
    ctx.closePath();
    ctx.fillStyle = grad;
    ctx.fill();
  }

  // Line
  ctx.strokeStyle = '#00d4ff';
  ctx.lineWidth = 2;
  ctx.lineJoin = 'round';
  ctx.beginPath();
  let started = false;
  for (const p of points) {
    if (p.y === null) { started = false; continue; }
    if (!started) { ctx.moveTo(p.x, p.y); started = true; }
    else ctx.lineTo(p.x, p.y);
  }
  ctx.stroke();

  // Dots
  for (const p of points) {
    if (p.y === null) continue;
    ctx.beginPath();
    ctx.arc(p.x, p.y, 3, 0, Math.PI * 2);
    ctx.fillStyle = '#00d4ff';
    ctx.fill();
    ctx.beginPath();
    ctx.arc(p.x, p.y, 1.5, 0, Math.PI * 2);
    ctx.fillStyle = '#fff';
    ctx.fill();
  }

  // Threshold lines
  const drawThreshold = (ms, color, label) => {
    const y = padY + gH - ((ms - minVal) / range) * gH;
    if (y < padY || y > padY + gH) return;
    ctx.strokeStyle = color;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(padX, y);
    ctx.lineTo(W - padX, y);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = color;
    ctx.font = '9px Segoe UI';
    ctx.textAlign = 'left';
    ctx.fillText(label + 'ms', padX + 2, y - 3);
  };
  drawThreshold(50, 'rgba(0,255,136,.5)', '50');
  drawThreshold(100, 'rgba(255,170,34,.5)', '100');
}

// ─── Logs ─────────────────────────────────────────────────────

async function loadLogs() {
  try {
    if (!selectedLogDate) {
      logContent = await api.getLogs();
    } else {
      logContent = await api.getLogByDate(selectedLogDate);
    }
    logFiles = await api.getLogFiles();
  } catch (err) {
    logContent = 'Erreur: ' + err;
  }
}

function startLogAutoRefresh() {
  clearLogAutoRefresh();
  logAutoRefresh = setInterval(async () => {
    if (currentView === 'logs') {
      await loadLogs();
      const el = document.getElementById('log-content');
      if (el) {
        el.textContent = logContent;
        el.scrollTop = el.scrollHeight;
      }
    }
  }, 3000);
}

function clearLogAutoRefresh() {
  if (logAutoRefresh) { clearInterval(logAutoRefresh); logAutoRefresh = null; }
}

function colorizeLogLine(line) {
  if (line.includes('ERROR') || line.includes('PANIC')) return '<span class="log-error">' + line + '</span>';
  if (line.includes('FAILOVER')) return '<span class="log-failover">' + line + '</span>';
  if (line.includes('CMD')) return '<span class="log-cmd">' + line + '</span>';
  if (line.includes('===')) return '<span class="log-separator">' + line + '</span>';
  if (line.includes('Monitor')) return '<span class="log-monitor">' + line + '</span>';
  return '<span class="log-normal">' + line + '</span>';
}

function renderLogsView() {
  const lines = logContent ? logContent.trim().split('\n') : [];
  const colored = lines.map(colorizeLogLine).join('\n');
  const fileOptions = logFiles.map(f => {
    const date = f.replace('.log', '');
    const sel = selectedLogDate === date ? ' selected' : '';
    return '<option value="' + date + '"' + sel + '>' + date + '</option>';
  }).join('');
  const today = new Date().toISOString().slice(0, 10);
  const container = document.createElement('div');
  container.className = 'glass log-card';
  container.innerHTML = '<div class="log-header">'
    + '<div class="section-title">📋 Journal d\'événements</div>'
    + '<div class="log-controls">'
    + '<select id="log-date-select" class="log-select">'
    + '<option value=""' + (!selectedLogDate ? ' selected' : '') + '>Aujourd\'hui (' + today + ')</option>'
    + fileOptions
    + '</select>'
    + '<button class="btn btn-sm" id="btn-log-refresh">🔄</button>'
    + '<button class="btn btn-sm" id="btn-log-auto">' + (logAutoRefresh ? '⏸ Pause' : '▶ Auto') + '</button>'
    + '</div></div>'
    + '<div class="log-legend">'
    + '<span class="log-separator">═══ Séparateur</span>'
    + '<span class="log-cmd">CMD Commande</span>'
    + '<span class="log-monitor">Monitor Événement</span>'
    + '<span class="log-failover">FAILOVER Bascule</span>'
    + '<span class="log-error">ERROR/PANIC Erreur</span>'
    + '</div>'
    + '<pre id="log-content" class="log-content">' + colored + '</pre>';
  return container;
}

function renderSettingsView() {
  const s = settingsData || { interval_secs: 5, ping_target: '8.8.8.8', adapter_refresh_secs: 30 };
  const container = document.createElement('div');
  container.className = 'glass settings-card';
  container.innerHTML = '<div class="section-title">⚙️ Configuration du monitoring</div>'
    + '<div class="settings-grid">'
    + '<div class="settings-group">'
    + '<label class="settings-label">Intervalle de ping</label>'
    + '<div class="settings-input-wrap">'
    + '<input type="number" id="setting-interval" class="settings-input" value="' + s.interval_secs + '" min="1" max="300" step="1">'
    + '<span class="settings-unit">secondes</span>'
    + '</div>'
    + '<span class="settings-hint">Fréquence du test de connectivité (défaut: 5s)</span>'
    + '</div>'
    + '<div class="settings-group">'
    + '<label class="settings-label">Cible de ping</label>'
    + '<div class="settings-input-wrap">'
    + '<input type="text" id="setting-target" class="settings-input settings-input-text" value="' + s.ping_target + '" placeholder="8.8.8.8">'
    + '</div>'
    + '<span class="settings-hint">IP ou hostname pour tester la connectivité (défaut: 8.8.8.8)</span>'
    + '</div>'
    + '<div class="settings-group">'
    + '<label class="settings-label">Refresh interfaces</label>'
    + '<div class="settings-input-wrap">'
    + '<input type="number" id="setting-refresh" class="settings-input" value="' + s.adapter_refresh_secs + '" min="5" max="600" step="5">'
    + '<span class="settings-unit">secondes</span>'
    + '</div>'
    + '<span class="settings-hint">Fréquence de rafraîchissement de la liste des cartes (défaut: 30s)</span>'
    + '</div>'
    + '</div>'
    + '<div class="settings-actions">'
    + '<button class="btn" id="btn-settings-defaults">🔄 Rétablir défauts</button>'
    + '<button class="btn btn-primary" id="btn-settings-save">💾 Sauvegarder</button>'
    + '</div>'
    + '<div class="settings-note">⚠️ Le changement d\'intervalle nécessite un redémarrage de l\'application.</div>';
  return container;
}

function bindEvents() {
  document.querySelectorAll('.mode-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      currentMode = btn.dataset.mode;
      render();
    });
  });

  // Nav buttons
  const navMain = document.getElementById('nav-main');
  if (navMain) navMain.addEventListener('click', () => { currentView = 'main'; clearLogAutoRefresh(); render(); });
  const navSettings = document.getElementById('nav-settings');
  if (navSettings) navSettings.addEventListener('click', () => { currentView = 'settings'; clearLogAutoRefresh(); render(); });
  const navLogs = document.getElementById('nav-logs');
  if (navLogs) navLogs.addEventListener('click', async () => { currentView = 'logs'; render(); await loadLogs(); startLogAutoRefresh(); });
  const navExpert = document.getElementById('nav-expert');
  if (navExpert) navExpert.addEventListener('click', async () => { currentView = 'expert'; clearLogAutoRefresh(); await loadExpertDetails(); render(); });


  // Settings save
  const btnSave = document.getElementById('btn-settings-save');
  if (btnSave) btnSave.addEventListener('click', async () => {
    const interval = parseInt(document.getElementById('setting-interval').value) || 5;
    const target = (document.getElementById('setting-target').value || '8.8.8.8').trim();
    const refresh = parseInt(document.getElementById('setting-refresh').value) || 30;
    try {
      settingsData = await api.saveSettings(interval, target, refresh);
      showToast('✅ Settings sauvegardés (redémarrage pour interval)', 'success');
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });

  // Settings defaults
  const btnDefaults = document.getElementById('btn-settings-defaults');
  if (btnDefaults) btnDefaults.addEventListener('click', async () => {
    try {
      settingsData = await api.saveSettings(5, '8.8.8.8', 30);
      showToast('🔄 Défauts restaurés', 'info');
      render();
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });

  // Log controls
  const logDateSelect = document.getElementById('log-date-select');
  if (logDateSelect) logDateSelect.addEventListener('change', async (e) => {
    selectedLogDate = e.target.value;
    await loadLogs();
    render();
    startLogAutoRefresh();
  });
  const btnLogRefresh = document.getElementById('btn-log-refresh');
  if (btnLogRefresh) btnLogRefresh.addEventListener('click', async () => {
    await loadLogs();
    const el = document.getElementById('log-content');
    if (el) { el.innerHTML = logContent.trim().split('\n').map(colorizeLogLine).join('\n'); el.scrollTop = el.scrollHeight; }
  });
  const btnLogAuto = document.getElementById('btn-log-auto');
  if (btnLogAuto) btnLogAuto.addEventListener('click', () => {
    if (logAutoRefresh) { clearLogAutoRefresh(); } else { startLogAutoRefresh(); }
    render();
  });

  document.querySelectorAll('.toggle input').forEach(input => {
    input.addEventListener('change', async (e) => {
      const name = e.target.dataset.adapter;
      const checked = e.target.checked;
      e.target.disabled = true;
      try {
        if (checked) {
          await api.enableAdapter(name);
          showToast('✅ ' + name + ' activé', 'success');
        } else {
          await api.disableAdapter(name);
          showToast('⛔ ' + name + ' désactivé', 'success');
        }
        await refreshAdapters();
      } catch (err) {
        showToast('❌ Erreur: ' + err, 'error');
        e.target.checked = !checked;
      } finally {
        e.target.disabled = false;
      }
    });
  });

  const btnRefresh = document.getElementById('btn-refresh');
  if (btnRefresh) btnRefresh.addEventListener('click', async () => {
    await refreshAdapters();
    showToast('🔄 Listes actualisées', 'info');
  });

  const btnDisableFo = document.getElementById('btn-disable-failover');
  if (btnDisableFo) btnDisableFo.addEventListener('click', async () => {
    try {
      failoverState = await api.disableAutoFailover();
      showToast('🛡️ Auto Failover désactivé', 'info');
      render();
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });

  const btnApply = document.getElementById('btn-apply');
  if (btnApply) btnApply.addEventListener('click', async () => {
    const active = adapters.filter(a => a.status === 'Up');
    if (active.length < 2) {
      showToast('⚠️ Il faut au moins 2 cartes actives', 'error');
      return;
    }
    try {
      const p = active[0], s = active[1];
      if (currentMode === 'load-balancing') {
        await api.configureLoadBalancing(p.interface_index, s.interface_index);
        showToast('⚖️ Load Balancing configuré', 'success');
      } else if (currentMode === 'failover') {
        const result = await api.enableAutoFailover(
          p.interface_index, s.interface_index,
          p.name || p.description, s.name || s.description
        );
        failoverState = result;
        showToast('🛡️ Auto Failover activé: ' + (p.name || p.description) + ' → ' + (s.name || s.description), 'success');
      } else {
        showToast('ℹ️ Sélectionnez un mode dabord', 'info');
      }
      render();
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });

  // Expert refresh
  const btnExpertRefresh = document.getElementById('btn-expert-refresh');
  if (btnExpertRefresh) btnExpertRefresh.addEventListener('click', async () => {
    await loadExpertDetails();
    render();
    showToast('🔄 Détails actualisés', 'info');
  });
}

export async function loadSettings() {
  try {
    settingsData = await api.getSettings();
  } catch (err) {
    // Use defaults
  }
}

export async function refreshAdapters() {
  try {
    adapters = await api.listAdapters();
    render();
  } catch (err) {
    showToast('❌ Impossible de lister les cartes: ' + err, 'error');
  }
}

// ─── Expert Mode ───────────────────────────────────────────

async function loadExpertDetails() {
  try {
    expertDetails = await api.getAllAdapterDetails();
  } catch (err) {
    expertDetails = [];
    showToast('\u274c Erreur: ' + err, 'error');
  }
}

function renderExpertView() {
  const container = document.createElement('div');
  container.className = 'glass expert-card';
  if (!expertDetails || expertDetails.length === 0) {
    container.innerHTML = '<div class="section-title">\ud83d\udd2c Mode Expert</div>'
      + '<div class="empty-state"><div class="empty-state-icon">\ud83d\udd0d</div>Chargement des donn\u00e9es r\u00e9seau...</div>';
    return container;
  }
  const cards = expertDetails.map(d => {
    const isUp = d.status === 'Up';
    const icon = getAdapterIcon(d.description);
    return '<div class="expert-adapter">'
      + '<div class="expert-adapter-header">'
      + '<span class="expert-icon">' + icon + '</span>'
      + '<span class="expert-name">' + d.name + '</span>'
      + '<span class="expert-status ' + (isUp ? 'up' : 'down') + '">' + (isUp ? '\u25cf Actif' : '\u25cb Inactif') + '</span>'
      + '</div>'
      + '<div class="expert-grid">'
      + '<div class="expert-field"><span class="expert-label">Description</span><span class="expert-value">' + d.description + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">Index</span><span class="expert-value">' + d.interface_index + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">MAC</span><span class="expert-value">' + d.mac_address + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">Vitesse</span><span class="expert-value">' + d.speed + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">IP</span><span class="expert-value expert-ip">' + d.ip_address + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">Masque</span><span class="expert-value">' + d.subnet_mask + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">Passerelle</span><span class="expert-value expert-gw">' + d.default_gateway + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">DNS</span><span class="expert-value">' + d.dns_servers + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">DHCP</span><span class="expert-value">' + (d.dhcp_enabled ? '\u2705 Oui' : '\u274c Non') + '</span></div>'
      + '<div class="expert-field"><span class="expert-label">M\u00e9trique</span><span class="expert-value">' + (d.routing_metric != null ? d.routing_metric : 'N/A') + '</span></div>'
      + '</div>'
      + '</div>';
  }).join('');
  container.innerHTML = '<div class="section-title">\ud83d\udd2c Mode Expert \u2014 D\u00e9tails r\u00e9seau</div>'
    + '<div class="expert-adapter-list">' + cards + '</div>'
    + '<div class="expert-actions">'
    + '<button class="btn" id="btn-expert-refresh"><span>\ud83d\udd04</span> Actualiser</button>'
    + '</div>';
  return container;
}

export function updateMonitorState(state) {
  monitorData = state;
  if (state.failover) failoverState = state.failover;
  // Track latency history (last 10 measurements)
  if (state.overall_latency_ms != null) {
    latencyHistory.push({ ms: state.overall_latency_ms, time: state.last_check, ok: true });
  } else {
    latencyHistory.push({ ms: null, time: state.last_check, ok: state.internet_reachable });
  }
  if (latencyHistory.length > MAX_HISTORY) latencyHistory.shift();
  render();
  drawDashboardCanvas();
}

function showToast(message, type) {
  type = type || 'info';
  const container = document.getElementById('toasts');
  if (!container) return;
  const icons = { success: '✅', error: '❌', info: 'ℹ️' };
  const toast = document.createElement('div');
  toast.className = 'toast ' + type;
  toast.innerHTML = '<span class="toast-icon">' + (icons[type] || 'ℹ️') + '</span><span>' + message + '</span>';
  container.appendChild(toast);
  setTimeout(() => {
    toast.classList.add('exiting');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}
