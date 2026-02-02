import path from 'path';
import { serve, file } from 'bun';
import { WindowBuilder, WebViewBuilder, EventLoop, WebView } from 'webview-napi';
import { StreamAPI } from './api/StreamAPI.ts';
import { AuthManager } from './auth/AuthManager.ts';

// Bun server to serve the HTML UI
const server = serve({
    port: 3000,
    async fetch(req) {
        const url = new URL(req.url);
        let filePath = url.pathname;
        
        // Default to index.html
        if (filePath === '/') {
            filePath = '/index.html';
        }
        
        // Map to the src/ui directory
        const fullPath = path.join(import.meta.dir, 'ui', filePath);
        
        try {
            const f = file(fullPath);
            const exists = await f.exists();
            if (!exists) {
                return new Response('Not Found', { status: 404 });
            }
            
            // Set appropriate content type
            const ext = path.extname(filePath);
            const contentTypes: Record<string, string> = {
                '.html': 'text/html',
                '.js': 'application/javascript',
                '.css': 'text/css',
                '.json': 'application/json',
                '.png': 'image/png',
                '.jpg': 'image/jpeg',
                '.svg': 'image/svg+xml',
            };
            
            return new Response(f, {
                headers: {
                    'Content-Type': contentTypes[ext] || 'application/octet-stream',
                },
            });
        } catch (error) {
            console.error('[Server] Error serving file:', error);
            return new Response('Internal Server Error', { status: 500 });
        }
    },
});
const baseurl =  `http://localhost:${server.port}`
console.log(`[Server] Bun server running at ${baseurl}`);

const eventloop = new EventLoop();
const window = new WindowBuilder().build(eventloop);

const webview = new WebViewBuilder()
    .withUrl(baseurl)
    .withDevtools(true)
    .buildOnWindow(window, 'tiktok_keygen');

// IPC Handler Registry
interface IpcHandler {
    channel: string;
    handler: (args: any[]) => Promise<any>;
}

class IpcMain {
    private handlers: Map<string, (args: any[]) => Promise<any>> = new Map();
    private webview: WebView | null = null;

    setWebview(wv: WebView) {
        this.webview = wv;
        
        // Set up IPC message handler
        wv.on((error, message) => {
            if (error) {
                console.error('[IPC] Error receiving message:', error);
                return;
            }
            this.handleMessage(message);
        });
    }

    handle(channel: string, handler: (args: any[]) => Promise<any>) {
        this.handlers.set(channel, handler);
        console.log(`[IPC] Registered handler for: ${channel}`);
    }

    private async handleMessage(message: string) {
        try {
            const data = JSON.parse(message);
            const { channel, requestId, args } = data;

            const handler = this.handlers.get(channel);
            if (!handler) {
                console.warn(`[IPC] No handler registered for channel: ${channel}`);
                this.sendResponse(requestId, { error: `No handler for ${channel}` });
                return;
            }

            try {
                const result = await handler(args || []);
                this.sendResponse(requestId, { success: true, data: result });
            } catch (error: any) {
                console.error(`[IPC] Handler error for ${channel}:`, error);
                this.sendResponse(requestId, { success: false, error: error.message || 'Unknown error' });
            }
        } catch (error) {
            console.error('[IPC] Error parsing message:', error);
        }
    }

    private sendResponse(requestId: string, response: any) {
        if (this.webview) {
            this.webview.send(JSON.stringify({ requestId, ...response }));
        }
    }
}

const ipcMain = new IpcMain();

// StreamAPI instance
let streamAPI: StreamAPI | null = null;

