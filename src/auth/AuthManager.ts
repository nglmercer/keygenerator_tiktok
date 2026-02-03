import crypto from 'node:crypto';
import path from 'path';
import { StreamlabsAuth } from './electron-login';
import { 
    AUTH_CONFIG, 
    PATHS, 
    API_ENDPOINTS, 
    CONSOLE_MESSAGES 
} from '../constants';
import { TokenStorage, FileUtils, resolveAppPath } from '../utils/fileUtils';

export class AuthManager {
    private codeVerifier: string;
    private codeChallenge: string;

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
        const params = new URLSearchParams({
            force_verify: AUTH_CONFIG.FORCE_VERIFY,
            external: AUTH_CONFIG.EXTERNAL,
            skip_splash: AUTH_CONFIG.SKIP_SPLASH,
            tiktok: '1',
            code_challenge: this.codeChallenge,
        });
        return `${API_ENDPOINTS.LOGIN_URL}?${params.toString()}`;
    }

    async retrieveToken(): Promise<string> {
        const tokenStorage = new TokenStorage(PATHS.TOKENS);
        const savedToken = tokenStorage.get();

        if (savedToken) {
            console.log(CONSOLE_MESSAGES.AUTH_SAVED_TOKEN);
            return savedToken;
        }

        console.log(CONSOLE_MESSAGES.AUTH_START_FLOW);

        const authUrl = await this.getAuthUrl();
        const cookiePathAbs = resolveAppPath(PATHS.COOKIES);

        const auth = new StreamlabsAuth(authUrl, cookiePathAbs, this.codeVerifier);
        const authData = await auth.findToken();

        tokenStorage.save(authData);
        console.log(CONSOLE_MESSAGES.AUTH_SAVED);

        return authData.oauth_token;
    }
}
