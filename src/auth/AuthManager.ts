import path from 'path';
import fs from 'fs';
import crypto from 'node:crypto';
import type { EventLoop } from 'webview-napi';
import { StreamlabsAuth } from './electron-login.ts';

export interface AuthData {
    oauth_token: string;
    uniqueId?: string;
    roomId?: string;
    cookies?: any[];
    [key: string]: any;
}

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

    async retrieveToken(eventLoop: EventLoop): Promise<string> {
        const authData = await this.retrieveAuthData(eventLoop);
        return authData.oauth_token;
    }

    async retrieveAuthData(eventLoop: EventLoop): Promise<AuthData> {
        const tokenPath = path.resolve(process.cwd(), 'tokens.json');

        // Try to load existing token
        if (fs.existsSync(tokenPath)) {
            try {
                const saved = JSON.parse(fs.readFileSync(tokenPath, 'utf-8'));
                if (saved && saved.oauth_token) {
                    console.log('[AuthManager] Using saved token from tokens.json');
                    return saved;
                }
            } catch (e) {
                console.error('[AuthManager] Failed to load saved tokens:', e);
            }
        }

        const authUrl = await this.getAuthUrl();
        const cookiePathAbs = path.resolve(process.cwd(), this.COOKIES_PATH);

        console.log('[AuthManager] Starting authentication via internal webview window...');

        const auth = new StreamlabsAuth(authUrl, cookiePathAbs, this.codeVerifier, eventLoop);
        const authData = await auth.findToken();

        // Save all data for later discovery scripts
        fs.writeFileSync(tokenPath, JSON.stringify(authData, null, 2));
        console.log('[AuthManager] Tokens saved to tokens.json');

        return authData;
    }
}
