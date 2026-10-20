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
// Taille du log du jour déjà affichée (octets). Sert au backend pour ne
// renvoyer le contenu que lorsqu'une ligne a été ajoutée.
let logLength = 0;
let logRefreshBusy = false;
// Une seule relecture de la liste des cartes à la fois: le monitor signale un
// changement matériel, le rendu ne doit pas être relancé à chaque tick.
let adapterRefreshBusy = false;
// Signature du bandeau failover: évite de reconstruire le panneau à chaque
// tick quand rien n'a changé.
let lastFailoverSignature = '';
const MAX_HISTORY = 10;
const LOG_POLL_MS = 10000;
const CANVAS_W = 500;
const CANVAS_H = 120;
// Adaptateurs qui ne portent pas une connexion internet réelle: les choisir
// comme primaire/secondaire appliquerait des métriques à un commutateur
// virtuel (Hyper-V, VMware, VPN).
const VIRTUAL_ADAPTER_MARKERS = [
  'virtual', 'vmware', 'hyper-v', 'vethernet', 'loopback',
  'openvpn', 'tap-windows', 'wireguard', 'bluetooth'
];
let latencyHistory = [];
let appVersion = '';
// Élévation du processus: statique par session. `true` par défaut pour ne
// jamais afficher la bannière par erreur si la commande échoue.
let isElevated = true;

// Marque de l'application: deux anneaux entrelacés (deux liens), le même
// dessin que src-tauri/icons. Version 1.1.0, même dégradé cyan -> violet.
const LOGO_SVG = '<svg class="header-logo" viewBox="0 0 48 48" fill="none" aria-hidden="true">'
  + '<defs><linearGradient id="dl-mark" x1="0" y1="0" x2="1" y2="1">'
  + '<stop offset="0%" stop-color="#00d4ff"/><stop offset="100%" stop-color="#aa66ff"/>'
  + '</linearGradient></defs>'
  + '<g stroke="url(#dl-mark)" stroke-width="4.2">'
  + '<circle cx="17" cy="26" r="10.4"/>'
  + '<circle cx="31" cy="22" r="10.4"/>'
  + '</g></svg>';

// Charge la version affichée dans l'en-tête. Une commande absente ou en
// erreur ne doit pas empêcher l'application de s'afficher.
export async function loadAppVersion() {
  try {
    appVersion = 'v' + await api.getAppVersion();
  } catch (e) {
    appVersion = '';
  }
}

// Charge l'état d'élévation, qui pilote la bannière d'avertissement.
// Sans droits admin, chaque action réseau échoue: l'utilisateur doit le
// savoir AVANT de cliquer, pas au travers d'une erreur PowerShell.
export async function loadElevationStatus() {
  try {
    isElevated = await api.getElevationStatus();
  } catch (e) {
    isElevated = true;
  }
}

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

function isModeEligible(adapter) {
  const haystack = ((adapter.name || '') + ' ' + (adapter.description || '')).toLowerCase();
  return !VIRTUAL_ADAPTER_MARKERS.some(marker => haystack.includes(marker));
}

// Connectivité affichée. `internet_known` distingue « pas encore mesuré » de
// « hors ligne »: au démarrage, afficher « Déconnecté » était faux.
function connectivity() {
  if (!monitorData || !monitorData.internet_known) {
    return { known: false, online: false, label: 'Vérification…' };
  }
  const online = !!monitorData.internet_reachable;
  return { known: true, online, label: online ? 'Connecté' : 'Déconnecté' };
}

// État d'un adaptateur. Une sonde absente ou dégradée ne prouve pas qu'il est
// déconnecté: on affiche « État inconnu » au lieu de faire clignoter le badge.
function adapterConnectivity(adapter) {
  if (adapter.status === 'Disabled') return { cls: 'inactive', label: 'Désactivé' };
  if (adapter.status !== 'Up') return { cls: 'inactive', label: 'Inactif' };

  const mon = getMonitorForAdapter(adapter.name);
  if (!mon) return { cls: 'unknown', label: 'Vérification…' };
  if (mon.status_known === false) return { cls: 'unknown', label: 'État inconnu' };
  if (mon.reachable) return { cls: 'connected', label: 'Connecté' };
  return { cls: 'disconnected', label: 'Déconnecté' };
}

