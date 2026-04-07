// Wait for Tauri to be ready
function waitForTauri(callback) {
  if (window.__TAURI__) {
    callback();
  } else {
    setTimeout(() => waitForTauri(callback), 100);
  }
}

waitForTauri(async () => {
  const { invoke } = window.__TAURI__.core;

  // Load config
  try {
    const config = await invoke('get_config');
    document.getElementById('api-key').value = config.groq_api_key || '';
    document.getElementById('language').value = config.language || 'pt';
    document.getElementById('auto-paste').checked = config.auto_paste;
    document.getElementById('command-mode').checked = config.command_mode;
    document.getElementById('command-prefix').value = config.command_prefix || 'comando';
    document.getElementById('llm-model').value = config.llm_model || 'llama-3.3-70b-versatile';
    document.getElementById('opacity').value = (config.opacity || 0.8) * 100;
    document.getElementById('opacity-value').textContent = Math.round((config.opacity || 0.8) * 100) + '%';
    document.getElementById('autostart').checked = config.autostart;
  } catch (e) {
    console.error('Failed to load config:', e);
  }

  // Save
  document.getElementById('btn-save').addEventListener('click', async () => {
    const config = {
      groq_api_key: document.getElementById('api-key').value,
      hotkey: 'Ctrl+Win+Space',
      language: document.getElementById('language').value,
      auto_paste: document.getElementById('auto-paste').checked,
      command_mode: document.getElementById('command-mode').checked,
      command_prefix: document.getElementById('command-prefix').value,
      llm_model: document.getElementById('llm-model').value,
      opacity: parseInt(document.getElementById('opacity').value) / 100,
      autostart: document.getElementById('autostart').checked,
      window_x: 100.0,
      window_y: 100.0,
      window_width: 280.0,
      window_height: 56.0,
    };

    try {
      await invoke('save_config_cmd', { newConfig: config });
      document.getElementById('save-status').textContent = 'Salvo! Reinicie o VoxFlow.';
      document.getElementById('save-status').style.display = 'block';
    } catch (e) {
      alert('Erro ao salvar: ' + e);
    }
  });

  // Opacity slider
  document.getElementById('opacity').addEventListener('input', (e) => {
    document.getElementById('opacity-value').textContent = e.target.value + '%';
  });

  // Groq link — open in system browser
  document.getElementById('link-groq').addEventListener('click', () => {
    try {
      window.__TAURI__.shell.open('https://console.groq.com/keys');
    } catch (e) {
      // Fallback: just show the URL
      prompt('Abra este link no navegador:', 'https://console.groq.com/keys');
    }
  });
});
