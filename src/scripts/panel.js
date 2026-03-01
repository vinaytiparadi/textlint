// GrammarLens Correction Panel
// Handles all feedback: corrections, "no changes", and errors
// Data injected via window.showCorrections() / window.showInfo() / window.showError() from Rust

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

let currentResult = null;
let autoDismissTimer = null;

// ===========================
// Theme
// ===========================
function applyTheme() {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
}

// ===========================
// Show corrections — called by Rust via webview.eval()
// ===========================
window.showCorrections = function (result, learnMode = true) {
    currentResult = result;
    clearAutoDismiss();

    document.getElementById('panel-corrections').classList.remove('hidden');
    document.getElementById('panel-info').classList.add('hidden');

    const count = result.num_corrections || (result.corrections ? result.corrections.length : 0);
    document.getElementById('panel-title-text').textContent =
        count > 0 ? `${count} correction${count !== 1 ? 's' : ''}` : 'Corrections';

    document.getElementById('original-text').textContent = result.original_text || '';
    document.getElementById('corrected-text').textContent = result.corrected_text || '';

    // Actions row
    document.getElementById('panel-actions').classList.remove('hidden');

    const list = document.getElementById('corrections-list');
    list.innerHTML = '';

    if (result.corrections && result.corrections.length > 0) {
        result.corrections.forEach((c) => {
            const item = document.createElement('div');
            item.className = 'correction-item';
            const sc = c.severity === 'error' ? 'badge-error' : c.severity === 'warning' ? 'badge-warning' : 'badge-suggestion';
            item.innerHTML = `
        <div class="correction-top">
          <span class="badge ${sc}">${cap(c.severity)}</span>
          <span class="text-small text-secondary">${cap(c.category || 'grammar')}</span>
        </div>
        <div class="correction-change">
          <span class="diff-removed">${esc(c.original)}</span>
          <span class="correction-arrow">→</span>
          <span class="diff-added">${esc(c.corrected)}</span>
        </div>
        ${(learnMode && c.explanation) ? `<div class="correction-explanation">${esc(c.explanation)}</div>` : ''}
      `;
            list.appendChild(item);
        });
    }
};

// ===========================
// Show info ("Looks good!") — called by Rust via webview.eval()
// ===========================
window.showInfo = function (message, subtitle) {
    currentResult = null;
    clearAutoDismiss();

    document.getElementById('panel-corrections').classList.add('hidden');
    document.getElementById('panel-info').classList.remove('hidden');
    document.getElementById('panel-actions').classList.add('hidden');

    document.getElementById('panel-title-text').textContent = 'GrammarLens';
    document.getElementById('info-icon').textContent = '✓';
    document.getElementById('info-icon').className = 'info-icon success';
    document.getElementById('info-message').textContent = message;
    document.getElementById('info-subtitle').textContent = subtitle || '';

    // Auto-dismiss after 2.5 seconds
    autoDismissTimer = setTimeout(closePanel, 2500);
};

// ===========================
// Show error — called by Rust via webview.eval()
// ===========================
window.showError = function (message) {
    currentResult = null;
    clearAutoDismiss();

    document.getElementById('panel-corrections').classList.add('hidden');
    document.getElementById('panel-info').classList.remove('hidden');
    document.getElementById('panel-actions').classList.add('hidden');

    document.getElementById('panel-title-text').textContent = 'Error';
    document.getElementById('info-icon').textContent = '!';
    document.getElementById('info-icon').className = 'info-icon error';
    document.getElementById('info-message').textContent = message;
    document.getElementById('info-subtitle').textContent = '';

    // Auto-dismiss after 4 seconds
    autoDismissTimer = setTimeout(closePanel, 4000);
};

function clearAutoDismiss() {
    if (autoDismissTimer) {
        clearTimeout(autoDismissTimer);
        autoDismissTimer = null;
    }
}

// ===========================
// Actions
// ===========================
async function applyFix() {
    if (!currentResult) return;
    try {
        await invoke('apply_correction_text', { text: currentResult.corrected_text });
    } catch (e) {
        console.error('Apply fix error:', e);
    }
    closePanel();
}

async function copyText() {
    if (!currentResult) return;
    try {
        await navigator.clipboard.writeText(currentResult.corrected_text);
        const btn = document.getElementById('copy-btn');
        btn.textContent = '✓ Copied';
        setTimeout(() => { btn.textContent = 'Copy'; }, 1000);
    } catch (e) {
        console.error('Copy error:', e);
    }
}

function closePanel() {
    clearAutoDismiss();
    currentResult = null;
    try {
        getCurrentWindow().hide();
    } catch (e) {
        console.error('Hide error:', e);
    }
}

// Drag-to-move is handled natively via data-tauri-drag-region on the header element

// ===========================
// Utilities
// ===========================
function esc(t) {
    if (!t) return '';
    const d = document.createElement('div');
    d.textContent = t;
    return d.innerHTML;
}
function cap(s) { return s ? s.charAt(0).toUpperCase() + s.slice(1) : ''; }

// ===========================
// Init
// ===========================
applyTheme();
document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('apply-btn').addEventListener('click', applyFix);
    document.getElementById('copy-btn').addEventListener('click', copyText);
    document.getElementById('dismiss-btn').addEventListener('click', closePanel);
    document.getElementById('panel-close').addEventListener('click', closePanel);
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') closePanel(); });
});
