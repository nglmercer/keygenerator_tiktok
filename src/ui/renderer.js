const { electronAPI } = window;

const authStatus = document.getElementById('auth-status');
const loginBtn = document.getElementById('login-btn');
const loginSection = document.getElementById('login-section');
const streamSection = document.getElementById('stream-section');
const resultSection = document.getElementById('result-section');
const startBtn = document.getElementById('start-btn');
const stopBtn = document.getElementById('stop-btn');
const gameSearch = document.getElementById('game-search');
const searchResults = document.getElementById('search-results');
const streamTitle = document.getElementById('stream-title');
const logConsole = document.getElementById('log-console');

const rtmpUrlInput = document.getElementById('rtmp-url');
const streamKeyInput = document.getElementById('stream-key');

let selectedCategory = '';

function log(msg, type = '') {
    const entry = document.createElement('div');
    entry.className = `log-entry ${type}`;
    entry.innerText = `[${new Date().toLocaleTimeString()}] ${msg}`;
    logConsole.appendChild(entry);
    logConsole.scrollTop = logConsole.scrollHeight;
}

loginBtn.addEventListener('click', async () => {
    log('Starting login flow...', 'info');
    const result = await electronAPI.invoke('auth:login');
    if (result.success) {
        log('Login successful!', 'success');
        updateUI(true);
    } else {
        log('Login failed: ' + result.error, 'error');
    }
});

function updateUI(isAuthorized) {
    if (isAuthorized) {
        authStatus.innerText = 'Authenticated';
        authStatus.className = 'status-badge authorized';
        loginSection.classList.add('hidden');
        streamSection.classList.remove('hidden');
    } else {
        authStatus.innerText = 'Not Authenticated';
        authStatus.className = 'status-badge unauthorized';
        loginSection.classList.remove('hidden');
        streamSection.classList.add('hidden');
        resultSection.classList.add('hidden');
    }
}

// Search Logic
let searchTimeout;
gameSearch.addEventListener('input', () => {
    clearTimeout(searchTimeout);
    const query = gameSearch.value.trim();
    if (query.length < 2) {
        searchResults.classList.add('hidden');
        return;
    }

    searchTimeout = setTimeout(async () => {
        log(`Searching for: ${query}...`);
        const results = await electronAPI.invoke('stream:search', query);
        displaySearchResults(results);
    }, 500);
});

function displaySearchResults(results) {
    searchResults.innerHTML = '';
    if (!results || results.length === 0) {
        searchResults.classList.add('hidden');
        return;
    }

    results.forEach(res => {
        const item = document.createElement('div');
        item.className = 'result-item';
        item.innerText = res.full_name;
        item.addEventListener('click', () => {
            gameSearch.value = res.full_name;
            selectedCategory = res.game_mask_id;
            searchResults.classList.add('hidden');
            log(`Selected category: ${res.full_name}`);
        });
        searchResults.appendChild(item);
    });
    searchResults.classList.remove('hidden');
}

startBtn.addEventListener('click', async () => {
    if (!selectedCategory && gameSearch.value !== 'Other') {
        log('Please select a category first', 'error');
        return;
    }

    const title = streamTitle.value || 'TikTok Live Stream';
    const category = selectedCategory || '';

    log('Starting stream...', 'info');
    startBtn.disabled = true;

    const result = await electronAPI.invoke('stream:start', { title, category });

    if (result && result.rtmpUrl) {
        log('Stream started successfully!', 'success');
        rtmpUrlInput.value = result.rtmpUrl;
        streamKeyInput.value = result.streamKey;
        resultSection.classList.remove('hidden');
        startBtn.classList.add('hidden');
        stopBtn.classList.remove('hidden');
    } else {
        log('Failed to start stream', 'error');
    }
    startBtn.disabled = false;
});

stopBtn.addEventListener('click', async () => {
    log('Stopping stream...', 'info');
    const success = await electronAPI.invoke('stream:end');
    if (success) {
        log('Stream stopped.', 'success');
        startBtn.classList.remove('hidden');
        stopBtn.classList.add('hidden');
        resultSection.classList.add('hidden');
    } else {
        log('Failed to stop stream', 'error');
    }
});

// Copy and Toggle logic
document.querySelectorAll('.copy-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        const targetId = btn.getAttribute('data-target');
        const input = document.getElementById(targetId);
        input.select();
        document.execCommand('copy');
        const originalText = btn.innerText;
        btn.innerText = 'Copied!';
        setTimeout(() => btn.innerText = originalText, 2000);
    });
});

document.querySelectorAll('.toggle-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        const targetId = btn.getAttribute('data-target');
        const input = document.getElementById(targetId);
        if (input.type === 'password') {
            input.type = 'text';
            btn.innerText = 'Hide';
        } else {
            input.type = 'password';
            btn.innerText = 'Show';
        }
    });
});

// Check initial status
(async () => {
    const info = await electronAPI.invoke('stream:info');
    if (info) {
        log('Already authenticated with TikTok.', 'success');
        updateUI(true);
    }
})();
