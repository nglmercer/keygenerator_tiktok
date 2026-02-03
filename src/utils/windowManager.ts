import { BrowserWindow, ipcMain, app } from 'electron';
import type { IpcMainEvent } from 'electron';
import path from 'path';
import { resolveAppPath } from './fileUtils';
import { 
    WINDOW_CONFIG, 
    PATHS, 
    WEB_PREFERENCES,
    IPC_CHANNELS 
} from '../constants';
import type { IpcHandlerFn } from './ipcHandler';

/**
 * Window lifecycle manager to avoid repeated window creation code
 */
export class WindowManager {
    private window: BrowserWindow | null = null;
    private preloadPath: string;
    private title: string;
    private width: number;
    private height: number;
    private backgroundColor?: string;

    constructor(options: {
        title: string;
        width: number;
        height: number;
        backgroundColor?: string;
        preloadPath: string;
    }) {
        this.title = options.title;
        this.width = options.width;
        this.height = options.height;
        this.backgroundColor = options.backgroundColor;
        this.preloadPath = options.preloadPath;
    }

    create(): BrowserWindow {
        this.window = new BrowserWindow({
            title: this.title,
            width: this.width,
            height: this.height,
            backgroundColor: this.backgroundColor,
            webPreferences: {
                ...WEB_PREFERENCES,
                preload: this.preloadPath,
            },
        });

        this.window.on('closed', () => {
            this.window = null;
        });

        return this.window;
    }

    loadFile(filePath: string): void {
        this.window?.loadFile(filePath);
    }

    loadUrl(url: string): Promise<void> {
        return this.window?.loadURL(url) ?? Promise.reject('Window not created');
    }

    getUrl(): string | undefined {
        return this.window?.webContents.getURL();
    }

    close(): void {
        this.window?.close();
        this.window = null;
    }

    getWindow(): BrowserWindow | null {
        return this.window;
    }

    exists(): boolean {
        return this.window !== null;
    }

    injectScript(script: string): Promise<void> {
        return this.window?.webContents.executeJavaScript(script).then(() => {}) 
            ?? Promise.reject('Window not created');
    }
}

/**
 * Auth window manager with IPC setup
 */
export class AuthWindowManager extends WindowManager {
    private codeVerifier: string;
    private onAuthComplete?: (result: any) => void;
    private onAuthError?: (error: any) => void;

    constructor(codeVerifier: string) {
        super({
            title: WINDOW_CONFIG.AUTH.title,
            width: WINDOW_CONFIG.AUTH.width,
            height: WINDOW_CONFIG.AUTH.height,
            preloadPath: resolveAppPath(PATHS.PRELOAD_AUTH),
        });
        this.codeVerifier = codeVerifier;
    }

    setupAuthHandlers(
        authUrl: string,
        onComplete: (result: any) => void,
        onError: (error: any) => void
    ): void {
        this.onAuthComplete = onComplete;
        this.onAuthError = onError;

        const logHandler = (event: IpcMainEvent, message: string) => {
            if (event.sender !== this.getWindow()?.webContents) return;
            if (message === IPC_CHANNELS.TRIGGER_STREAMLABS_AUTH) {
                this.forceNavigate(authUrl);
            }
        };

        const resultHandler = (event: IpcMainEvent, result: any) => {
            if (event.sender !== this.getWindow()?.webContents) return;
            this.handleResult(result);
        };

        ipcMain.on(IPC_CHANNELS.LOG_CONSOLE, logHandler);
        ipcMain.on(IPC_CHANNELS.FETCH_RESULT, resultHandler);

        this.getWindow()?.on('closed', () => {
            ipcMain.removeListener(IPC_CHANNELS.LOG_CONSOLE, logHandler);
            ipcMain.removeListener(IPC_CHANNELS.FETCH_RESULT, resultHandler);
        });
    }

    private handleResult(result: any): void {
        if (result.success && result.data?.success) {
            this.onAuthComplete?.(result.data.data);
        } else {
            this.onAuthError?.(result);
        }
        this.close();
    }

    private forceNavigate(url: string): void {
        this.loadUrl(url).catch(console.error);
    }

    injectAuthButton(): void {
        const script = `
            (function() {
                if (document.getElementById('sl-auth-btn') || !window.location.href.includes('tiktok.com')) return;
                const btn = document.createElement('button');
                btn.id = 'sl-auth-btn';
                btn.innerText = 'Start Streamlabs Auth';
                btn.style.cssText = 'position:fixed;top:10px;right:10px;z-index:99999;padding:12px 20px;background:#00f2ea;color:#000;font-weight:bold;border:none;border-radius:5px;cursor:pointer;';
                btn.onclick = function() { window.electronAPI.log('${IPC_CHANNELS.TRIGGER_STREAMLABS_AUTH}'); };
                document.body.appendChild(btn);
            })();
        `;
        this.injectScript(script).catch(() => {});
    }
}

/**
 * Main window manager for the application
 */
export class MainWindowManager extends WindowManager {
    constructor() {
        super({
            title: WINDOW_CONFIG.MAIN.title,
            width: WINDOW_CONFIG.MAIN.width,
            height: WINDOW_CONFIG.MAIN.height,
            backgroundColor: WINDOW_CONFIG.MAIN.backgroundColor,
            preloadPath: resolveAppPath(PATHS.PRELOAD),
        });
    }

    load(): void {
        const indexPath = resolveAppPath(PATHS.INDEX_HTML);
        if (require('fs').existsSync(indexPath)) {
            this.loadFile(indexPath);
        } else {
            console.error('Could not find index.html at', indexPath);
        }
    }
}

/**
 * Helper to create window with standard IPC handlers
 */
export function createWindowWithHandlers(
    handlers: Array<{ channel: string; handler: IpcHandlerFn }>
): WindowManager {
    handlers.forEach(({ channel, handler }) => {
        ipcMain.handle(channel, async (event, ...args) => {
            try {
                return await handler(...args);
            } catch (error: any) {
                console.error(`[IPC] ${channel}:`, error);
                return { success: false, error: error.message };
            }
        });
    });
    return new MainWindowManager();
}
