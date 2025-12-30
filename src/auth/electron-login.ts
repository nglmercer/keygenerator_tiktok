import { app, BrowserWindow, session, ipcMain, type IpcMainEvent } from 'electron';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Recreate __dirname for ESM
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface AuthResult {
    success: boolean;
    data?: any;
    error?: any;
    status?: number;
    body?: string;
}

class StreamlabsAuth {
    private window: BrowserWindow | null = null;
    private authUrl: string;
    private cookiesPath: string;
    private codeVerifier: string;
    private tokenFetchStarted: boolean = false;

    constructor() {
        this.authUrl = process.env.AUTH_URL || '';
        this.cookiesPath = process.env.COOKIES_PATH || '';
        this.codeVerifier = process.env.CODE_VERIFIER || '';

        if (!this.authUrl || !this.cookiesPath) {
            console.error('[Electron] Missing AUTH_URL or COOKIES_PATH env vars');
            app.quit();
            process.exit(1);
        }
    }

    public async init() {
        await app.whenReady();
        await this.createWindow();
        this.setupIPC();
        this.setupLifecycle();
    }

    private async createWindow() {
        const preloadPath = path.join(__dirname, 'preload.js');
        console.log(`[Electron] Preload path: ${preloadPath}`);

        this.window = new BrowserWindow({
            width: 1280,
            height: 800,
            show: true,
            title: 'TikTok Auth - Streamlabs',
            webPreferences: {
                nodeIntegration: false,
                contextIsolation: true,
                sandbox: false,
                preload: preloadPath,
            }
        });

        await this.loadCookies();

        console.log('[Electron] Navigating to TikTok login...');
        await this.window.loadURL('https://www.tiktok.com/login');

        this.checkLoginStatus(this.window.webContents.getURL());
    }

    private setupIPC() {
        ipcMain.on('log-console', (event: IpcMainEvent, message: string) => {
            console.log(`[Renderer]: ${message}`);
            if (message === 'TRIGGER_STREAMLABS_AUTH') {
                this.forceNavigateAuth();
            }
        });

        ipcMain.on('fetch-result', (event: IpcMainEvent, result: AuthResult) => {
            this.handleFetchResult(result);
        });

        ipcMain.on('manual-trigger', () => {
            this.forceNavigateAuth();
        });
    }

    private setupLifecycle() {
        if (!this.window) return;

        this.window.webContents.on('did-navigate', (_, url) => {
            this.checkLoginStatus(url);
            this.checkSuccess(url);
        });

        this.window.webContents.on('did-navigate-in-page', (_, url) => {
            this.checkSuccess(url);
        });

        this.window.webContents.on('did-finish-load', () => {
            this.injectManualAuthButton();
        });

        this.window.on('closed', () => {
            this.window = null;
        });
    }

    private async loadCookies() {
        if (fs.existsSync(this.cookiesPath)) {
            try {
                const cookies = JSON.parse(fs.readFileSync(this.cookiesPath, 'utf-8'));
                const promises = cookies.map((cookie: any) => {
                    const scheme = cookie.secure ? 'https' : 'http';
                    const domain = cookie.domain.startsWith('.') ? cookie.domain.substring(1) : cookie.domain;
                    const url = `${scheme}://${domain}${cookie.path}`;
                    return session.defaultSession.cookies.set({ ...cookie, url });
                });
                await Promise.all(promises);
                console.log('[Electron] Cookies loaded.');
            } catch (e) {
                console.error('[Electron] Failed to load cookies:', e);
            }
        }
    }

    private async saveCookies() {
        try {
            const cookies = await session.defaultSession.cookies.get({});
            fs.writeFileSync(this.cookiesPath, JSON.stringify(cookies, null, 2));
            console.log('[Electron] Cookies saved.');
        } catch (e) {
            console.error('[Electron] Failed to save cookies:', e);
        }
    }

    private checkLoginStatus(url: string) {
        if ((url.includes('tiktok.com') && !url.includes('login') && !url.includes('streamlabs')) || url.includes('/foryou')) {
            console.log('[Electron] Login detected. Preparing to navigate to Streamlabs Auth...');

            setTimeout(() => {
                const current = this.window?.webContents.getURL();
                if (current && !current.includes('streamlabs')) {
                    this.forceNavigateAuth();
                }
            }, 2000);
        }
    }