// Set up IPC handlers
function setupIPC() {
    ipcMain.handle('auth:login', async () => {
        try {
            console.log('[Main] Starting authentication flow...');
            const authManager = new AuthManager();
            const token = await authManager.retrieveToken(eventloop);
            streamAPI = new StreamAPI(token);
            console.log('[Main] Authentication successful');
            return { success: true };
        } catch (error: any) {
            console.error('[Main] Authentication failed:', error);
            return { success: false, error: error.message || 'Unknown error during login' };
        }
    });

    ipcMain.handle('stream:info', async () => {
        if (!streamAPI) return null;
        try {
            return await streamAPI.getInfo();
        } catch (error) {
            return null;
        }
    });

    ipcMain.handle('stream:search', async (args: any[]) => {
        if (!streamAPI) return [];
        const query = args[0] || '';
        return await streamAPI.search(query);
    });

    ipcMain.handle('stream:start', async (args: any[]) => {
        if (!streamAPI) return null;
        const { title, category } = args[0] || {};
        return await streamAPI.start(title, category);
    });

    ipcMain.handle('stream:end', async () => {
        if (!streamAPI) return false;
        return await streamAPI.end();
    });

    ipcMain.handle('user:profile', async () => {
        if (!streamAPI) return null;
        return await streamAPI.getUserProfile();
    });

    ipcMain.handle('stream:current', async () => {
        if (!streamAPI) return null;
        return await streamAPI.getCurrentStream();
    });
}

// Initialize IPC
ipcMain.setWebview(webview);
setupIPC();

// Inject preload script to set up window.electronAPI
const preloadScript = `
(function() {
    'use strict';
    
    // Request ID generator
    let requestIdCounter = 0;
    const pendingRequests = new Map();
    
    // Set up IPC communication
    window.electronAPI = {
        invoke: async (channel, ...args) => {
            return new Promise((resolve, reject) => {
                const requestId = 'req_' + (++requestIdCounter) + '_' + Date.now();
                pendingRequests.set(requestId, { resolve, reject });
                
                // Send message to main process
                if (window.__webview_on_message__) {
                    window.__webview_on_message__(JSON.stringify({ channel, requestId, args }));
                } else if (window.ipc && window.ipc.postMessage) {
                    window.ipc.postMessage(JSON.stringify({ channel, requestId, args }));
                }
                
                // Timeout after 30 seconds
                setTimeout(() => {
                    if (pendingRequests.has(requestId)) {
                        pendingRequests.delete(requestId);
                        reject(new Error('IPC request timeout'));
                    }
                }, 30000);
            });
        },
        
        on: (channel, callback) => {
            // Store callback for the channel
            if (!window.__ipcListeners) {
                window.__ipcListeners = {};
            }
            if (!window.__ipcListeners[channel]) {
                window.__ipcListeners[channel] = [];
            }
            window.__ipcListeners[channel].push(callback);
        },
        
        send: (channel, ...args) => {
            // Fire and forget message
            if (window.__webview_on_message__) {
                window.__webview_on_message__(JSON.stringify({ channel, args, noResponse: true }));
            } else if (window.ipc && window.ipc.postMessage) {
                window.ipc.postMessage(JSON.stringify({ channel, args, noResponse: true }));
            }
        }
    };
    
    // Handle incoming messages from main process
    window.__on_webview_message__ = function(message) {
        try {
            const data = JSON.parse(message);
            const { requestId, success, data: responseData, error } = data;
            
            // Handle pending request responses
            if (requestId && pendingRequests.has(requestId)) {
                const { resolve, reject } = pendingRequests.get(requestId);
                pendingRequests.delete(requestId);
                
                if (error) {
                    reject(new Error(error));
                } else {
                    resolve(responseData);
                }
                return;
            }
            
            // Handle broadcast messages
            if (data.channel && window.__ipcListeners && window.__ipcListeners[data.channel]) {
                window.__ipcListeners[data.channel].forEach(cb => cb(...(data.args || [])));
            }
        } catch (e) {
            console.error('[Preload] Error handling message:', e);
        }
    };
    
    console.log('[Preload] electronAPI initialized');
})();
`;

// Evaluate the preload script in the webview
webview.evaluateScript(preloadScript);

//webview.openDevtools();
setInterval(async function(){
   if ( eventloop.runIteration()){
       window.id;
       webview.id;
   }else {
    process.exit(1)
   }
},16)
