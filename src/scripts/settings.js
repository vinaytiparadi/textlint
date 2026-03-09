// TextLint Settings Script
// Manages settings UI, auto-saves changes, communicates with Rust backend via Tauri IPC

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentSettings = null;
let currentApiKey = '';
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
    const result = await invoke('get_settings');
    // api_key comes back from keyring via the flattened response
    currentApiKey = result.api_key || '';
    // The rest of the fields map directly to AppSettings
    const { api_key: _ignored, ...settings } = result;
    currentSettings = settings;
    populateUI(currentSettings, currentApiKey);
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function populateUI(settings, apiKey) {
  // API Key
  document.getElementById('api-key-input').value = apiKey || '';
  updateApiKeyStatus(apiKey);

  // Correction Behavior
  document.getElementById('learn-mode-input').checked = settings.learn_mode;
  document.getElementById('auto-apply-input').checked = settings.auto_apply;
  document.getElementById('strictness-select').value = settings.strictness;
  document.getElementById('enhance-writing-input').checked = settings.enhance_writing;

  // Appearance
  document.getElementById('floating-icon-input').checked = settings.show_floating_icon;
  document.getElementById('theme-select').value = settings.theme;
  document.getElementById('toast-duration-slider').value = settings.toast_duration;
  document.getElementById('toast-duration-value').textContent = settings.toast_duration + 's';

  // Startup
  document.getElementById('startup-input').checked = settings.launch_on_startup;

  // Disabled Apps — load running processes list
  loadRunningApps();

  // Apply theme
  applyTheme(settings.theme);
}

async function saveSettings() {
  if (!currentSettings) return;

  try {
    console.log('[TextLint] Saving settings, apiKey length:', currentApiKey.length);
    await invoke('save_settings', { settings: currentSettings, apiKey: currentApiKey });
    showSaveIndicator();
  } catch (e) {
    console.error('Failed to save settings:', e);
    // Show error visibly in the page
    const status = document.getElementById('api-key-status');
    if (status) {
      status.className = 'api-key-status invalid';
      status.textContent = '✗ Save failed: ' + e;
    }
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
// Disabled Apps (Running Apps Toggle List)
// ===========================
let runningApps = [];
let searchFilter = '';

async function loadRunningApps() {
  try {
    runningApps = await invoke('get_running_apps');
    renderRunningAppsList();
  } catch (e) {
    console.error('Failed to load running apps:', e);
    document.getElementById('running-apps-list').innerHTML =
      '<div class="setting-description" style="padding: 12px 0; text-align: center;">Failed to load running apps.</div>';
  }
}

function renderRunningAppsList() {
  const list = document.getElementById('running-apps-list');
  list.innerHTML = '';

  // Merge: running apps + manually added apps that aren't currently running
  const allApps = [...new Set([...runningApps, ...currentSettings.disabled_apps])].sort();
  const filtered = searchFilter
    ? allApps.filter(app => app.includes(searchFilter))
    : allApps;

  if (filtered.length === 0) {
    list.innerHTML = '<div class="setting-description" style="padding: 12px 0; text-align: center;">No matching apps found.</div>';
    return;
  }

  filtered.forEach(app => {
    const isDisabled = currentSettings.disabled_apps.includes(app);
    const isRunning = runningApps.includes(app);
    const item = document.createElement('div');
    item.className = 'app-toggle-item' + (isDisabled ? ' blocked' : '');
    item.innerHTML = `
      <div class="app-toggle-info">
        <span class="app-name">${escapeHtml(app)}</span>
        ${!isRunning ? '<span class="app-badge manual">manual</span>' : ''}
        ${isDisabled ? '<span class="app-badge blocked-badge">blocked</span>' : ''}
      </div>
      <label class="toggle-switch toggle-sm">
        <input type="checkbox" ${isDisabled ? 'checked' : ''} />
        <span class="toggle-slider"></span>
      </label>
    `;
    item.querySelector('input').addEventListener('change', (e) => {
      if (e.target.checked) {
        if (!currentSettings.disabled_apps.includes(app)) {
          currentSettings.disabled_apps.push(app);
        }
      } else {
        const idx = currentSettings.disabled_apps.indexOf(app);
        if (idx > -1) currentSettings.disabled_apps.splice(idx, 1);
      }
      renderRunningAppsList();
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
  renderRunningAppsList();
  input.value = '';
  debouncedSave();
}

// ===========================
// Event Listeners
// ===========================
function setupListeners() {
  // API Key
  document.getElementById('api-key-input').addEventListener('input', (e) => {
    currentApiKey = e.target.value;
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

  // Enhance Writing
  document.getElementById('enhance-writing-input').addEventListener('change', (e) => {
    currentSettings.enhance_writing = e.target.checked;
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

  // Search running apps
  document.getElementById('app-search-input').addEventListener('input', (e) => {
    searchFilter = e.target.value.trim().toLowerCase();
    renderRunningAppsList();
  });

  // Refresh running apps list
  document.getElementById('refresh-apps-btn').addEventListener('click', loadRunningApps);
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
