import path from 'path';

import { WindowBuilder,WebViewBuilder,EventLoop } from 'webview-napi';
import html from './ui/index.html' with { type: "text" };
import { StreamAPI } from './api/StreamAPI.ts';
import { AuthManager } from './auth/AuthManager.ts';

const eventloop = new EventLoop();
const window = new WindowBuilder().build(eventloop);

const webview = new WebViewBuilder()
                .withUrl('http:localhost:3000')
                .withDevtools(true)
                .buildOnWindow(window,'tiktok_keygen');


webview.openDevtools()
eventloop.run()
function setupIPC() {
    let streamAPI:any;
    let token:any;
    const ipcMain = {
        handle: (a:string,cb:Function) => {}
    };
    ipcMain.handle('auth:login', async () => {
        try {
            console.log('[Main] Starting authentication flow...');
            const authManager = new AuthManager();
            token = await authManager.retrieveToken();
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

    ipcMain.handle('stream:search', async (_event: any, query: string) => {
        if (!streamAPI) return [];
        return await streamAPI.search(query);
    });

    ipcMain.handle('stream:start', async (_event: any, { title, category }: any) => {
        if (!streamAPI) return null;
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

