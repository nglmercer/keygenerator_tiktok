(function () {
    'use strict';

    if (window.__rendererInitialized) return;
    window.__rendererInitialized = true;

    const electronAPI = window.electronAPI;

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
    const quickCats = document.getElementById('quick-categories');

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

    async function updateUI(isAuthorized) {
        if (isAuthorized) {
            authStatus.innerText = 'Authenticated';
            authStatus.className = 'status-badge authorized';
            loginSection.classList.add('hidden');
            streamSection.classList.remove('hidden');

            // Fetch and show user info
            const profile = await electronAPI.invoke('user:profile');
            if (profile) {
                document.getElementById('user-info').classList.remove('hidden');
                document.getElementById('user-avatar').src = profile.avatar_thumb || profile.avatar_url || 'https://www.tiktok.com/favicon.ico';
                document.getElementById('user-name').innerText = profile.display_name || profile.username || 'User';
            }

            // Fetch and show categories
            fetchInitialCategories();
        } else {
            authStatus.innerText = 'Not Authenticated';
            authStatus.className = 'status-badge unauthorized';
            loginSection.classList.remove('hidden');
            streamSection.classList.add('hidden');
            resultSection.classList.add('hidden');
            document.getElementById('user-info').classList.add('hidden');
        }
    }

    async function fetchInitialCategories() {
        if (!quickCats) return;
        quickCats.innerHTML = '<span class="loading-text">Loading...</span>';
        try {
            // We use 'stream:search' with empty string which now triggers getInitialCategories in API
            const categories = await electronAPI.invoke('stream:search', '');
            if (categories && categories.length > 0) {
                renderCategoryPills(categories);
            } else {
                quickCats.innerHTML = '<span class="error-text">No categories found</span>';
            }
        } catch (e) {
            quickCats.innerHTML = '<span class="error-text">Error loading categories</span>';
        }
    }

    function renderCategoryPills(categories) {
        quickCats.innerHTML = '';
        categories.slice(0, 15).forEach(cat => {
            const pill = document.createElement('span');
            pill.className = 'category-pill';
            pill.innerText = cat.full_name;
            pill.addEventListener('click', () => {
                gameSearch.value = cat.full_name;
                selectedCategory = cat.game_mask_id;
                log(`Selected: ${cat.full_name}`);

                document.querySelectorAll('.category-pill').forEach(p => p.classList.remove('active'));
                pill.classList.add('active');
            });
            quickCats.appendChild(pill);
        });
    }

    // Search Logic
    let searchTimeout;
    gameSearch.addEventListener('input', () => {
        clearTimeout(searchTimeout);
        const query = gameSearch.value.trim();
        if (query.length === 0) {
            fetchInitialCategories(); // Show defaults again if cleared
            searchResults.classList.add('hidden');
            return;
        }
        if (query.length < 2) {
            searchResults.classList.add('hidden');
            return;
        }

        searchTimeout = setTimeout(async () => {
            searchResults.innerHTML = '<div class="result-item" style="color: var(--text-secondary); font-style: italic;">Searching...</div>';
            searchResults.classList.remove('hidden');

            log(`Searching categories for: "${query}"...`);
            const results = await electronAPI.invoke('stream:search', query);
            displaySearchResults(results, query);
        }, 500);
    });

    function displaySearchResults(results, query) {
        searchResults.innerHTML = '';
        if (!results || results.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'result-item';
            empty.style.color = 'var(--text-secondary)';
            empty.style.fontSize = '0.85rem';
            empty.innerHTML = `No direct matches for "${query}"<br><small>Try a shorter name or broad term.</small>`;
            searchResults.appendChild(empty);
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

                document.querySelectorAll('.category-pill').forEach(p => p.classList.remove('active'));
            });
            searchResults.appendChild(item);
        });
        searchResults.classList.remove('hidden');
    }

    // Close results when clicking outside
    document.addEventListener('click', (e) => {
        if (!gameSearch.contains(e.target) && !searchResults.contains(e.target)) {
            searchResults.classList.add('hidden');
        }
    });

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
            log('Failed to start stream. Check if you have Live Access.', 'error');
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
        try {
            const info = await electronAPI.invoke('stream:info');
            if (info) {
                log('Session restored.', 'success');
                updateUI(true);
            }
        } catch (e) {
            updateUI(false);
        }
    })();
})();