function renderAdapterCard(adapter, index) {
  const icon = getAdapterIcon(adapter.description);
  const mon = getMonitorForAdapter(adapter.name);
  const isUp = adapter.status === 'Up';
  const conn = adapterConnectivity(adapter);
  const latency = mon && mon.status_known !== false ? mon.latency_ms : null;
  const latencyHtml = latency != null
    ? '<span class="adapter-latency ' + latencyClass(latency) + '">🎯 ' + latency + 'ms</span>'
    : '<span class="adapter-latency" hidden></span>';
  return '<div class="adapter-card ' + (isUp ? '' : 'disabled') + '" data-card-index="' + index + '">'
    + '<div class="adapter-icon">' + icon + '</div>'
    + '<div class="adapter-info">'
    + '<div class="adapter-name">' + adapter.name + '</div>'
    + '<div class="adapter-meta">'
    + '<span class="adapter-speed">⚡ ' + adapter.speed + '</span>'
    + latencyHtml
    + '<span class="adapter-connectivity ' + conn.cls + '">' + conn.label + '</span>'
    + '</div></div>'
    + '<label class="toggle">'
    + '<input type="checkbox" ' + (isUp ? 'checked' : '') + ' data-adapter="' + adapter.name + '" ' + (isUp && adapters.length <= 1 ? 'disabled' : '') + '>'
    + '<span class="toggle-slider"></span></label>'
    + '</div>';
}

function renderStatus() {
  const n = adapters.filter(a => a.status === 'Up').length;
  const lat = monitorData ? monitorData.overall_latency_ms : null;
  const conn = connectivity();
  const lc = monitorData ? monitorData.last_check : 'N/A';
  const icon = conn.known ? (conn.online ? '🟢' : '🔴') : '⏳';
  return '<div class="status-item"><span>' + icon + '</span><span class="status-value">' + conn.label + '</span></div>'
    + '<div class="status-item"><span>⚡</span><span class="status-value">' + n + ' carte(s) active(s)</span></div>'
    + (lat != null ? '<div class="status-item"><span>🎯</span><span class="status-value ' + latencyClass(lat) + '">' + lat + 'ms</span></div>' : '')
    + '<div class="status-item"><span>🕐</span><span class="status-value">' + lc + '</span></div>';
}

function failoverSignature() {
  if (!failoverState || !failoverState.config) return 'none';
  const c = failoverState.config;
  return [c.enabled, failoverState.is_failed_over, failoverState.failover_count,
    c.primary_name, c.secondary_name].join('|');
}

function renderFailoverBanner() {
  if (!failoverState || !failoverState.config || !failoverState.config.enabled) return '';
  const active = failoverState.is_failed_over;
  return '<div class="failover-banner ' + (active ? 'failover-active' : 'failover-standby') + '">'
    + '<span class="failover-banner-icon">' + (active ? '⚡' : '🛡️') + '</span>'
    + '<span class="failover-banner-text">'
    + (active
      ? '⚠️ FAILOVER ACTIF — Bascule sur ' + (failoverState.config.secondary_name || 'secondary')
      : '✅ Failover en standby — ' + (failoverState.config.primary_name || 'primary') + ' actif')
    + '</span>'
    + '<span class="failover-banner-count">Switches: ' + (failoverState.failover_count || 0) + '</span>'
    + '<button class="btn btn-sm btn-danger" id="btn-disable-failover">Désactiver</button>'
    + '</div>';
}

