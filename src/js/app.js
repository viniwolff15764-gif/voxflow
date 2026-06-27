// ---- Tauri bridge (with a no-op fallback so the page can be previewed in a plain browser) ----
const TAURI = window.__TAURI__ || null;
const invoke = TAURI ? TAURI.core.invoke : async () => ({});
const listen = TAURI ? TAURI.event.listen : (_e, _cb) => {};
const getCurrentWindow = TAURI ? TAURI.window.getCurrentWindow : () => ({ hide() {} });

const pill = document.getElementById('pill');
const settings = document.getElementById('settings');
const hint = document.getElementById('hint');
const statusEl = document.getElementById('status');
const wave = document.getElementById('wave');

const PILL_SIZE = { w: 340, h: 88 };
const SETTINGS_SIZE = { w: 380, h: 640 };
const BARS = 22;

let isSettings = false;
let currentHotkey = 'F9';

// ---- Build waveform bars ----
const bars = [];
for (let i = 0; i < BARS; i++) {
  const b = document.createElement('span');
  b.className = 'bar';
  wave.appendChild(b);
  // Bell-shaped weight: taller in the middle.
  const x = (i - (BARS - 1) / 2) / (BARS / 2);
  bars.push({
    el: b,
    weight: 0.35 + 0.65 * Math.exp(-x * x * 2.2),
    phase: Math.random() * Math.PI * 2,
    speed: 0.06 + Math.random() * 0.05,
  });
}

let targetLevel = 0; // 0..1 from mic
let dispLevel = 0; // smoothed
let mode = 'idle'; // idle | recording | processing
let t = 0;

function animate() {
  t += 1;
  dispLevel += (targetLevel - dispLevel) * 0.28;

  for (let i = 0; i < BARS; i++) {
    const b = bars[i];
    let h;
    if (mode === 'recording') {
      const wobble = 0.55 + 0.45 * Math.sin(t * b.speed + b.phase);
      h = 0.12 + dispLevel * b.weight * wobble * 1.7;
    } else if (mode === 'processing') {
      // Traveling sine wave (loading look).
      h = 0.18 + 0.5 * (0.5 + 0.5 * Math.sin(t * 0.12 - i * 0.5));
    } else {
      h = 0.1;
    }
    b.el.style.transform = `scaleY(${Math.max(0.08, Math.min(1, h)).toFixed(3)})`;
  }
  requestAnimationFrame(animate);
}
requestAnimationFrame(animate);

// ---- State ----
function setState(s) {
  pill.className = 'pill ' + s;
  if (s === 'recording') mode = 'recording';
  else if (s === 'processing') mode = 'processing';
  else mode = 'idle';
}

function showPlaceholder() {
  hint.textContent = currentHotkey + ' para ditar';
  statusEl.textContent = '';
}

// ---- Drag ----
pill.addEventListener('mousedown', (e) => {
  if (e.target.closest('button')) return;
  invoke('drag_window');
});
document.querySelector('.settings-header').addEventListener('mousedown', (e) => {
  if (e.target.closest('button')) return;
  invoke('drag_window');
});

// ---- Buttons ----
document.getElementById('btn-settings').addEventListener('click', async () => {
  isSettings = true;
  pill.classList.add('hidden');
  settings.classList.remove('hidden');
  await invoke('resize_window', { width: SETTINGS_SIZE.w, height: SETTINGS_SIZE.h });
  loadConfig();
});
document.getElementById('btn-back').addEventListener('click', async () => {
  isSettings = false;
  settings.classList.add('hidden');
  pill.classList.remove('hidden');
  await invoke('resize_window', { width: PILL_SIZE.w, height: PILL_SIZE.h });
});
document.getElementById('btn-minimize').addEventListener('click', () => getCurrentWindow().hide());
document.getElementById('btn-close').addEventListener('click', () => invoke('exit_app'));

// ---- Events from backend ----
listen('recording-started', () => { if (!isSettings) { setState('recording'); statusEl.textContent = ''; } });
listen('audio-level', (e) => { targetLevel = typeof e.payload === 'number' ? e.payload : 0; });
listen('processing', () => { if (!isSettings) { setState('processing'); targetLevel = 0; } });
listen('command-processing', () => { if (!isSettings) { setState('processing'); } });

listen('recording-stopped', (e) => {
  if (isSettings) return;
  const txt = (e.payload || '').trim();
  if (txt) showDone(txt);
  else { setState('idle'); showPlaceholder(); }
});
listen('command-complete', (e) => { if (!isSettings) showDone((e.payload || '').trim() || '✓'); });

listen('recording-error', (e) => {
  if (isSettings) return;
  setState('error');
  statusEl.textContent = e.payload;
  setTimeout(() => { setState('idle'); showPlaceholder(); }, 4500);
});

// Show the transcribed text briefly (confirms it worked, even if pasting didn't).
function showDone(text) {
  setState('done');
  statusEl.textContent = text.length > 42 ? text.slice(0, 41) + '…' : text;
  setTimeout(() => { setState('idle'); showPlaceholder(); }, 2600);
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
    opacity: 0.9, autostart: true,
    window_x: 0, window_y: 0, window_width: PILL_SIZE.w, window_height: PILL_SIZE.h,
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
async function init() {
  try {
    const c = await invoke('get_config');
    currentHotkey = c.hotkey || 'F9';
    if (!c.groq_api_key) hint.textContent = 'Clique ⚙ e cole sua API Key';
    else showPlaceholder();
  } catch (e) { showPlaceholder(); }
}
init();

// ---- Preview helpers (only used when opened in a plain browser) ----
if (!TAURI) {
  window.__demo = {
    rec() { setState('recording'); let l = 0; window.__demoTimer = setInterval(() => { l = 0.3 + Math.random() * 0.7; targetLevel = l; }, 120); },
    proc() { clearInterval(window.__demoTimer); setState('processing'); },
    idle() { clearInterval(window.__demoTimer); targetLevel = 0; setState('idle'); showPlaceholder(); },
    error(m) { setState('error'); statusEl.textContent = m || 'Erro de exemplo'; },
  };
}
