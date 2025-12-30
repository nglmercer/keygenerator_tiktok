import * as electron from 'electron';
const { BrowserWindow, session, ipcMain } = electron;
import type { IpcMainEvent } from 'electron';
import fs from 'fs';
import path from 'path';

interface AuthResult {
    success: boolean;
    data?: any;
    error?: any;
    status?: number;
    body?: string;
}

export class StreamlabsAuth {
    private window: electron.BrowserWindow | null = null;
    private authUrl: string;
    private cookiesPath: string;
    private codeVerifier: string;
    private tokenFetchStarted: boolean = false;
    private resolveToken: ((value: string) => void) | null = null;
    private rejectToken: ((reason: any) => void) | null = null;

    constructor(authUrl: string, cookiesPath: string, codeVerifier: string) {
        this.authUrl = authUrl;
        this.cookiesPath = cookiesPath;
        this.codeVerifier = codeVerifier;
    }

    public async findToken(): Promise<any> {
        return new Promise((resolve, reject) => {
            this.resolveToken = resolve;
            this.rejectToken = reject;
            this.createWindow().catch(reject);
        });
    }

    private async createWindow() {
        // In a bundled environment, __dirname might point to the bundle location
        // We'll try to find preload.js in the same dir as the current script
        const preloadPath = path.join(process.cwd(), 'src/auth/preload.js');
        console.log(`[Electron-Login] Preload path: ${preloadPath}`);

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

        this.setupIPC();
        this.setupLifecycle();

        await this.loadCookies();

        console.log('[Electron-Login] Navigating to TikTok login...');
        await this.window.loadURL('https://www.tiktok.com/login');

        this.checkLoginStatus(this.window.webContents.getURL());
    }

    private setupIPC() {
        const logHandler = (event: IpcMainEvent, message: string) => {
            if (event.sender !== this.window?.webContents) return;
            console.log(`[Renderer]: ${message}`);
            if (message === 'TRIGGER_STREAMLABS_AUTH') {
                this.forceNavigateAuth();
            }
        };

        const resultHandler = (event: IpcMainEvent, result: AuthResult) => {
            if (event.sender !== this.window?.webContents) return;
            this.handleFetchResult(result);
        };

        ipcMain.on('log-console', logHandler);
        ipcMain.on('fetch-result', resultHandler);

        // Cleanup handlers when window is closed
        this.window?.on('closed', () => {
            ipcMain.removeListener('log-console', logHandler);
            ipcMain.removeListener('fetch-result', resultHandler);
        });
    }

    private setupLifecycle() {
        if (!this.window) return;

        this.window.webContents.on('did-navigate', (_: any, url: string) => {
            this.checkLoginStatus(url);
            this.checkSuccess(url);
        });

        this.window.webContents.on('did-navigate-in-page', (_: any, url: string) => {
            this.checkSuccess(url);
        });

        this.window.webContents.on('did-finish-load', () => {
            this.injectManualAuthButton();
        });

        this.window.on('closed', () => {
            this.window = null;
            if (!this.tokenFetchStarted) {
                this.rejectToken?.(new Error('Window closed by user'));
            }
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
                console.log('[Electron-Login] Cookies loaded.');
            } catch (e) {
                console.error('[Electron-Login] Failed to load cookies:', e);
            }
        }
    }

    private async saveCookies() {
        try {
            const cookies = await session.defaultSession.cookies.get({});
            fs.writeFileSync(this.cookiesPath, JSON.stringify(cookies, null, 2));
            console.log('[Electron-Login] Cookies saved.');
        } catch (e) {
            console.error('[Electron-Login] Failed to save cookies:', e);
        }
    }

    private checkLoginStatus(url: string) {
        if ((url.includes('tiktok.com') && !url.includes('login') && !url.includes('streamlabs')) || url.includes('/foryou')) {
            console.log('[Electron-Login] Login detected. Preparing to navigate to Streamlabs Auth...');

            setTimeout(() => {
                const current = this.window?.webContents.getURL();
                if (current && !current.includes('streamlabs')) {
                    this.forceNavigateAuth();
                }
            }, 2000);
        }
    }

    private forceNavigateAuth() {
        console.log(`[Electron-Login] Navigating to Auth URL: ${this.authUrl}`);
        this.window?.loadURL(this.authUrl).catch(e => console.error('Failed to load Auth URL:', e));
    }

    private checkSuccess(url: string) {
        let code: string | null = null;
        try {
            const urlObj = new URL(url);
            code = urlObj.searchParams.get('code');
        } catch (e) {
            return;
        }

        const isSuccess = (url.includes('success=true') && code) ||
            (url.includes('streamlabs.com/dashboard') && code) ||
            (url.includes('streamlabs.com/slobs/dashboard') && code);

        if (isSuccess && !this.tokenFetchStarted && code) {
            this.tokenFetchStarted = true;
            console.log('[Electron-Login] Success URL detected:', url);
            console.log('[Electron-Login] Authorization code extracted:', code);
            console.log('[Electron-Login] Starting token fetch...');

            this.saveCookies().then(() => {
                this.executeTokenFetch(code!);
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

    private async executeTokenFetch(code: string) {
        if (!this.codeVerifier) {
            console.error('[Electron-Login] No CodeVerifier found!');
            this.rejectToken?.(new Error('No CodeVerifier found'));
            return;
        }

        console.log('[Electron-Login] Fetching token from browser context...');

        const fetchCode = `
        (async () => {
            try {
                const res = await fetch('https://streamlabs.com/api/v5/slobs/auth/data?code=${code}&code_verifier=${this.codeVerifier}', {
                    method: 'GET',
                    credentials: 'include',
                    headers: { 
                        'Accept': 'application/json', 
                        'X-Requested-With': 'XMLHttpRequest' 
                    }
                });
                const text = await res.text();
                try {
                    const json = JSON.parse(text);
                    return { success: true, data: json, status: res.status };
                } catch(e) {
                    return { success: false, error: 'JSON Parse Error', body: text, status: res.status };
                }
            } catch (err) {
                return { success: false, error: err.toString() };
            }
        })()
        `;

        try {
            const result = await this.window?.webContents.executeJavaScript(fetchCode);
            this.handleFetchResult(result);
        } catch (err: any) {
            console.error('[Electron-Login] executeJavaScript error:', err.message);
            this.handleFetchResult({ success: false, error: err.message });
        }
    }

    private handleFetchResult(result: AuthResult) {
        console.log('[Electron-Login] Token fetch result:', JSON.stringify(result));

        if (result.success && result.data?.success) {
            const authData = result.data.data;
            console.log('[Electron-Login] Auth data received successfully');
            this.resolveToken?.(authData);
            this.cleanup();
        } else {
            console.error('[Electron-Login] Error in fetch result:', JSON.stringify(result));
            this.rejectToken?.(new Error(`Fetch failed: ${JSON.stringify(result)}`));
            this.cleanup();
        }
    }

    private async cleanup() {
        await this.saveCookies();
        if (this.window) {
            this.window.close();
            this.window = null;
        }
    }
}
