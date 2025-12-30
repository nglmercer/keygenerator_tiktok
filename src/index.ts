import path from 'path';
import { fileURLToPath } from 'url';
import { spawn } from 'child_process';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Self-relaunch in Electron if running in Bun/Node
if (!process.versions.electron) {
    const electronPath = path.resolve(process.cwd(), 'node_modules', '.bin', 'electron');
    spawn(electronPath, ['-r', 'ts-node/register', __filename], {
        stdio: 'inherit',
        env: { ...process.env, ELECTRON_RUN_AS_NODE: '' }
    });
    process.exit(0);
}

// Re-importing inside the Electron environment using dynamic imports to satisfy Bun's pre-parsing
async function init() {
    const { app, BrowserWindow, ipcMain } = await import('electron');
    const { AuthManager } = await import('./auth/AuthManager.ts');
    const { StreamAPI } = await import('./api/StreamAPI.ts');

    let mainWindow: any = null;
    let streamAPI: any = null;
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

        const indexPath = path.join(__dirname, 'ui/index.html');
        if (fs.existsSync(indexPath)) {
            mainWindow.loadFile(indexPath);
        } else {
            mainWindow.loadFile(path.join(process.cwd(), 'src/ui/index.html'));
        }
    }

    app.whenReady().then(async () => {
        setupIPC(ipcMain, AuthManager, StreamAPI);
        createWindow();
    });

    app.on('window-all-closed', () => {
        if (process.platform !== 'darwin') app.quit();
    });

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });

    function setupIPC(ipcMain: any, AuthManager: any, StreamAPI: any) {
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
    }
}

init().catch(err => {
    console.error('Failed to initialize app:', err);
});
