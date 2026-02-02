import { WindowBuilder, WebViewBuilder, EventLoop, type WebView, type Window } from "webview-napi";
import fs from 'fs';
import path from 'path';
import { IpcMessageRouter } from "../utils/ipc-handler.js";
import { IpcMessageType, type WindowContextPayload, type RawStringPayload } from "../utils/ipc-types.js";
import { safeJsonParse } from "../utils/json.js";

interface AuthResult {
    success: boolean;
    data?: any;
    error?: any;
    status?: number;
    body?: string;
}

interface TikTokData {
    uniqueId?: string;
    roomId?: string;
    payload?: any;
}

export class StreamlabsAuth {
    private window: Window | null = null;
    private webview: WebView | null = null;
    private authUrl: string;
    private cookiesPath: string;
    private codeVerifier: string;
    private tokenFetchStarted: boolean = false;
    private resolveToken: ((value: any) => void) | null = null;
    private rejectToken: ((reason: any) => void) | null = null;
    private ipcRouter: IpcMessageRouter;
    private tiktokData: TikTokData = {};
    private eventLoop: EventLoop;

    constructor(authUrl: string, cookiesPath: string, codeVerifier: string, eventLoop: EventLoop) {
        this.authUrl = authUrl;
        this.cookiesPath = cookiesPath;
        this.codeVerifier = codeVerifier;
        this.eventLoop = eventLoop;
        
        // Setup IPC router for handling messages from webview
        this.ipcRouter = new IpcMessageRouter({
            enableLogging: true,
            autoGenerateId: true,
            autoTimestamp: true,
            onUnhandledMessage: (msg) => {
                console.warn("[Webview-Login] Mensaje IPC no manejado:", msg.type);
            },
            onParseError: (error, raw) => {
                console.error("[Webview-Login] Error parseando mensaje:", error.message);
            },
        });

        this.setupIPCHandlers();
    }

    /**
     * Get the window instance (for external access)
     */
    public getWindow(): Window | null {
        return this.window;
    }

    private setupIPCHandlers() {
        this.ipcRouter
            .on(IpcMessageType.WINDOW_CONTEXT, (payload: WindowContextPayload) => {
                console.log("[Webview-Login] Contexto de ventana recibido:");
                console.log("  - URL:", payload.url.full);
                
                // Check login status and success URL
                this.checkLoginStatus(payload.url.full);
                this.checkSuccess(payload.url.full);
            })
            .on(IpcMessageType.RAW_STRING, (payload: RawStringPayload) => {
                // Handle raw messages including TikTok WebSocket data
                if (payload.data.includes("setUniqueId")) {
                    console.log("[Webview-Login] 🎯 TikTok payload detectado!");
                    this.handleTikTokPayload(payload.data);
                }
            })
            .on(IpcMessageType.LOG_EVENT, (payload) => {
                const level = payload.level;
                const logFn = level === 'error' ? console.error : level === 'warn' ? console.warn : console.log;
                logFn(`[Webview-Login] [${level.toUpperCase()}] ${payload.message}`);
            });
    }

    private handleTikTokPayload(data: string) {
        const result = safeJsonParse<any>(data);
        if (result.success) {
            this.tiktokData.payload = result.data;
            
            // Extract uniqueId and roomId if present
            if (result.data && typeof result.data === 'object') {
                if ('uniqueId' in result.data) {
                    this.tiktokData.uniqueId = result.data.uniqueId;
                }
                if ('roomId' in result.data) {
                    this.tiktokData.roomId = result.data.roomId;
                }
            }
            
            console.log("[Webview-Login] TikTok data extraído:", {
                uniqueId: this.tiktokData.uniqueId,
                roomId: this.tiktokData.roomId
            });
        }
    }

    public async findToken(): Promise<any> {
        return new Promise((resolve, reject) => {
            this.resolveToken = resolve;
            this.rejectToken = reject;
            this.createWindow().catch(reject);
        });
    }

