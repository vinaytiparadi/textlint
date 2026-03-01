// GrammarLens Toast Notification Script
// Displays non-intrusive toast notifications for correction results

const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

let toastTimeout = null;
let toastDuration = 3000; // default 3s, overridden by settings

// ===========================
// Theme
// ===========================
function applyTheme() {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
}

// ===========================
// Toast Creation
// ===========================
function showToast(result) {
    const container = document.getElementById('toast-container');
    container.innerHTML = '';

    const toast = document.createElement('div');
    toast.className = 'toast';

    if (result.has_changes) {
        const count = result.num_corrections || result.corrections.length;
        toast.innerHTML = `
      <div class="toast-icon success">✓</div>
      <div class="toast-body">
        <div class="toast-title">Fixed ${count} issue${count !== 1 ? 's' : ''}</div>
        <div class="toast-message">Corrected text has been applied.</div>
        <a class="toast-link" id="toast-see-details">See details</a>
      </div>
      <button class="toast-close" id="toast-close">✕</button>
    `;
    } else {
        toast.innerHTML = `
      <div class="toast-icon success">✓</div>
      <div class="toast-body">
        <div class="toast-title">Looks good!</div>
        <div class="toast-message">No changes needed.</div>
      </div>
      <button class="toast-close" id="toast-close">✕</button>
    `;
    }

    container.appendChild(toast);

    // Event listeners
    const closeBtn = document.getElementById('toast-close');
    if (closeBtn) {
        closeBtn.addEventListener('click', dismissToast);
    }

    const detailsLink = document.getElementById('toast-see-details');
    if (detailsLink) {
        detailsLink.addEventListener('click', (e) => {
            e.preventDefault();
            // Emit event to show the expanding panel with details
            showDetails(result);
        });
    }

    // Auto-dismiss
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(dismissToast, toastDuration);
}

function showErrorToast(message) {
    const container = document.getElementById('toast-container');
    container.innerHTML = '';

    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.innerHTML = `
    <div class="toast-icon error">!</div>
    <div class="toast-body">
      <div class="toast-title">Error</div>
      <div class="toast-message">${escapeHtml(message)}</div>
    </div>
    <button class="toast-close" id="toast-close">✕</button>
  `;

    container.appendChild(toast);

    document.getElementById('toast-close').addEventListener('click', dismissToast);

    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(dismissToast, toastDuration + 2000);
}

function dismissToast() {
    const container = document.getElementById('toast-container');
    const toast = container.querySelector('.toast');
    if (toast) {
        toast.classList.add('hiding');
        setTimeout(() => {
            container.innerHTML = '';
        }, 200);
    }
    if (toastTimeout) {
        clearTimeout(toastTimeout);
        toastTimeout = null;
    }
}

async function showDetails(result) {
    // Trigger the panel to show correction details
    try {
        await invoke('show_panel_at_cursor', {
            correctionsJson: JSON.stringify(result),
        });
    } catch (e) {
        console.error('Failed to show details panel:', e);
    }
    dismissToast();
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
// Load Settings for Duration
// ===========================
async function loadToastDuration() {
    try {
        const settings = await invoke('get_settings');
        toastDuration = (settings.toast_duration || 3) * 1000;
        applyTheme(); // also apply theme from settings
    } catch (e) {
        console.error('Failed to load settings:', e);
    }
}

// ===========================
// Event Listeners
// ===========================
async function setupListeners() {
    // Listen for correction results
    await listen('correction-result', (event) => {
        showToast(event.payload);
    });

    // Listen for errors
    await listen('correction-error', (event) => {
        showErrorToast(event.payload);
    });
}

// ===========================
// Init
// ===========================
applyTheme();
document.addEventListener('DOMContentLoaded', async () => {
    await loadToastDuration();
    await setupListeners();
});
