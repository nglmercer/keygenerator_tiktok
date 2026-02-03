import * as electron from 'electron';
const { BrowserWindow, session, ipcMain } = electron;
import type { IpcMainEvent } from 'electron';
import fs from 'fs';
import path from 'path';
import { 
    IPC_CHANNELS, 
    WINDOW_CONFIG, 
    PATHS, 
    API_ENDPOINTS, 
    ERROR_MESSAGES, 
    CONSOLE_MESSAGES 
} from '../constants.ts';
import { createIpcListener, removeIpcListener } from '../utils/ipcHandler.ts';

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
        const preloadPath = path.join(process.cwd(), PATHS.PRELOAD_AUTH);
        console.log(CONSOLE_MESSAGES.ELECTRON_PRELOAD(preloadPath));

        this.window = new BrowserWindow({
            ...WINDOW_CONFIG.AUTH,
            show: true,
            webPreferences: {
                nodeIntegration: false,
                contextIsolation: true,
                sandbox: false,
                preload: preloadPath,
            },
        });

        this.setupIPC();
        this.setupLifecycle();

        await this.loadCookies();

        console.log(CONSOLE_MESSAGES.ELECTRON_NAVIGATE);
        await this.window.loadURL(API_ENDPOINTS.TIKTOK_LOGIN);

        this.checkLoginStatus(this.window.webContents.getURL());
    }

    private setupIPC() {
        const logHandler = (event: IpcMainEvent, message: string) => {
            if (event.sender !== this.window?.webContents) return;
            console.log(`[Renderer]: ${message}`);
            if (message === IPC_CHANNELS.TRIGGER_STREAMLABS_AUTH) {
                this.forceNavigateAuth();
            }
        };

        const resultHandler = (event: IpcMainEvent, result: AuthResult) => {
            if (event.sender !== this.window?.webContents) return;
            this.handleFetchResult(result);
        };

        createIpcListener(IPC_CHANNELS.LOG_CONSOLE, logHandler);
        createIpcListener(IPC_CHANNELS.FETCH_RESULT, resultHandler);

        this.window?.on('closed', () => {
            removeIpcListener(IPC_CHANNELS.LOG_CONSOLE, logHandler);
            removeIpcListener(IPC_CHANNELS.FETCH_RESULT, resultHandler);
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
                this.rejectToken?.(new Error(ERROR_MESSAGES.WINDOW_CLOSED));
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
                console.log(CONSOLE_MESSAGES.ELECTRON_COOKIES_LOADED);
            } catch (e) {
                console.error(CONSOLE_MESSAGES.ELECTRON_COOKIES_SAVE_ERROR, e);
            }
        }
    }

    private async saveCookies() {
        try {
            const cookies = await session.defaultSession.cookies.get({});
            fs.writeFileSync(this.cookiesPath, JSON.stringify(cookies, null, 2));
            console.log('[Electron-Login] Cookies saved.');
        } catch (e) {
            console.error(CONSOLE_MESSAGES.ELECTRON_COOKIES_SAVE_ERROR, e);
        }
    }

    private checkLoginStatus(url: string) {
        if ((url.includes('tiktok.com') && !url.includes('login') && !url.includes('streamlabs')) || url.includes('/foryou')) {
            console.log(CONSOLE_MESSAGES.ELECTRON_LOGIN_DETECTED);

            setTimeout(() => {
                const current = this.window?.webContents.getURL();
                if (current && !current.includes('streamlabs')) {
                    this.forceNavigateAuth();
                }
            }, 2000);
        }
    }

    private forceNavigateAuth() {
        console.log(CONSOLE_MESSAGES.ELECTRON_FORCE_NAVIGATE(this.authUrl));
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
            (url.includes(API_ENDPOINTS.DASHBOARD) && code) ||
            (url.includes(API_ENDPOINTS.SLOBS_DASHBOARD) && code);

        if (isSuccess && !this.tokenFetchStarted && code) {
            this.tokenFetchStarted = true;
            console.log(CONSOLE_MESSAGES.ELECTRON_SUCCESS(url));
            console.log(CONSOLE_MESSAGES.ELECTRON_CODE(code));
            console.log(CONSOLE_MESSAGES.ELECTRON_FETCH_START);

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
                window.electronAPI.log('${IPC_CHANNELS.TRIGGER_STREAMLABS_AUTH}');
            };
            document.body.appendChild(btn);
        })();
        `;
        this.window?.webContents.executeJavaScript(script).catch(() => { });
    }

    private async executeTokenFetch(code: string) {
        if (!this.codeVerifier) {
            console.error(CONSOLE_MESSAGES.ELECTRON_NO_VERIFIER);
            this.rejectToken?.(new Error(ERROR_MESSAGES.NO_CODE_VERIFIER));
            return;
        }

        console.log(CONSOLE_MESSAGES.ELECTRON_FETCHING);

        const fetchCode = `
        (async () => {
            try {
                const res = await fetch('${API_ENDPOINTS.AUTH_DATA}?code=${code}&code_verifier=${this.codeVerifier}', {
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
                    return { success: false, error: '${ERROR_MESSAGES.JSON_PARSE_ERROR}', body: text, status: res.status };
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
            console.error(CONSOLE_MESSAGES.ELECTRON_JS_ERROR(err.message));
            this.handleFetchResult({ success: false, error: err.message });
        }
    }

    private handleFetchResult(result: AuthResult) {
        console.log(CONSOLE_MESSAGES.ELECTRON_RESULT(JSON.stringify(result)));

        if (result.success && result.data?.success) {
            const authData = result.data.data;
            console.log(CONSOLE_MESSAGES.ELECTRON_AUTH_SUCCESS);
            this.resolveToken?.(authData);
            this.cleanup();
        } else {
            console.error(CONSOLE_MESSAGES.ELECTRON_ERROR_RESULT(JSON.stringify(result)));
            this.rejectToken?.(new Error(`${ERROR_MESSAGES.FETCH_FAILED}: ${JSON.stringify(result)}`));
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