    private async createWindow() {
        console.log('[Webview-Login] Iniciando ventana de autenticación...');

        // Use the shared EventLoop instead of creating a new Application
        this.window = new WindowBuilder()
            .build(this.eventLoop);

        // Create injection script for WebSocket interception and IPC
        const injectionScript = this.createInjectionScript();

        this.webview = new WebViewBuilder()
            .withUrl("https://www.tiktok.com/login")
            .withDevtools(true)
            .buildOnWindow(this.window!, 'auth-webview');

        // Inject the script after the webview is created
        this.webview.evaluateScript(injectionScript);

        this.webview.openDevtools();

        // Setup IPC message handling
        this.webview.on((error, message) => {
            if (error) {
                console.error('[Webview-Login] IPC Error:', error);
                return;
            }
            const payload = message.toString();
            this.ipcRouter.handle(payload);
        });

        // Setup navigation monitoring
        this.setupNavigationMonitoring();

        // Load cookies if they exist
        await this.loadCookies();
    }

    private createInjectionScript(): string {
        return `
            (function() {
                // Setup IPC bridge
                if (!window.ipc) {
                    console.error('[Renderer] IPC not available');
                    return;
                }

                // WebSocket interceptor for TikTok data
                window.TiktokPayload = "";
                window.getPayload = function() {
                    return window.TiktokPayload;
                };

                const originalSend = WebSocket.prototype.send;
                WebSocket.prototype.send = function(data) {
                    if (typeof data === 'string' && data.includes("setUniqueId")) {
                        console.log("[Renderer] TikTok WebSocket data intercepted", data);
                        window.TiktokPayload = data;
                        window.ipc.postMessage(data);
                    }
                    return originalSend.apply(this, arguments);
                };

                // Navigation monitoring
                let lastUrl = window.location.href;
                const notifyNavigation = () => {
                    const context = {
                        type: 'WINDOW_CONTEXT',
                        payload: {
                            url: {
                                full: window.location.href,
                                protocol: window.location.protocol,
                                host: window.location.host,
                                pathname: window.location.pathname,
                                hash: window.location.hash,
                                origin: window.location.origin,
                                params: Object.fromEntries(new URLSearchParams(window.location.search))
                            },
                            document: {
                                title: document.title,
                                referrer: document.referrer,
                                language: navigator.language,
                                encoding: document.characterSet
                            },
                            screen: {
                                width: window.innerWidth,
                                height: window.innerHeight,
                                pixelRatio: window.devicePixelRatio,
                                orientation: screen.orientation ? screen.orientation.type : 'unknown'
                            },
                            userAgent: navigator.userAgent,
                            timestamp: new Date().toISOString()
                        }
                    };
                    window.ipc.postMessage(JSON.stringify(context));
                };

                // Monitor URL changes
                setInterval(() => {
                    if (window.location.href !== lastUrl) {
                        lastUrl = window.location.href;
                        notifyNavigation();
                    }
                }, 500);

                // Initial notification
                notifyNavigation();

                // Inject manual auth button
                const injectButton = () => {
                    if (document.getElementById('sl-auth-btn') || !window.location.href.includes('tiktok.com')) return;
                    
                    const btn = document.createElement('button');
                    btn.id = 'sl-auth-btn';
                    btn.innerText = 'Start Streamlabs Auth';
                    btn.style.cssText = 'position:fixed;top:10px;right:10px;z-index:99999;padding:12px 20px;background:#00f2ea;color:#000;font-weight:bold;border:none;border-radius:5px;cursor:pointer;box-shadow:0 4px 6px rgba(0,0,0,0.1);font-family:sans-serif;';
                    btn.onclick = function() {
                        const msg = {
                            type: 'USER_ACTION',
                            payload: { action: 'TRIGGER_STREAMLABS_AUTH', timestamp: Date.now() }
                        };
                        window.ipc.postMessage(JSON.stringify(msg));
                    };
                    document.body.appendChild(btn);
                    console.log('[Renderer] Botón de auth inyectado');
                };

                // Try to inject button periodically
                setInterval(injectButton, 2000);

                console.log("[Renderer] 💉 Scripts de inyección inicializados");
            })();
        `;
    }

    private setupNavigationMonitoring() {
        // Monitor URL changes via IPC
        this.ipcRouter.on(IpcMessageType.USER_ACTION, (payload) => {
            if (payload.action === 'TRIGGER_STREAMLABS_AUTH') {
                console.log('[Webview-Login] Botón de auth presionado manualmente');
                this.forceNavigateAuth();
            }
        });
    }

