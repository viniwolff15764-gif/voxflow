const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const widget = document.getElementById('widget');
const textArea = document.getElementById('text-area');
const statusText = document.getElementById('status-text');
const errorMsg = document.getElementById('error-msg');

let history = [];
const MAX_HISTORY = 10;

function setState(state, data) {
  widget.className = 'widget ' + state;

  switch (state) {
    case 'idle':
      statusText.textContent = 'segure para ditar';
      break;
    case 'recording':
      textArea.innerHTML = '<span class="cursor-blink"></span>';
      statusText.textContent = 'gravando...';
      errorMsg.style.display = 'none';
      break;
    case 'processing':
      statusText.textContent = 'processando...';
      break;
    case 'error':
      errorMsg.textContent = data || 'Erro desconhecido';
      errorMsg.style.display = 'block';
      setTimeout(() => setState('idle'), 3000);
      break;
  }
}

listen('recording-started', () => {
  setState('recording');
});

listen('transcription-update', (event) => {
  const text = event.payload;
  textArea.innerHTML = text + '<span class="cursor-blink"></span>';
});

listen('recording-stopped', (event) => {
  const text = event.payload;
  if (text) {
    textArea.textContent = text;
    addToHistory(text);
  } else {
    textArea.innerHTML = '<span class="placeholder">Segure a tecla e fale...</span>';
  }
  setState('idle');
});

listen('command-processing', (event) => {
  setState('processing');
  textArea.textContent = 'Executando: ' + event.payload + '...';
});

listen('command-complete', (event) => {
  textArea.textContent = event.payload;
  addToHistory(event.payload);
  setState('idle');
});

listen('recording-error', (event) => {
  setState('error', event.payload);
});

function addToHistory(text) {
  history.unshift({ text, time: new Date().toLocaleTimeString() });
  if (history.length > MAX_HISTORY) history.pop();
}

document.getElementById('btn-minimize').addEventListener('click', () => {
  getCurrentWindow().hide();
});

document.getElementById('btn-close').addEventListener('click', () => {
  getCurrentWindow().hide();
});

document.getElementById('btn-settings').addEventListener('click', async () => {
  await invoke('open_settings');
});

async function init() {
  const config = await invoke('get_config');

  if (!config.groq_api_key) {
    textArea.innerHTML = '<span class="placeholder">Configure sua Groq API Key em ⚙️</span>';
  } else {
    textArea.innerHTML = '<span class="placeholder">Segure a tecla e fale...</span>';
  }

  document.getElementById('hotkey-display').textContent = config.hotkey;
  setState('idle');
}

init();
