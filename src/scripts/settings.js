// TextLint Settings Script
// Manages settings UI, auto-saves changes, communicates with Rust backend via Tauri IPC

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentSettings = null;
let saveTimeout = null;

// ===========================
// Theme Management
// ===========================
function applyTheme(theme) {
  if (theme === 'system') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
  } else {
    document.documentElement.setAttribute('data-theme', theme);
  }
}

// ===========================
// Settings Load / Save
// ===========================
async function loadSettings() {
  try {
    currentSettings = await invoke('get_settings');
    populateUI(currentSettings);
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function populateUI(settings) {
  // API Key
  document.getElementById('api-key-input').value = settings.api_key || '';
  updateApiKeyStatus(settings.api_key);

  // Correction Behavior
  document.getElementById('learn-mode-input').checked = settings.learn_mode;
  document.getElementById('auto-apply-input').checked = settings.auto_apply;
  document.getElementById('strictness-select').value = settings.strictness;

  // Appearance
  document.getElementById('floating-icon-input').checked = settings.show_floating_icon;
  document.getElementById('theme-select').value = settings.theme;
  document.getElementById('toast-duration-slider').value = settings.toast_duration;
  document.getElementById('toast-duration-value').textContent = settings.toast_duration + 's';

  // Startup
  document.getElementById('startup-input').checked = settings.launch_on_startup;

  // Disabled Apps
  renderDisabledApps(settings.disabled_apps);

  // Apply theme
  applyTheme(settings.theme);
}

async function saveSettings() {
  if (!currentSettings) return;

  try {
    await invoke('save_settings', { settings: currentSettings });
    showSaveIndicator();
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
}

function debouncedSave() {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSettings, 300);
}

function showSaveIndicator() {
  const indicator = document.getElementById('save-indicator');
  indicator.classList.add('visible');
  setTimeout(() => indicator.classList.remove('visible'), 1500);
}

// ===========================
// API Key
// ===========================
function updateApiKeyStatus(key) {
  const status = document.getElementById('api-key-status');
  if (key && key.length > 10) {
    status.className = 'api-key-status valid';
    status.textContent = '✓ API key set';
  } else if (key && key.length > 0) {
    status.className = 'api-key-status invalid';
    status.textContent = '⚠ API key looks too short';
  } else {
    status.className = 'api-key-status invalid';
    status.textContent = '⚠ API key required';
  }
}

// ===========================
// Disabled Apps
// ===========================
function renderDisabledApps(apps) {
  const list = document.getElementById('disabled-apps-list');
  list.innerHTML = '';

  if (apps.length === 0) {
    list.innerHTML = '<div class="setting-description" style="padding: 8px 0;">No apps disabled. All apps will receive grammar corrections.</div>';
    return;
  }

  apps.forEach((app, index) => {
    const item = document.createElement('div');
    item.className = 'app-item';
    item.innerHTML = `
      <span class="app-name">${escapeHtml(app)}</span>
      <button class="btn" data-index="${index}" style="min-height:24px; padding: 2px 8px; font-size: 11px;">Remove</button>
    `;
    item.querySelector('button').addEventListener('click', () => {
      currentSettings.disabled_apps.splice(index, 1);
      renderDisabledApps(currentSettings.disabled_apps);
      debouncedSave();
    });
    list.appendChild(item);
  });
}

function addDisabledApp() {
  const input = document.getElementById('add-app-input');
  const appName = input.value.trim().toLowerCase();

  if (!appName) return;
  if (currentSettings.disabled_apps.includes(appName)) return;

  currentSettings.disabled_apps.push(appName);
  renderDisabledApps(currentSettings.disabled_apps);
  input.value = '';
  debouncedSave();
}

// ===========================
// Event Listeners
// ===========================
function setupListeners() {
  // API Key
  document.getElementById('api-key-input').addEventListener('input', (e) => {
    currentSettings.api_key = e.target.value;
    updateApiKeyStatus(e.target.value);
    debouncedSave();
  });

  document.getElementById('api-key-toggle').addEventListener('click', () => {
    const input = document.getElementById('api-key-input');
    const toggle = document.getElementById('api-key-toggle');
    if (input.type === 'password') {
      input.type = 'text';
      toggle.textContent = '🔒';
    } else {
      input.type = 'password';
      toggle.textContent = '👁';
    }
  });

  // Learn Mode
  document.getElementById('learn-mode-input').addEventListener('change', (e) => {
    currentSettings.learn_mode = e.target.checked;
    debouncedSave();
  });

  // Auto Apply
  document.getElementById('auto-apply-input').addEventListener('change', (e) => {
    currentSettings.auto_apply = e.target.checked;
    debouncedSave();
  });

  // Strictness
  document.getElementById('strictness-select').addEventListener('change', (e) => {
    currentSettings.strictness = e.target.value;
    debouncedSave();
  });

  // Floating Icon
  document.getElementById('floating-icon-input').addEventListener('change', (e) => {
    currentSettings.show_floating_icon = e.target.checked;
    debouncedSave();
  });

  // Theme
  document.getElementById('theme-select').addEventListener('change', (e) => {
    currentSettings.theme = e.target.value;
    applyTheme(e.target.value);
    debouncedSave();
  });

  // Toast Duration
  document.getElementById('toast-duration-slider').addEventListener('input', (e) => {
    const val = parseInt(e.target.value, 10);
    document.getElementById('toast-duration-value').textContent = val + 's';
    currentSettings.toast_duration = val;
    debouncedSave();
  });

  // Startup
  document.getElementById('startup-input').addEventListener('change', (e) => {
    currentSettings.launch_on_startup = e.target.checked;
    debouncedSave();
  });

  // Add disabled app
  document.getElementById('add-app-btn').addEventListener('click', addDisabledApp);
  document.getElementById('add-app-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addDisabledApp();
  });
}

// ===========================
// Utilities
// ===========================
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ===========================
// Listen for external events
// ===========================
async function setupEvents() {
  await listen('learn-mode-changed', (event) => {
    if (currentSettings) {
      currentSettings.learn_mode = event.payload;
      document.getElementById('learn-mode-input').checked = event.payload;
    }
  });
}

// ===========================
// Init
// ===========================
document.addEventListener('DOMContentLoaded', async () => {
  await loadSettings();
  setupListeners();
  setupEvents();
});
