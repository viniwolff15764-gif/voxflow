const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const widget = document.getElementById('widget');
const textArea = document.getElementById('text-area');

let history = [];
const MAX_HISTORY = 10;

// Drag the window by clicking anywhere on the widget
widget.addEventListener('mousedown', (e) => {
  if (e.target.tagName === 'BUTTON') return;
  getCurrentWindow().startDragging();
});

function setState(state) {
  widget.className = 'widget ' + state;
}

listen('recording-started', () => {
  setState('recording');
  textArea.innerHTML = '<span class="cursor-blink"></span>';
});

listen('transcription-update', (event) => {
  textArea.innerHTML = event.payload + '<span class="cursor-blink"></span>';
});

listen('recording-stopped', (event) => {
  const text = event.payload;
  if (text) {
    textArea.textContent = text;
    history.unshift({ text, time: new Date().toLocaleTimeString() });
    if (history.length > MAX_HISTORY) history.pop();
  } else {
    textArea.innerHTML = '<span class="placeholder">Ctrl+Win+Space</span>';
  }
  setState('idle');
});

listen('command-processing', () => {
  setState('processing');
  textArea.textContent = 'Processando...';
});

listen('command-complete', (event) => {
  textArea.textContent = event.payload;
  setState('idle');
});

listen('recording-error', (event) => {
  setState('error');
  textArea.textContent = event.payload;
  setTimeout(() => {
    setState('idle');
    textArea.innerHTML = '<span class="placeholder">Ctrl+Win+Space</span>';
  }, 3000);
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
    textArea.innerHTML = '<span class="placeholder">Clique ⚙ para configurar</span>';
  }
}

init();
