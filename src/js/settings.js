const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.shell;

async function loadConfig() {
  const config = await invoke('get_config');

  document.getElementById('api-key').value = config.groq_api_key;
  document.getElementById('hotkey').value = config.hotkey;
  document.getElementById('language').value = config.language;
  document.getElementById('auto-paste').checked = config.auto_paste;
  document.getElementById('command-mode').checked = config.command_mode;
  document.getElementById('command-prefix').value = config.command_prefix;
  document.getElementById('llm-model').value = config.llm_model;
  document.getElementById('opacity').value = config.opacity * 100;
  document.getElementById('opacity-value').textContent = Math.round(config.opacity * 100) + '%';
  document.getElementById('autostart').checked = config.autostart;
}

document.getElementById('btn-save').addEventListener('click', async () => {
  const config = {
    groq_api_key: document.getElementById('api-key').value,
    hotkey: document.getElementById('hotkey').value,
    language: document.getElementById('language').value,
    auto_paste: document.getElementById('auto-paste').checked,
    command_mode: document.getElementById('command-mode').checked,
    command_prefix: document.getElementById('command-prefix').value,
    llm_model: document.getElementById('llm-model').value,
    opacity: parseInt(document.getElementById('opacity').value) / 100,
    autostart: document.getElementById('autostart').checked,
    window_x: 100,
    window_y: 100,
    window_width: 350,
    window_height: 200,
  };

  await invoke('save_config_cmd', { newConfig: config });
  alert('Configurações salvas! Reinicie o VoxFlow para aplicar.');
});

document.getElementById('opacity').addEventListener('input', (e) => {
  document.getElementById('opacity-value').textContent = e.target.value + '%';
});

document.getElementById('link-groq').addEventListener('click', () => {
  open('https://console.groq.com/keys');
});

loadConfig();
