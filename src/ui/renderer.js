// App State
const state = { authorized: false, category: '', stream: null };
const $ = id => document.getElementById(id);
const api = channel => window.electronAPI.invoke(channel);

// Logger
const log = (() => {
    const el = $('log-console');
    return (msg, type = '') => {
        const div = document.createElement('div');
        div.className = `log-entry ${type}`;
        div.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
        el.appendChild(div);
        el.scrollTop = el.scrollHeight;
    };
})();

// UI Helpers
const show = (el, visible = true) => el.classList.toggle('hidden', !visible);

// Auth Handler
async function handleLogin() {
    log('Login...', 'info');
    const res = await api('auth:login');
    if (res.success) {
        state.authorized = true;
        log('Success!', 'success');
        updateUI();
    } else {
        log(`Error: ${res.error}`, 'error');
    }
}

// Category Search
let searchTimeout;
async function handleSearch() {
    clearTimeout(searchTimeout);
    const query = $('game-search').value.trim();
    const results = $('search-results');

    if (!query) { show(results, false); return loadCategories(); }
    if (query.length < 2) { show(results, false); return; }

    show(results);
    results.innerHTML = '<div style="color:var(--text-secondary)">Searching...</div>';
    log(`Searching: "${query}"...`);

    searchTimeout = setTimeout(async () => {
        const cats = await api('stream:search', query);
        results.innerHTML = cats.length ? cats.map(c => 
            `<div class="result-item">${c.full_name}</div>`
        ).join('') : 'No results';
        
        results.querySelectorAll('.result-item').forEach((el, i) => {
            el.onclick = () => {
                $('game-search').value = cats[i].full_name;
                state.category = cats[i].game_mask_id;
                show(results, false);
            };
        });
    }, 500);
}

// Load Categories
async function loadCategories() {
    const container = $('quick-categories');
    container.innerHTML = 'Loading...';
    const cats = await api('stream:search', '');
    container.innerHTML = cats?.slice(0, 15).map(c => 
        `<span class="category-pill">${c.full_name}</span>`
    ).join('');
    
    container.querySelectorAll('.category-pill').forEach((el, i) => {
        el.onclick = () => {
            $('game-search').value = cats[i].full_name;
            state.category = cats[i].game_mask_id;
            container.querySelectorAll('.category-pill').forEach(p => p.classList.remove('active'));
            el.classList.add('active');
        };
    });
}

// Stream Controls
async function startStream() {
    if (!state.category && $('game-search').value !== 'Other') {
        return log('Select category', 'error');
    }
    $('start-btn').disabled = true;
    log('Starting...', 'info');
    
    const res = await api('stream:start', { 
        title: $('stream-title').value || 'TikTok Live', 
        category: state.category 
    });
    
    $('start-btn').disabled = false;
    if (res?.rtmpUrl) {
        log('Started!', 'success');
        $('rtmp-url').value = res.rtmpUrl;
        $('stream-key').value = res.streamKey;
        show($('result-section'), true);
        show($('start-btn'), false);
        show($('stop-btn'), true);
    } else {
        log('Failed to start', 'error');
    }
}

async function stopStream() {
    log('Stopping...', 'info');
    if (await api('stream:end')) {
        log('Stopped', 'success');
        show($('result-section'), false);
        show($('start-btn'), true);
        show($('stop-btn'), false);
    } else {
        log('Failed to stop', 'error');
    }
}

// Init
function init() {
    $('login-btn').onclick = handleLogin;
    $('game-search').oninput = handleSearch;
    $('start-btn').onclick = startStream;
    $('stop-btn').onclick = stopStream;

    document.querySelectorAll('.copy-btn').forEach(btn => {
        btn.onclick = () => {
            const el = $(btn.dataset.target);
            el.select();
            document.execCommand('copy');
            log('Copied');
        };
    });

    document.querySelectorAll('.toggle-btn').forEach(btn => {
        btn.onclick = () => {
            const el = $(btn.dataset.target);
            el.type = el.type === 'password' ? 'text' : 'password';
            btn.textContent = el.type === 'password' ? 'Show' : 'Hide';
        };
    });

    document.onclick = e => {
        if (!e.target.closest('.search-container')) show($('search-results'), false);
    };

    (async () => {
        try { await api('stream:info'); state.authorized = true; } 
        catch { state.authorized = false; }
        updateUI();
    })();
}

function updateUI() {
    const status = $('auth-status');
    status.textContent = state.authorized ? 'Authenticated' : 'Not Authenticated';
    status.className = `status-badge ${state.authorized ? 'authorized' : 'unauthorized'}`;
    show($('login-section'), !state.authorized);
    show($('stream-section'), state.authorized);
    show($('user-info'), state.authorized);
    
    if (state.authorized) {
        loadCategories();
        api('user:profile').then(p => {
            if (p) {
                $('user-avatar').src = p.avatar_thumb || '';
                $('user-name').textContent = p.display_name || p.username || '';
            }
        });
    }
}

init();
