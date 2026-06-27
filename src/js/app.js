// ---- Tauri bridge (no-op fallback so the page can be previewed in a browser) ----
const TAURI = window.__TAURI__ || null;
const invoke = TAURI ? TAURI.core.invoke : async () => ({});
const listen = TAURI ? TAURI.event.listen : (_e, _cb) => {};

const pill = document.getElementById('pill');
const settings = document.getElementById('settings');
const statusEl = document.getElementById('status');
const wave = document.getElementById('wave');
const orb = document.getElementById('orb');

const IDLE = { w: 56, h: 56 };
const ACTIVE = { w: 300, h: 60 };
const SETTINGS = { w: 380, h: 600 };
const BARS = 20;

let isSettings = false;
let currentHotkey = 'F9';

async function resize(s) { try { await invoke('resize_widget', { width: s.w, height: s.h }); } catch (_) {} }

// ---- Waveform bars ----
const bars = [];
for (let i = 0; i < BARS; i++) {
  const b = document.createElement('span');
  b.className = 'bar';
  wave.appendChild(b);
  const x = (i - (BARS - 1) / 2) / (BARS / 2);
  bars.push({ el: b, weight: 0.35 + 0.65 * Math.exp(-x * x * 2.2), phase: Math.random() * 6.28, speed: 0.06 + Math.random() * 0.05 });
}
let targetLevel = 0, dispLevel = 0, mode = 'idle', t = 0;
function animate() {
  t += 1;
  dispLevel += (targetLevel - dispLevel) * 0.28;
  for (let i = 0; i < BARS; i++) {
    const b = bars[i];
    let h;
    if (mode === 'recording') h = 0.12 + dispLevel * b.weight * (0.55 + 0.45 * Math.sin(t * b.speed + b.phase)) * 1.7;
    else if (mode === 'processing') h = 0.18 + 0.5 * (0.5 + 0.5 * Math.sin(t * 0.12 - i * 0.5));
    else h = 0.1;
    b.el.style.transform = `scaleY(${Math.max(0.08, Math.min(1, h)).toFixed(3)})`;
  }
  requestAnimationFrame(animate);
}
requestAnimationFrame(animate);

// ---- State ----
function setState(s) {
  pill.className = 'pill ' + s;
  mode = s === 'recording' ? 'recording' : s === 'processing' ? 'processing' : 'idle';
}
async function goIdle() { setState('idle'); await resize(IDLE); }

// ---- Orb click → settings (only when idle) ----
orb.addEventListener('click', () => { if (pill.classList.contains('idle')) openSettings(); });

async function openSettings() {
  isSettings = true;
  pill.classList.add('hidden');
  settings.classList.remove('hidden');
  await resize(SETTINGS);
  loadConfig();
}
document.getElementById('btn-back').addEventListener('click', async () => {
  isSettings = false;
  settings.classList.add('hidden');
  pill.classList.remove('hidden');
  setState('idle');
  await resize(IDLE);
});
document.querySelector('.settings-header').addEventListener('mousedown', (e) => {
  if (e.target.closest('button')) return;
  invoke('drag_window');
});

// ---- Events from backend ----
listen('recording-started', async () => { if (isSettings) return; await resize(ACTIVE); setState('recording'); statusEl.textContent = ''; });
listen('audio-level', (e) => { targetLevel = typeof e.payload === 'number' ? e.payload : 0; });
listen('processing', () => { if (!isSettings) setState('processing'); });
listen('command-processing', () => { if (!isSettings) setState('processing'); });
listen('recording-stopped', (e) => { if (!isSettings) { const txt = (e.payload || '').trim(); txt ? showDone(txt) : goIdle(); } });
listen('command-complete', (e) => { if (!isSettings) showDone((e.payload || '').trim() || '✓'); });
listen('recording-error', async (e) => {
  if (isSettings) return;
  await resize(ACTIVE);
  setState('error');
  statusEl.textContent = e.payload;
  setTimeout(goIdle, 4500);
});
listen('open-settings', () => { if (!isSettings) openSettings(); });

function showDone(text) {
  setState('done');
  statusEl.textContent = text.length > 42 ? text.slice(0, 41) + '…' : text;
  setTimeout(goIdle, 2600);
}

// ---- Config ----
async function loadConfig() {
  try {
    const c = await invoke('get_config');
    document.getElementById('api-key').value = c.groq_api_key || '';
    document.getElementById('hotkey').value = c.hotkey || 'F9';
    document.getElementById('hold-mode').value = c.hold_to_talk === false ? 'toggle' : 'hold';
    document.getElementById('whisper-model').value = c.whisper_model || 'whisper-large-v3-turbo';
    document.getElementById('language').value = c.language || 'pt';
    document.getElementById('auto-paste').checked = c.auto_paste;
    document.getElementById('command-mode').checked = c.command_mode;
    document.getElementById('command-prefix').value = c.command_prefix || 'comando';
    document.getElementById('llm-model').value = c.llm_model || 'llama-3.3-70b-versatile';
  } catch (e) { console.error('Config load error:', e); }
}

document.getElementById('btn-save').addEventListener('click', async () => {
  const apiKey = document.getElementById('api-key').value.trim();
  const msg = document.getElementById('save-msg');
  if (!apiKey) {
    msg.className = 'save-msg error';
    msg.textContent = 'Cole sua Groq API Key (criar grátis em console.groq.com/keys).';
    msg.style.display = 'block';
    return;
  }
  const config = {
    groq_api_key: apiKey,
    hotkey: document.getElementById('hotkey').value,
    hold_to_talk: document.getElementById('hold-mode').value !== 'toggle',
    whisper_model: document.getElementById('whisper-model').value,
    language: document.getElementById('language').value,
    auto_paste: document.getElementById('auto-paste').checked,
    command_mode: document.getElementById('command-mode').checked,
    command_prefix: document.getElementById('command-prefix').value || 'comando',
    llm_model: document.getElementById('llm-model').value,
    opacity: 0.9, autostart: true, window_x: 0, window_y: 0, window_width: IDLE.w, window_height: IDLE.h,
  };
  try {
    await invoke('save_config_cmd', { newConfig: config });
    currentHotkey = config.hotkey;
    msg.className = 'save-msg ok';
    msg.textContent = 'Salvo! Já pode usar.';
    msg.style.display = 'block';
  } catch (e) {
    msg.className = 'save-msg error';
    msg.textContent = 'Erro: ' + e;
    msg.style.display = 'block';
  }
});

document.getElementById('link-groq').addEventListener('click', (e) => {
  e.preventDefault();
  try { window.__TAURI__.shell.open('https://console.groq.com/keys'); }
  catch (_) { window.open('https://console.groq.com/keys'); }
});

// ---- Init ----
(async function init() {
  try { const c = await invoke('get_config'); currentHotkey = c.hotkey || 'F9'; } catch (_) {}
})();

// ---- Preview helpers (browser only) ----
if (!TAURI) {
  window.__demo = {
    rec() { setState('recording'); window.__demoTimer = setInterval(() => { targetLevel = 0.3 + Math.random() * 0.7; }, 120); },
    done(txt) { clearInterval(window.__demoTimer); setState('done'); statusEl.textContent = txt || 'teste teste teste'; },
    idle() { clearInterval(window.__demoTimer); targetLevel = 0; setState('idle'); },
  };
}