    private async loadCookies() {
        if (fs.existsSync(this.cookiesPath)) {
            try {
                const cookies = JSON.parse(fs.readFileSync(this.cookiesPath, 'utf-8'));
                console.log('[Webview-Login] Cookies file found:', cookies.length, 'cookies');
                
                // Inject cookies via JavaScript if needed
                if (this.webview && cookies.length > 0) {
                    const cookieScript = cookies.map((cookie: any) => {
                        const name = cookie.name;
                        const value = cookie.value;
                        const domain = cookie.domain.startsWith('.') ? cookie.domain.substring(1) : cookie.domain;
                        const path = cookie.path || '/';
                        const secure = cookie.secure ? 'true' : 'false';
                        return `document.cookie = "${name}=${value}; domain=${domain}; path=${path}; secure=${secure}";`;
                    }).join('\n');
                    
                    try {
                        this.webview.evaluateScript(cookieScript);
                    } catch (e: any) {
                        console.error('[Webview-Login] Error setting cookies:', e);
                    }
                }
            } catch (e) {
                console.error('[Webview-Login] Failed to load cookies:', e);
            }
        }
    }

    private async saveCookies() {
        try {
            // Extract cookies via JavaScript evaluation
            if (this.webview) {
                this.webview.evaluateScript(`
                    document.cookie.split(';').map(c => {
                        const [name, ...valueParts] = c.trim().split('=');
                        return { name, value: valueParts.join('=') };
                    })
                `);
                console.log('[Webview-Login] Cookies save attempted.');
            }
        } catch (e) {
            console.error('[Webview-Login] Failed to save cookies:', e);
        }
    }

    private checkLoginStatus(url: string) {
        if ((url.includes('tiktok.com') && !url.includes('login') && !url.includes('streamlabs')) || url.includes('/foryou')) {
            console.log('[Webview-Login] Login detected. Preparing to navigate to Streamlabs Auth...');

            setTimeout(() => {
                if (!url.includes('streamlabs')) {
                    this.forceNavigateAuth();
                }
            }, 2000);
        }
    }

    private forceNavigateAuth() {
        console.log(`[Webview-Login] Navigating to Auth URL: ${this.authUrl}`);
        try {
            this.webview?.evaluateScript(`window.location.href = "${this.authUrl}";`);
        } catch (e: any) {
            console.error('[Webview-Login] Failed to navigate to Auth URL:', e);
        }
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
            console.log('[Webview-Login] Success URL detected:', url);
            console.log('[Webview-Login] Authorization code extracted:', code);
            console.log('[Webview-Login] Starting token fetch...');

            this.saveCookies().then(() => {
                this.executeTokenFetch(code);
            });
        }
    }

    private async executeTokenFetch(code: string) {
        if (!this.codeVerifier) {
            console.error('[Webview-Login] No CodeVerifier found!');
            this.rejectToken?.(new Error('No CodeVerifier found'));
            return;
        }

        console.log('[Webview-Login] Fetching token from browser context...');

        const fetchScript = `
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
            const result = this.webview?.evaluateScript(fetchScript);
            this.handleFetchResult(result as unknown as AuthResult);
        } catch (err: any) {
            console.error('[Webview-Login] executeScript error:', err.message);
            this.handleFetchResult({ success: false, error: err.message });
        }
    }

    private handleFetchResult(result: AuthResult) {
        console.log('[Webview-Login] Token fetch result:', JSON.stringify(result));

        if (result.success && result.data?.success) {
            const authData = result.data.data;
            console.log('[Webview-Login] Auth data received successfully');
            
            // Include TikTok data in the result
            const finalResult = {
                ...authData,
                tiktok: this.tiktokData
            };
            
            this.resolveToken?.(finalResult);
            this.cleanup();
        } else {
            console.error('[Webview-Login] Error in fetch result:', JSON.stringify(result));
            this.rejectToken?.(new Error(`Fetch failed: ${JSON.stringify(result)}`));
            this.cleanup();
        }
    }

    private async cleanup() {
        await this.saveCookies();
        // Window will be cleaned up when the auth flow completes
        this.window = null;
        this.webview = null;
    }

    /**
     * Get the captured TikTok data
     */
    public getTikTokData(): TikTokData {
        return this.tiktokData;
    }
}