export function render() {
  const app = document.getElementById('app');
  const conn = connectivity();
  const noAdapters = adapters.length === 0;
  const adapterCards = noAdapters ?
    '<div class="empty-state"><div class="empty-state-icon">📡</div>Aucune interface détectée<br>Cliquez sur Actualiser</div>' :
    adapters.map((adapter, index) => renderAdapterCard(adapter, index)).join('');

  app.innerHTML = '';
  const header = document.createElement('div');
  header.className = 'glass header';
  header.innerHTML = '<div class="header-left">' + LOGO_SVG
    + '<span class="header-title">DualLink</span>'
    + '<span class="header-version">' + (appVersion || '') + '</span></div>'
    + '<div class="header-nav">'
    + '<button class="nav-btn ' + (currentView === 'main' ? 'active' : '') + '" id="nav-main"><span>🏠</span> Accueil</button>'
    + '<button class="nav-btn ' + (currentView === 'settings' ? 'active' : '') + '" id="nav-settings"><span>⚙️</span> Settings</button>'
    + '<button class="nav-btn ' + (currentView === 'logs' ? 'active' : '') + '" id="nav-logs"><span>📋</span> Logs</button>'
    + '<button class="nav-btn ' + (currentView === 'expert' ? 'active' : '') + '" id="nav-expert"><span>🔬</span> Expert</button>'
    + '</div>'
    + '<div class="header-status"><span class="status-dot ' + (conn.known ? (conn.online ? 'online' : '') : 'unknown') + '" id="header-dot"></span><span id="header-status-text">' + conn.label + '</span></div>';
  app.appendChild(header);

  if (!isElevated) {
    const banner = document.createElement('div');
    banner.className = 'elevation-banner';
    banner.innerHTML = '<span class="elevation-banner-icon">🛡️</span>'
      + '<span><b>DualLink s\'exécute sans droits administrateur.</b> '
      + 'Les actions réseau (activation des cartes, routes, métriques, failover) seront refusées par Windows. '
      + 'Relancez l\'application en tant qu\'administrateur : clic droit sur duallink.exe → Exécuter en tant qu\'administrateur.</span>';
    app.appendChild(banner);
  }

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
    mode.innerHTML = '<div class="section-title">Mode de mutualisation</div>'
      + '<div class="mode-selector">'
      + '<button class="mode-btn ' + (currentMode === 'none' ? 'active' : '') + '" data-mode="none"><span class="mode-btn-icon">🔀</span><div class="mode-btn-text"><span class="mode-btn-label">Individuel</span><span class="mode-btn-desc">Chaque carte gère son trafic</span></div></button>'
      + '<button class="mode-btn ' + (currentMode === 'load-balancing' ? 'active' : '') + '" data-mode="load-balancing"><span class="mode-btn-icon">⚖️</span><div class="mode-btn-text"><span class="mode-btn-label">Load Balancing</span><span class="mode-btn-desc">Répartir le trafic sur les deux</span></div></button>'
      + '<button class="mode-btn ' + (currentMode === 'failover' ? 'active' : '') + '" data-mode="failover"><span class="mode-btn-icon">🛡️</span><div class="mode-btn-text"><span class="mode-btn-label">Failover</span><span class="mode-btn-desc">Backup automatique si chute</span></div></button>'
      + '</div>'
      + '<div id="failover-banner">' + renderFailoverBanner() + '</div>';
    app.appendChild(mode);
    lastFailoverSignature = failoverSignature();

    const cards = document.createElement('div');
    cards.className = 'glass';
    cards.innerHTML = '<div class="section-title">Interfaces réseau</div><div class="adapter-list">' + adapterCards + '</div>';
    app.appendChild(cards);

    const dash = document.createElement('div');
    dash.id = 'dashboard-card';
    dash.innerHTML = renderDashboard();
    app.appendChild(dash);

    const status = document.createElement('div');
    status.className = 'glass status-bar';
    status.innerHTML = '<div class="status-bar-left" id="status-bar-content">' + renderStatus() + '</div>';
    app.appendChild(status);

    const eligible = adapters.filter(a => a.status === 'Up' && isModeEligible(a)).length;
    const actions = document.createElement('div');
    actions.className = 'actions-bar';
    actions.innerHTML = '<button class="btn" id="btn-refresh"><span>🔄</span> Actualiser</button>'
      + '<button class="btn btn-primary" id="btn-apply" ' + (eligible < 2 ? 'disabled' : '') + '><span>💾</span> Appliquer le mode</button>';
    app.appendChild(actions);
  }

  const toasts = document.createElement('div');
  toasts.className = 'toast-container';
  toasts.id = 'toasts';
  app.appendChild(toasts);

  bindEvents();

  if (currentView === 'main') {
    drawDashboardCanvas();
  }
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
  // Taille logique fixe: relire canvas.width après l'avoir multiplié par le
  // DPR faisait grossir le canevas à chaque tick sur un écran 125/150 %.
  const W = CANVAS_W;
  const H = CANVAS_H;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = W * dpr;
  canvas.height = H * dpr;
  canvas.style.width = W + 'px';
  canvas.style.height = H + 'px';
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

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

