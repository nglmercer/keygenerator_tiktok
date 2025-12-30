import { describe, it, expect } from 'bun:test';
import { AuthManager } from '../src/auth/AuthManager';

describe('AuthManager', () => {
    const authManager = new AuthManager();

    it('should generate code verifier and challenge', () => {
        // We can't access private properties easily in TS without casting to any or changing visibility.
        // Ideally we'd export the generator functions or make them public/internal.
        // For this test, we verify the public method getAuthUrl contains the challenge.

        // We can interact with private members via 'any' cast for testing purposes or test public behavior
        const verifier = (authManager as any).codeVerifier;
        const challenge = (authManager as any).codeChallenge;

        expect(verifier).toBeDefined();
        expect(challenge).toBeDefined();
        expect(verifier.length).toBeGreaterThan(0);
        expect(challenge.length).toBeGreaterThan(0);
    });

    it('should generate correct auth URL', async () => {
        const url = await authManager.getAuthUrl();
        expect(url).toContain('https://streamlabs.com/m/login');
        expect(url).toContain('code_challenge=');
        expect(url).toContain('force_verify=1');
    });
});
