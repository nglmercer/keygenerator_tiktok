import { app, BrowserWindow, ipcMain } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { AuthManager } from './auth/AuthManager';
import { StreamAPI } from './api/StreamAPI';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow: BrowserWindow | null = null;
let streamAPI: StreamAPI | null = null;
let token: string | null = null;

async function createWindow() {
    mainWindow = new BrowserWindow({
        width: 1000,
        height: 800,
        title: 'TikTok Stream Key Generator',
        backgroundColor: '#0f172a',
        webPreferences: {
            preload: path.join(__dirname, 'ui/preload.js'),
            contextIsolation: true,
            nodeIntegration: false,
        },
    });

    // In production, we'd use a better path, but for dev:
    const indexPath = path.join(__dirname, 'ui/index.html');
    if (fs.existsSync(indexPath)) {
        mainWindow.loadFile(indexPath);
    } else {
        // Fallback for when running from dist or different structure
        mainWindow.loadFile(path.join(process.cwd(), 'src/ui/index.html'));
    }

    // mainWindow.webContents.openDevTools();
}

app.whenReady().then(async () => {
    setupIPC();
    createWindow();
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

function setupIPC() {
    // Authentication handler
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

    // Stream Info handler
    ipcMain.handle('stream:info', async () => {
        if (!streamAPI) return null;
        try {
            return await streamAPI.getInfo();
        } catch (error) {
            return null;
        }
    });

    // Search handler
    ipcMain.handle('stream:search', async (_event, query: string) => {
        if (!streamAPI) return [];
        return await streamAPI.search(query);
    });

    // Start Stream handler
    ipcMain.handle('stream:start', async (_event, { title, category }) => {
        if (!streamAPI) return null;
        return await streamAPI.start(title, category);
    });

    // End Stream handler
    ipcMain.handle('stream:end', async () => {
        if (!streamAPI) return false;
        return await streamAPI.end();
    });
}
