import { app, BrowserWindow, session, ipcMain } from 'electron';
import fs from 'fs';
import path from 'path';

// Args passed via process.env
const authUrl = process.env.AUTH_URL || '';
const cookiesPath = process.env.COOKIES_PATH || '';

if (!authUrl || !cookiesPath) {
    console.error('Missing AUTH_URL or COOKIES_PATH env vars');
    app.quit();
    process.exit(1);
}

async function createWindow() {
    const win = new BrowserWindow({
        width: 1280,
        height: 720,
        webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            sandbox: true,
            preload: path.join(__dirname, 'preload.js'), // Use the new preload script
        }
    });

    // Handle IPC messages from renderer
    ipcMain.on('log-console', (event, message) => {
        console.log('[Renderer Log]:', message);
    });

    ipcMain.on('fetch-result', (event, result) => {
        console.log('Received Fetch Result via IPC:', JSON.stringify(result, null, 2));

        if (result && result.success && result.data && result.data.success) {
            if (process.send) {
                process.send({ type: 'token-success', token: result.data.data.oauth_token });
            }
            console.log('TOKEN RETRIEVED SUCCESSFULLY');
        } else if (result.data && result.data.message) {
            console.log('API Error Message:', result.data.message);
        }

        if (!result.data || !result.data.success) {
            if (process.send) {
                process.send({ type: 'error', error: JSON.stringify(result) });
            }
        }

        // Save cookies and cleanup
        saveCookies().then(() => {
            setTimeout(() => { app.quit(); process.exit(0); }, 3000); // 3s wait to read logs
        });
    });

    // Load cookies
    if (fs.existsSync(cookiesPath)) {
        try {
            const cookies = JSON.parse(fs.readFileSync(cookiesPath, 'utf-8'));
            for (const cookie of cookies) {
                const scheme = cookie.secure ? 'https' : 'http';
                const domain = cookie.domain.startsWith('.') ? cookie.domain.substring(1) : cookie.domain;
                const cookieUrl = `${scheme}://${domain}${cookie.path}`;

                await session.defaultSession.cookies.set({
                    url: cookieUrl,
                    name: cookie.name,
                    value: cookie.value,
                    domain: cookie.domain,
                    path: cookie.path,
                    secure: cookie.secure,
                    httpOnly: cookie.httpOnly,
                    expirationDate: cookie.expires
                });
            }
            console.log('Cookies loaded.');
        } catch (e) {
            console.error('Failed to load cookies:', e);
        }
    }

    // Initial navigation
    console.log('Navigating to TikTok...');
    await win.loadURL('https://www.tiktok.com/login');

    // Logic to detect if we are already logged in
    const checkLoginStatus = (url: string) => {
        // If we are on the main page or foryou, we are likely logged in
        // Also avoid triggering on the Streamlabs auth URL itself
        if ((url.includes('tiktok.com') && !url.includes('login') && !url.includes('streamlabs')) || url.includes('/foryou')) {
            console.log('Login detected (URL check). Proceeding to Streamlabs Auth...');
            // Small delay to ensure cookies are stable
            setTimeout(() => {
                // Only load if we haven't already moved to streamlabs
                const currentUrl = win.webContents.getURL();
                if (!currentUrl.includes('streamlabs')) {
                    win.loadURL(authUrl).catch(e => console.error('Failed to load Auth URL:', e));
                }
            }, 2000);
        }
    };

    win.webContents.on('did-navigate', (event, url) => checkLoginStatus(url));
    // check current url immediately after load
    checkLoginStatus(win.webContents.getURL());

    // Inject a floating button to trigger the next step manually if auto-detection fails
    const injectButton = async () => {
        try {
            const url = win.webContents.getURL();
            // Only show on tiktok pages
            if (!url.includes('tiktok.com')) return;

            await win.webContents.executeJavaScript(`
            (function() {
                if (document.getElementById('sl-auth-btn')) return;
                const btn = document.createElement('button');
                btn.id = 'sl-auth-btn';
                btn.innerText = 'Go to Streamlabs (I am Logged In)';
                btn.style.cssText = 'position: fixed; top: 10px; right: 10px; z-index: 99999; padding: 12px 20px; background-color: #00f2ea; color: #000; font-weight: bold; border: none; border-radius: 5px; cursor: pointer; box-shadow: 0 4px 6px rgba(0,0,0,0.1); font-family: sans-serif; font-size: 14px;';
                btn.onclick = function() {
                    window.electronAPI.log('TRIGGER_STREAMLABS_AUTH');
                    window.electronAPI.sendResult('manual-trigger', {}); // Optional
                };
                document.body.appendChild(btn);
            })();
          `);
        } catch (e) {
            // Ignore execution errors
        }
    };

    // Re-inject button on navigation
    win.webContents.on('did-finish-load', () => {
        injectButton();
        checkLoginStatus(win.webContents.getURL());
    });
    setInterval(injectButton, 3000);

    // Listen for manual trigger via IPC (from log message "TRIGGER_STREAMLABS_AUTH")
    // Note: We use the log channel to detect this for simplicity or can use a specific channel
    ipcMain.on('manual-trigger', () => {
        console.log('User manually requested auth flow. Navigating to:', authUrl);
        win.loadURL(authUrl).catch(e => console.error('Failed to load Auth URL:', e));
    });

    // Also listen to log-console for the specific string just in case
    ipcMain.on('log-console', (event, message) => {
        if (message === 'TRIGGER_STREAMLABS_AUTH') {
            console.log('User manually requested auth flow via log. Navigating to:', authUrl);
            win.loadURL(authUrl).catch(e => console.error('Failed to load Auth URL:', e));
        }
    });


    // Flag to prevent multiple fetch attempts
    let tokenFetchStarted = false;

    // Monitor redirects for success
    const checkSuccess = (url: string) => {
        if (url.includes('success=true') && !tokenFetchStarted) {
            tokenFetchStarted = true; // Lock
            console.log('LOGIN_SUCCESS_DETECTED');
            if (process.send) {
                process.send({ type: 'login-success' });
            }

            // Save cookies first
            saveCookies().then(() => {
                // Fetch Token from inside the window to usage session cookies without hassle
                const codeVerifier = process.env.CODE_VERIFIER;
                if (!codeVerifier) {
                    console.error('No CODE_VERIFIER provided!');
                    setTimeout(() => { app.quit(); process.exit(1); }, 1000);
                    return;
                }

                console.log(`Verifying CodeVerifier: ${codeVerifier.substring(0, 5)}...`);

                const fetchScript = `
                (async () => {
                    try {
                        window.electronAPI.log('Starting fetch inside window...');
                        const res = await fetch('https://streamlabs.com/api/v5/slobs/auth/data?code_verifier=${codeVerifier}', {
                            method: 'GET',
                            headers: {
                                'Accept': 'application/json',
                                'X-Requested-With': 'XMLHttpRequest'
                            }
                        });
                        const text = await res.text();
                        window.electronAPI.log('Fetch complete. Status: ' + res.status);
                        try {
                            const json = JSON.parse(text);
                            window.electronAPI.sendResult('fetch-result', { success: true, data: json, status: res.status, bodyPreview: text.substring(0, 200) });
                        } catch (e) {
                            window.electronAPI.sendResult('fetch-result', { success: false, error: 'Invalid JSON', body: text, status: res.status });
                        }
                    } catch (err) {
                        window.electronAPI.sendResult('fetch-result', { success: false, error: err.toString() });
                    }
                })()
             `;

                console.log('Fetching OAuth token via in-page fetch using IPC for result...');
                win.webContents.executeJavaScript(fetchScript)
                    .catch(err => {
                        console.error('Failed to execute fetch in page:', err);
                        setTimeout(() => { app.quit(); process.exit(1); }, 1000);
                    });
            });
        }
    };

    win.webContents.on('did-navigate', (event, url) => checkSuccess(url));
    win.webContents.on('did-navigate-in-page', (event, url) => checkSuccess(url));

    async function saveCookies() {
        try {
            const cookies = await session.defaultSession.cookies.get({});
            fs.writeFileSync(cookiesPath, JSON.stringify(cookies, null, 2));
            console.log('Cookies saved.');
        } catch (e) {
            console.error('Failed to save cookies:', e);
        }
    }

}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
    app.quit();
});
