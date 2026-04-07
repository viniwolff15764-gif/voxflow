const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const pill = document.getElementById('pill');
const pillText = document.getElementById('pill-text');
const settings = document.getElementById('settings');

const PILL_SIZE = { w: 280, h: 56 };
const SETTINGS_SIZE = { w: 320, h: 460 };

let isSettings = false;
let currentHotkey = 'F9';

// === DRAG ===
pill.addEventListener('mousedown', (e) => {
  if (e.target.tagName === 'BUTTON') return;
  invoke('drag_window');
});

document.querySelector('.settings-header').addEventListener('mousedown', (e) => {
  if (e.target.tagName === 'BUTTON') return;
  invoke('drag_window');
});

// === SETTINGS TOGGLE ===
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

// === MINIMIZE (hide to tray) ===
document.getElementById('btn-minimize').addEventListener('click', () => {
  getCurrentWindow().hide();
});

// === CLOSE (exit app) ===
document.getElementById('btn-close').addEventListener('click', () => {
  invoke('exit_app');
});

// === RECORDING EVENTS ===
function setState(state) {
  pill.className = 'pill ' + state;
}

listen('recording-started', () => {
  if (isSettings) return;
  setState('recording');
  pillText.innerHTML = '<span class="cursor-blink"></span>';
});

listen('transcription-update', (event) => {
  if (isSettings) return;
  pillText.innerHTML = event.payload + '<span class="cursor-blink"></span>';
});

listen('recording-stopped', (event) => {
  if (isSettings) return;
  const text = event.payload;
  if (text) {
    pillText.textContent = text;
  } else {
    showPlaceholder();
  }
  setState('idle');
});

listen('command-processing', () => {
  if (isSettings) return;
  setState('processing');
  pillText.textContent = 'Processando...';
});

listen('command-complete', (event) => {
  if (isSettings) return;
  pillText.textContent = event.payload;
  setState('idle');
});

listen('recording-error', (event) => {
  if (isSettings) return;
  setState('error');
  pillText.textContent = event.payload;
  setTimeout(() => {
    setState('idle');
    showPlaceholder();
  }, 5000);
});

function showPlaceholder() {
  pillText.innerHTML = '<span class="placeholder">' + currentHotkey + ' para ditar</span>';
}

// === CONFIG ===
async function loadConfig() {
  try {
    const c = await invoke('get_config');
    document.getElementById('api-key').value = c.groq_api_key || '';
    document.getElementById('hotkey').value = c.hotkey || 'F9';
    document.getElementById('language').value = c.language || 'pt';
    document.getElementById('auto-paste').checked = c.auto_paste;
    document.getElementById('command-mode').checked = c.command_mode;
    document.getElementById('command-prefix').value = c.command_prefix || 'comando';
    document.getElementById('llm-model').value = c.llm_model || 'llama-3.3-70b-versatile';
  } catch (e) {
    console.error('Config load error:', e);
  }
}

document.getElementById('btn-save').addEventListener('click', async () => {
  const apiKey = document.getElementById('api-key').value.trim();
  if (!apiKey) {
    alert('Cole sua Groq API Key! Crie grátis em console.groq.com/keys');
    return;
  }

  const config = {
    groq_api_key: apiKey,
    hotkey: document.getElementById('hotkey').value,
    language: document.getElementById('language').value,
    auto_paste: document.getElementById('auto-paste').checked,
    command_mode: document.getElementById('command-mode').checked,
    command_prefix: document.getElementById('command-prefix').value,
    llm_model: document.getElementById('llm-model').value,
    opacity: 0.85,
    autostart: true,
    window_x: 0.0,
    window_y: 0.0,
    window_width: 280.0,
    window_height: 56.0,
  };
  try {
    await invoke('save_config_cmd', { newConfig: config });
    const msg = document.getElementById('save-msg');
    msg.textContent = 'Salvo! Reinicie o VoxFlow para o novo atalho funcionar.';
    msg.style.display = 'block';
    currentHotkey = config.hotkey;
    document.getElementById('hotkey-badge').textContent = currentHotkey;
  } catch (e) {
    alert('Erro: ' + e);
  }
});

document.getElementById('link-groq').addEventListener('click', (e) => {
  e.preventDefault();
  try {
    window.__TAURI__.shell.open('https://console.groq.com/keys');
  } catch (_) {
    window.open('https://console.groq.com/keys');
  }
});

// === INIT ===
async function init() {
  try {
    const c = await invoke('get_config');
    currentHotkey = c.hotkey || 'F9';
    document.getElementById('hotkey-badge').textContent = currentHotkey;

    if (!c.groq_api_key) {
      pillText.innerHTML = '<span class="placeholder">Clique ⚙ e cole sua API Key</span>';
    } else {
      showPlaceholder();
    }
  } catch (e) {
    console.error('Init error:', e);
  }
}

init();