function escapeHtml(value) {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

async function loadLogFiles() {
  try {
    logFiles = await api.getLogFiles();
  } catch (err) {
    logFiles = [];
  }
}

// Charge le log affiché. `force` relit même sans changement (bouton 🔄).
async function loadLogs(force) {
  try {
    if (selectedLogDate) {
      logContent = await api.getLogByDate(selectedLogDate);
      paintLogs();
      return;
    }

    // Le backend ne renvoie le contenu que si le fichier a grandi.
    const update = await api.getLogsIfChanged(force ? 0 : logLength);
    if (!update) return;

    logContent = update.content;
    logLength = update.length;
    paintLogs();
  } catch (err) {
    // Échec transitoire (IPC, fichier verrouillé): repartir d'une lecture
    // complète au prochain tick, sinon le message d'erreur reste figé tant
    // que le fichier n'a pas grossi.
    logLength = 0;
    logContent = 'Erreur: ' + err;
    paintLogs();
  }
}

// Repeint le journal sans reconstruire la vue: conserve la position de
// lecture, sauf si l'utilisateur était déjà en bas.
function paintLogs(initial) {
  const el = document.getElementById('log-content');
  if (!el) return;

  const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
  const prevTop = el.scrollTop;
  el.innerHTML = logContent.trim().split('\n').map(colorizeLogLine).join('\n');
  el.scrollTop = initial || atBottom ? el.scrollHeight : prevTop;
}

async function pollLogs() {
  // Un fichier daté ne change plus; seul le log du jour est interrogé.
  // Fenêtre masquée ou onglet inactif: personne ne lit, on ne sonde pas.
  if (currentView !== 'logs' || selectedLogDate || logRefreshBusy || document.hidden) return;
  logRefreshBusy = true;
  try {
    await loadLogs();
  } finally {
    logRefreshBusy = false;
  }
}

function startLogAutoRefresh() {
  clearLogAutoRefresh();
  logAutoRefresh = setInterval(pollLogs, LOG_POLL_MS);
  pollLogs();
}

function clearLogAutoRefresh() {
  if (logAutoRefresh) { clearInterval(logAutoRefresh); logAutoRefresh = null; }
}

function colorizeLogLine(line) {
  const safe = escapeHtml(line);
  if (line.includes('ERROR') || line.includes('PANIC')) return '<span class="log-error">' + safe + '</span>';
  if (line.includes('FAILOVER')) return '<span class="log-failover">' + safe + '</span>';
  if (line.includes('CMD')) return '<span class="log-cmd">' + safe + '</span>';
  if (line.includes('===')) return '<span class="log-separator">' + safe + '</span>';
  if (line.includes('Monitor')) return '<span class="log-monitor">' + safe + '</span>';
  return '<span class="log-normal">' + safe + '</span>';
}

function todayIso() {
  const d = new Date();
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return d.getFullYear() + '-' + month + '-' + day;
}

function renderLogsView() {
  const lines = logContent ? logContent.trim().split('\n') : [];
  const colored = lines.map(colorizeLogLine).join('\n');
  const fileOptions = logFiles.map(f => {
    const date = f.replace('.log', '');
    const sel = selectedLogDate === date ? ' selected' : '';
    return '<option value="' + date + '"' + sel + '>' + date + '</option>';
  }).join('');
  const today = todayIso();
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
  const s = settingsData || { interval_secs: 5, ping_target: '1.1.1.1', adapter_refresh_secs: 30 };
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
    + '<input type="text" id="setting-target" class="settings-input settings-input-text" value="' + s.ping_target + '" placeholder="1.1.1.1">'
    + '</div>'
    + '<span class="settings-hint">IP ou hostname pour tester la connectivité (défaut: 1.1.1.1)</span>'
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
    + '<div class="settings-note">Les réglages sont appliqués au prochain cycle de monitoring.</div>';
  return container;
}

function bindFailoverDisable() {
  const btn = document.getElementById('btn-disable-failover');
  if (!btn) return;

  btn.addEventListener('click', async () => {
    try {
      failoverState = await api.disableAutoFailover();
      showToast('🛡️ Auto Failover désactivé', 'info');
      render();
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });
}

function bindEvents() {
  bindFailoverDisable();

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
  if (navLogs) navLogs.addEventListener('click', async () => {
    currentView = 'logs';
    await loadLogFiles();
    await loadLogs();
    startLogAutoRefresh();
    render();
    paintLogs(true);
  });
  const navExpert = document.getElementById('nav-expert');
  if (navExpert) navExpert.addEventListener('click', async () => { currentView = 'expert'; clearLogAutoRefresh(); await loadExpertDetails(); render(); });


  // Settings save
  const btnSave = document.getElementById('btn-settings-save');
  if (btnSave) btnSave.addEventListener('click', async () => {
    const interval = parseInt(document.getElementById('setting-interval').value) || 5;
    const target = (document.getElementById('setting-target').value || '1.1.1.1').trim();
    const refresh = parseInt(document.getElementById('setting-refresh').value) || 30;
    try {
      settingsData = await api.saveSettings(interval, target, refresh);
      showToast('✅ Réglages sauvegardés', 'success');
    } catch (err) {
      showToast('❌ Erreur: ' + err, 'error');
    }
  });

  // Settings defaults
  const btnDefaults = document.getElementById('btn-settings-defaults');
  if (btnDefaults) btnDefaults.addEventListener('click', async () => {
    try {
      settingsData = await api.saveSettings(5, '1.1.1.1', 30);
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
    logLength = 0;
    await loadLogs();
    render();
    paintLogs(true);
  });
  const btnLogRefresh = document.getElementById('btn-log-refresh');
  if (btnLogRefresh) btnLogRefresh.addEventListener('click', async () => {
    await loadLogs(true);
    paintLogs(true);
  });
  const btnLogAuto = document.getElementById('btn-log-auto');
  if (btnLogAuto) btnLogAuto.addEventListener('click', () => {
    if (logAutoRefresh) { clearLogAutoRefresh(); } else { startLogAutoRefresh(); }
    btnLogAuto.textContent = logAutoRefresh ? '⏸ Pause' : '▶ Auto';
  });

  document.querySelectorAll('.toggle input').forEach(input => {
    input.addEventListener('change', async (e) => {
      const name = e.target.dataset.adapter;
      const checked = e.target.checked;
      e.target.disabled = true;
      try {
        if (checked) {
          await api.enableAdapter(name);
        } else {
          await api.disableAdapter(name);
        }
        // Après le rendu: un refresh avant le toast le faisait disparaître.
        await refreshAdapters();
        showToast(checked ? '✅ ' + name + ' activé' : '⛔ ' + name + ' désactivé', 'success');
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

  const btnApply = document.getElementById('btn-apply');
  if (btnApply) btnApply.addEventListener('click', async () => {
    // Les cartes Up incluent les commutateurs virtuels (Hyper-V, VMware,
    // VPN): appliquer une métrique dessus ne route aucun trafic.
    const active = adapters.filter(a => a.status === 'Up' && isModeEligible(a));
    if (active.length < 2) {
      showToast('⚠️ Il faut au moins 2 cartes réseau physiques actives', 'error');
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

// Met à jour les éléments vivants sans reconstruire la page.
//
// `render()` à chaque tick (5 s) détruisait les toasts, les écouteurs et la
// position de lecture du journal: d'où l'impression que tout clignotait.
function patchLiveView() {
  const conn = connectivity();

  const dot = document.getElementById('header-dot');
  if (dot) {
    dot.className = 'status-dot ' + (conn.known ? (conn.online ? 'online' : '') : 'unknown');
  }

  const headerText = document.getElementById('header-status-text');
  if (headerText) headerText.textContent = conn.label;

  if (currentView !== 'main') return;

  document.querySelectorAll('[data-card-index]').forEach(card => {
    const adapter = adapters[Number(card.dataset.cardIndex)];
    if (!adapter) return;

    const state = adapterConnectivity(adapter);
    const badge = card.querySelector('.adapter-connectivity');
    if (badge) {
      badge.className = 'adapter-connectivity ' + state.cls;
      badge.textContent = state.label;
    }

    const mon = getMonitorForAdapter(adapter.name);
    const latency = mon && mon.status_known !== false ? mon.latency_ms : null;
    const latEl = card.querySelector('.adapter-latency');
    if (latEl) {
      latEl.hidden = latency == null;
      latEl.className = 'adapter-latency' + (latency != null ? ' ' + latencyClass(latency) : '');
      latEl.textContent = latency != null ? '🎯 ' + latency + 'ms' : '';
    }
  });

  const status = document.getElementById('status-bar-content');
  if (status) status.innerHTML = renderStatus();

  const dash = document.getElementById('dashboard-card');
  if (dash) {
    dash.innerHTML = renderDashboard();
    drawDashboardCanvas();
  }

  if (failoverSignature() !== lastFailoverSignature) {
    const banner = document.getElementById('failover-banner');
    if (banner) banner.innerHTML = renderFailoverBanner();
    lastFailoverSignature = failoverSignature();
    bindFailoverDisable();
  }
}

// Compare deux listes de noms sans tenir compte de l'ordre renvoyé par
// PowerShell.
function sameAdapterNames(left, right) {
  if (left.length !== right.length) return false;
  const a = [...left].sort();
  const b = [...right].sort();
  return a.every((name, i) => name === b[i]);
}

export function updateMonitorState(state) {
  monitorData = state;
  if (state.failover) failoverState = state.failover;

  // Le monitor voit les cartes au même rythme que son refresh. Si sa liste
  // diffère de celle affichée (téléphone branché, carte ajoutée/retirée),
  // recharger les cartes une fois au lieu d'attendre un clic sur Actualiser.
  const names = state.adapter_names || [];
  if (names.length > 0 && currentView === 'main' && !adapterRefreshBusy
      && !sameAdapterNames(names, adapters.map(a => a.name))) {
    adapterRefreshBusy = true;
    refreshAdapters().finally(() => { adapterRefreshBusy = false; });
  }

  // Tant que la première mesure n'a pas tranché, l'historique reste vide.
  if (state.internet_known) {
    if (state.internet_reachable && state.overall_latency_ms != null) {
      latencyHistory.push({ ms: state.overall_latency_ms, time: state.last_check, ok: true });
    } else {
      latencyHistory.push({ ms: null, time: state.last_check, ok: state.internet_reachable });
    }
    if (latencyHistory.length > MAX_HISTORY) latencyHistory.shift();
  }

  patchLiveView();
}

export function showToast(message, type) {
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

// Message affiché quand l'utilisateur clique sur X: l'application est
// masquée dans la zone de notification, pas fermée. Sans ce message, il la
// cherche dans la barre des tâches et conclut qu'elle a planté.
export function showTrayHint() {
  const container = document.getElementById('toasts');
  if (!container) return;

  // Pas de timeout: la fenêtre se cache 4s plus tard et emporte le toast.
  // L'utilisateur a eu le temps de lire, et rien ne traîne à l'écran.
  const toast = document.createElement('div');
  toast.className = 'toast info tray-hint';
  toast.innerHTML = '<span class="toast-icon">📥</span>'
    + '<span>DualLink reste actif dans la zone de notification.<br>'
    + 'Clic droit sur son icône (près de l\'horloge) → <b>Quitter</b>.</span>';
  container.appendChild(toast);
}
