(function () {
    'use strict';

    if (window.__rendererInitialized) return;
    window.__rendererInitialized = true;

    // Check for Tauri API
    const isTauri = window.__TAURI__ !== undefined;
    
    const authStatus = document.getElementById('auth-status');
    const loginBtn = document.getElementById('login-btn');
    const loginSection = document.getElementById('login-section');
    const streamSection = document.getElementById('stream-section');
    const resultSection = document.getElementById('result-section');
    const startBtn = document.getElementById('start-btn');
    const stopBtn = document.getElementById('stop-btn');
    const gameSearch = document.getElementById('game_search');
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
        console.log(`[Renderer] ${msg}`);
    }

    // Check existing login state on page load
    async function checkExistingLoginState() {
        if (!isTauri) return;
        
        log('Checking existing login state...', 'info');
        
        try {
            const state = await invoke('check_tiktok_login_state');
            
            if (state && state.success) {
                log(`Existing login state: ${state.state}`, 'info');
                
                switch (state.state) {
                    case 'fully_authenticated':
                        log('Already authenticated! Loading UI...', 'success');
                        updateUI(true);
                        break;
                        
                    case 'streamlabs_auth_pending':
                        log('Pending Streamlabs authentication, completing...', 'info');
                        if (state.auth_code) {
                            // Store auth code and complete authentication
                            const completeResult = await invoke('complete_authentication');
                            if (completeResult && completeResult.success) {
                                log('Authentication complete!', 'success');
                                updateUI(true);
                            }
                        }
                        break;
                        
                    case 'tiktok_logged_in':
                        log('TikTok already logged in, redirecting to Streamlabs...', 'info');
                        // Open login window which will redirect to Streamlabs
                        const result = await invoke('open_tiktok_login_window');
                        if (result && result.success) {
                            log('Login window opened, redirecting to Streamlabs...', 'info');
                            showLoginInstructions();
                        }
                        break;
                        
                    default:
                        log('Not logged in, ready for login', 'info');
                }
            } else {
                log('Not logged in, ready for login', 'info');
            }
        } catch (e) {
            log('Error checking login state: ' + e.message, 'error');
        }
    }
    
    // Check login state on page load
    checkExistingLoginState();

    // Helper function to invoke Tauri commands
    async function invoke(command, args = {}) {
        if (!isTauri) {
            console.warn('Tauri API not available');
            return null;
        }
        try {
            return await window.__TAURI__.core.invoke(command, args);
        } catch (e) {
            console.error(`Error invoking ${command}:`, e);
            throw e;
        }
    }

    // Listen for auth-code-received via Tauri event system
    if (isTauri) {
        // Listen for auth code from the login window
        window.addEventListener('tauri://auth-code-received', async (event) => {
            log('Auth code received from Streamlabs!', 'success');
            log('Completing authentication...', 'info');
            
            // Stop polling since we got the event
            if (authPollInterval) {
                clearInterval(authPollInterval);
                authPollInterval = null;
            }
            
            // Complete the authentication
            const completeResult = await invoke('complete_authentication');
            
            if (completeResult && completeResult.success) {
                log('Authentication complete! Token received.', 'success');
                // Close login window
                await invoke('close_login_window');
                
                // Remove instructions
                const instructions = document.querySelector('.login-instructions');
                if (instructions) {
                    instructions.remove();
                }
                
                // Update main UI
                updateUI(true);
            } else {
                log('Authentication completion failed: ' + (completeResult?.message || 'Unknown error'), 'error');
                
                if (statusEl) {
                    statusEl.innerHTML = '<span class="error-text">Authentication failed. Please try again.</span>';
                }
                
                // Show login button again
                loginBtn.style.display = 'inline-flex';
            }
        });

        // Listen for credentials captured
        window.addEventListener('tauri://credentials-captured', async (event) => {
            const data = event.detail;
            log('Credentials captured event received', 'info');
            
            if (data && data.data && data.data.streamlabs_code) {
                log('Streamlabs authorization code received! Completing authentication...', 'success');
                
                // Stop polling
                if (authPollInterval) {
                    clearInterval(authPollInterval);
                    authPollInterval = null;
                }
                
                // Complete the authentication (exchange code for token)
                const completeResult = await invoke('complete_authentication');
                
                if (completeResult && completeResult.success) {
                    log('Authentication complete! Token received.', 'success');
                    
                    // Update UI
                    if (statusEl) {
                        statusEl.innerHTML = '<span class="success-text">All done! You can now start streaming.</span>';
                    }
                    
                    // Close login window and update UI
                    await invoke('close_login_window');
                    
                    // Remove instructions
                    const instructions = document.querySelector('.login-instructions');
                    if (instructions) {
                        instructions.remove();
                    }
                    
                    // Update main UI
                    updateUI(true);
                } else {
                    log('Authentication completion failed: ' + (completeResult?.message || 'Unknown error'), 'error');
                    
                    if (statusEl) {
                        statusEl.innerHTML = '<span class="error-text">Authentication failed. Please try again.</span>';
                    }
                    
                    // Show login button again
                    loginBtn.style.display = 'inline-flex';
                }
                return;
            }
            
            // Check for TikTok cookies
            if (data && data.data && data.data.cookies && Object.keys(data.data.cookies).length > 0) {
                log('TikTok cookies received!', 'success');
                
                // Save credentials automatically
                const saveResult = await invoke('save_credentials_to_file');
                if (saveResult && saveResult.success) {
                    log('TikTok credentials saved!', 'success');
                }
                return;
            }
        });
        
        log('Auth listeners set up via Tauri events', 'success');
    }
    
    // TikTok Login Button Handler
    loginBtn.addEventListener('click', async () => {
        log('Opening TikTok login window...', 'info');
        
        try {
            const result = await invoke('open_tiktok_login_window');
            if (result && result.success) {
                log('TikTok login window opened!', 'success');
                log('Please complete the login process in the new window.', 'info');
                
                // Show instructions
                showLoginInstructions();
            } else {
                log('Failed to open login window: ' + (result?.message || 'Unknown error'), 'error');
            }
        } catch (e) {
            log('Login error: ' + e.message, 'error');
        }
    });

    function showLoginInstructions() {
        // Remove existing instructions if any
        const existing = document.querySelector('.login-instructions');
        if (existing) existing.remove();
        
        const instructions = document.createElement('div');
        instructions.className = 'login-instructions';
        instructions.innerHTML = `
            <p><strong>Login Instructions:</strong></p>
            <ol>
                <li>Enter your TikTok credentials in the login window</li>
                <li>Complete any required verification (SMS/Email if needed)</li>
                <li>Once logged in, you will be automatically redirected to Streamlabs</li>
                <li>Authorize the app when prompted</li>
                <li>Credentials will be saved automatically</li>
            </ol>
        `;
        loginSection.appendChild(instructions);
        loginBtn.style.display = 'none';
        
        // Start automatic polling for credentials
        startAutoAuth();
    }
    
    let authPollInterval = null;
    
    function startAutoAuth() {
        let attempts = 0;
        const maxAttempts = 120; // 2 minutes max
        
        if (authPollInterval) {
            clearInterval(authPollInterval);
        }
        
        authPollInterval = setInterval(async () => {
            attempts++;
            
            // Update progress bar
            const progress = Math.min((attempts / maxAttempts) * 100, 100);
            const progressFill = document.querySelector('.progress-fill');
            if (progressFill) {
                progressFill.style.width = progress + '%';
            }
            
            try {
                const creds = await invoke('get_captured_credentials');
                
                if (creds && creds.success && creds.data && creds.data.cookies && Object.keys(creds.data.cookies).length > 0) {
                    log('TikTok login detected! Saving credentials...', 'success');
                    
                    // Stop polling
                    if (authPollInterval) {
                        clearInterval(authPollInterval);
                        authPollInterval = null;
                    }
                    
                    // Save credentials automatically
                    const saveResult = await invoke('save_credentials_to_file');
                    if (saveResult && saveResult.success) {
                        log('TikTok credentials saved! Redirecting to Streamlabs...', 'success');
                        
                    }
                    
                    return; // Wait for redirect
                }
                
                // Check if we have auth code (Streamlabs redirect)
                if (creds && creds.success && creds.data && creds.data.auth_code) {
                    log('Streamlabs authorization code received! Completing authentication...', 'success');
                    
                    // Stop polling
                    if (authPollInterval) {
                        clearInterval(authPollInterval);
                        authPollInterval = null;
                    }
                    
                    // Complete the authentication (exchange code for token)
                    const completeResult = await invoke('complete_authentication');
                    
                    if (completeResult && completeResult.success) {
                        log('Authentication complete! Token received.', 'success');
                        
                        // Update UI
                        if (statusEl) {
                            statusEl.innerHTML = '<span class="success-text">All done! You can now start streaming.</span>';
                        }
                        
                        // Close login window and update UI
                        await invoke('close_login_window');
                        
                        // Remove instructions
                        const instructions = document.querySelector('.login-instructions');
                        if (instructions) {
                            instructions.remove();
                        }
                        
                        // Update main UI
                        updateUI(true);
                    } else {
                        log('Authentication completion failed: ' + (completeResult?.message || 'Unknown error'), 'error');
                        
                        if (statusEl) {
                            statusEl.innerHTML = '<span class="error-text">Authentication failed. Please try again.</span>';
                        }
                        
                        // Show login button again
                        loginBtn.style.display = 'inline-flex';
                    }
                    
                    return;
                }
                
            } catch (e) {
                log('Auth poll error: ' + e.message, 'error');
            }
            
            // Timeout
            if (attempts >= maxAttempts) {
                log('Authentication timeout. Please try again.', 'error');
                if (authPollInterval) {
                    clearInterval(authPollInterval);
                    authPollInterval = null;
                }
                
                // Close login window
                await invoke('close_login_window');
                
                // Show login button again
                loginBtn.style.display = 'inline-flex';
                const instructions = document.querySelector('.login-instructions');
                if (instructions) {
                    instructions.remove();
                }
            }
        }, 1000); // Check every second
    }

    async function checkLoginStatus() {
        try {
            const creds = await invoke('get_captured_credentials');
            log('Captured credentials: ' + JSON.stringify(creds, null, 2));
            
            if (creds && creds.success && creds.data && creds.data.cookies && Object.keys(creds.data.cookies).length > 0) {
                log('Login successful! Session cookies captured.', 'success');
                log('Number of cookies: ' + Object.keys(creds.data.cookies).length);
                
                // Save credentials automatically
                const saveResult = await invoke('save_credentials_to_file');
                if (saveResult && saveResult.success) {
                    log('Credentials saved to: ' + saveResult.path, 'success');
                }
                
                updateUI(true);
                
                // Close the login window
                await invoke('close_login_window');
                
                // Remove instructions
                const instructions = document.querySelector('.login-instructions');
                if (instructions) {
                    instructions.remove();
                }
                loginBtn.style.display = 'none';
            } else {
                log('No credentials captured yet. Please complete login first.', 'warning');
                log('Tip: Make sure you are logged in on TikTok before checking status.', 'info');
            }
        } catch (e) {
            log('Error checking login status: ' + e.message, 'error');
        }
    }

    async function updateUI(isAuthorized) {
        if (isAuthorized) {
            authStatus.innerText = 'Authenticated';
            authStatus.className = 'status-badge authorized';
            loginSection.classList.add('hidden');
            streamSection.classList.remove('hidden');

            // Fetch and show categories
            fetchInitialCategories();
        } else {
            authStatus.innerText = 'Not Authenticated';
            authStatus.className = 'status-badge unauthorized';
            loginSection.classList.remove('hidden');
            streamSection.classList.add('hidden');
            resultSection.classList.add('hidden');
            document.getElementById('user-info').classList.add('hidden');
            loginBtn.style.display = 'inline-flex';
        }
    }

    async function fetchInitialCategories() {
        if (!quickCats) return;
        quickCats.innerHTML = '<span class="loading-text">Loading...</span>';
        try {
            const categories = await invoke('stream_search', { query: '' });
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
            fetchInitialCategories();
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
            const results = await invoke('stream_search', { query });
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

        const result = await invoke('stream_start', { title, category });

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
        const success = await invoke('stream_end');
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
        log('Initializing... (Tauri: ' + isTauri + ')');
        if (!isTauri) {
            log('Running in browser mode - Tauri APIs not available', 'warning');
            return;
        }
        
        try {
            const info = await invoke('check_credentials');
            if (info && info.ready) {
                log('Session restored.', 'success');
                updateUI(true);
            } else {
                updateUI(false);
            }
        } catch (e) {
            log('Initial check: ' + e.message);
            updateUI(false);
        }
    })();
})();