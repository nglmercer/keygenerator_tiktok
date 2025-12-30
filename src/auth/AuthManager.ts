import { spawn } from 'child_process';
import path from 'path';
import fs from 'fs';
import axios from 'axios';
import crypto from 'node:crypto';

export class AuthManager {
    private CLIENT_KEY = 'awdjaq9ide8ofrtz';
    private REDIRECT_URI = 'https://streamlabs.com/tiktok/auth';
    private AUTH_DATA_URL = 'https://streamlabs.com/api/v5/slobs/auth/data';

    private codeVerifier: string;
    private codeChallenge: string;
    private COOKIES_PATH = 'cookies.json';

    constructor() {
        this.codeVerifier = this.generateCodeVerifier();
        this.codeChallenge = this.generateCodeChallenge(this.codeVerifier);
    }

    private generateCodeVerifier(): string {
        return crypto.randomBytes(64).toString('hex');
    }

    private generateCodeChallenge(verifier: string): string {
        const hash = crypto.createHash('sha256').update(verifier).digest();
        return hash.toString('base64')
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }

    async getAuthUrl(): Promise<string> {
        return `https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge=${this.codeChallenge}`;
    }

    async retrieveToken(): Promise<string> {
        const authUrl = await this.getAuthUrl();
        console.log('Launching Electron to handle authentication...');

        return new Promise((resolve, reject) => {
            const electronPath = path.resolve(process.cwd(), 'node_modules', '.bin', 'electron');
            const scriptPath = path.resolve(process.cwd(), 'src/auth/electron-login.ts');
            const cookiePathAbs = path.resolve(process.cwd(), this.COOKIES_PATH);

            const args = ['-r', 'ts-node/register', scriptPath];

            console.log(`Spawning: ${electronPath} ${args.join(' ')}`);

            const child = spawn(electronPath, args, {
                stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
                env: {
                    ...process.env,
                    AUTH_URL: authUrl,
                    COOKIES_PATH: cookiePathAbs,
                    CODE_VERIFIER: this.codeVerifier
                }
            });

            if (!child || !child.stdout || !child.stderr) {
                return reject(new Error('Failed to spawn Electron process'));
            }

            child.stdout.on('data', (data) => {
                const output = data.toString().trim();
                console.log(`[Electron]: ${output}`);
            });

            child.stderr.on('data', (data) => {
                // console.error(`[Electron Log]: ${data}`);
            });

            child.on('message', (message: any) => {
                if (message && message.type === 'token-success') {
                    console.log('[Parent] Received token from Electron.');
                    resolve(message.token);
                } else if (message && message.type === 'login-success') {
                    console.log('[Parent] Login success signaled (waiting for token...)');
                } else if (message && message.type === 'error') {
                    // Don't reject yet, process might exit with error code
                    console.error('[Parent] Error from Electron:', message.error);
                }
            });

            child.on('close', async (code) => {
                console.log('Electron process closed with code:', code);
                // We don't need to do anything here as the token should have been received via IPC
            });

            child.on('error', (err) => {
                reject(new Error(`Failed to start Electron process: ${err.message}`));
            });
        });
    }
}