    private forceNavigateAuth() {
        console.log(`[Electron] Navigating to Auth URL: ${this.authUrl}`);
        this.window?.loadURL(this.authUrl).catch(e => console.error('Failed to load Auth URL:', e));
    }

    private checkSuccess(url: string) {
        if (url.includes('success=true') && !this.tokenFetchStarted) {
            this.tokenFetchStarted = true;
            console.log('[Electron] Success URL detected. Starting token fetch...');

            if (process.send) process.send({ type: 'login-success' });

            this.saveCookies().then(() => {
                this.executeTokenFetch();
            });
        }
    }

    private injectManualAuthButton() {
        const script = `
        (function() {
            if (document.getElementById('sl-auth-btn') || !window.location.href.includes('tiktok.com')) return;
            const btn = document.createElement('button');
            btn.id = 'sl-auth-btn';
            btn.innerText = 'Start Streamlabs Auth';
            btn.style.cssText = 'position:fixed;top:10px;right:10px;z-index:99999;padding:12px 20px;background:#00f2ea;color:#000;font-weight:bold;border:none;border-radius:5px;cursor:pointer;box-shadow:0 4px 6px rgba(0,0,0,0.1);font-family:sans-serif;';
            btn.onclick = function() {
                window.electronAPI.log('TRIGGER_STREAMLABS_AUTH');
            };
            document.body.appendChild(btn);
        })();
        `;
        this.window?.webContents.executeJavaScript(script).catch(() => { });
    }

    private executeTokenFetch() {
        if (!this.codeVerifier) {
            console.error('[Electron] No CodeVerifier found!');
            return;
        }

        const fetchCode = `
        (async () => {
            const waitForApi = () => new Promise((resolve, reject) => {
                let attempts = 0;
                const check = () => {
                    attempts++;
                    if (window.electronAPI) {
                        resolve();
                    } else if (attempts > 50) { // 5 seconds
                        reject(new Error('electronAPI not available after 5s'));
                    } else {
                        setTimeout(check, 100);
                    }
                };
                check();
            });

            try {
                await waitForApi();
                window.electronAPI.log('Fetching token from inside renderer...');
                const res = await fetch('https://streamlabs.com/api/v5/slobs/auth/data?code_verifier=${this.codeVerifier}', {
                    method: 'GET',
                    headers: { 'Accept': 'application/json', 'X-Requested-With': 'XMLHttpRequest' }
                });
                const text = await res.text();
                let json;
                try {
                    json = JSON.parse(text);
                    window.electronAPI.sendResult('fetch-result', { success: true, data: json });
                } catch(e) {
                    window.electronAPI.sendResult('fetch-result', { success: false, error: 'JSON Parse Error', body: text });
                }
            } catch (err) {
                // If electronAPI is missing, we can't use it to log the error, so we console.error (which might not show in parent)
                if (window.electronAPI) {
                    window.electronAPI.sendResult('fetch-result', { success: false, error: err.toString() });
                } else {
                    console.error('CRITICAL: electronAPI missing in renderer', err);
                }
            }
        })()
        `;

        this.window?.webContents.executeJavaScript(fetchCode);
    }

    private handleFetchResult(result: AuthResult) {
        console.log('[Electron] Fetch result received:', result.success ? 'Success' : 'Failed');

        if (result.success && result.data?.success) {
            const token = result.data.data.oauth_token;
            console.log('[Electron] Token extracted successfully.');
            if (process.send) process.send({ type: 'token-success', token });
        } else {
            console.error('[Electron] Error in fetch result:', JSON.stringify(result));
            if (process.send) process.send({ type: 'error', error: JSON.stringify(result) });
        }

        this.cleanup();
    }

    private async cleanup() {
        await this.saveCookies();
        setTimeout(() => {
            console.log('[Electron] Exiting...');
            // app.quit(); // Keep window open for a moment if needed, but usually we exit
            app.quit();
            process.exit(0);
        }, 1000);
    }
}

new StreamlabsAuth().init();
