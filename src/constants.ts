/**
 * Application Constants
 * Centralized magic strings and configuration values
 */

// ============== IPC Channel Names ==============
export const IPC_CHANNELS = {
    // Auth channels
    AUTH_LOGIN: 'auth:login',
    
    // Stream channels
    STREAM_INFO: 'stream:info',
    STREAM_SEARCH: 'stream:search',
    STREAM_START: 'stream:start',
    STREAM_END: 'stream:end',
    STREAM_CURRENT: 'stream:current',
    
    // User channels
    USER_PROFILE: 'user:profile',
    
    // Electron-login channels
    LOG_CONSOLE: 'log-console',
    FETCH_RESULT: 'fetch-result',
    TRIGGER_STREAMLABS_AUTH: 'TRIGGER_STREAMLABS_AUTH',
} as const;

// ============== API Endpoints ==============
export const API_BASE_URL = 'https://streamlabs.com/api/v5/slobs';

export const API_ENDPOINTS = {
    TIKTOK_BASE: `${API_BASE_URL}/tiktok`,
    AUTH_DATA: `${API_BASE_URL}/auth/data`,
    TIKTOK_AUTH: 'https://streamlabs.com/tiktok/auth',
    LOGIN_URL: 'https://streamlabs.com/m/login',
    TIKTOK_LOGIN: 'https://www.tiktok.com/login',
    DASHBOARD: 'https://streamlabs.com/dashboard',
    SLOBS_DASHBOARD: 'https://streamlabs.com/slobs/dashboard',
} as const;

// ============== Authentication Constants ==============
export const AUTH_CONFIG = {
    CLIENT_KEY: 'awdjaq9ide8ofrtz',
    REDIRECT_URI: 'https://streamlabs.com/tiktok/auth',
    FORCE_VERIFY: '1',
    EXTERNAL: 'mobile',
    SKIP_SPLASH: '1',
} as const;

// ============== Window Configuration ==============
export const WINDOW_CONFIG = {
    MAIN: {
        width: 1000,
        height: 800,
        title: 'TikTok Stream Key Generator',
        backgroundColor: '#0f172a',
    },
    AUTH: {
        width: 1280,
        height: 800,
        title: 'TikTok Auth - Streamlabs',
    },
} as const;

// ============== File Paths ==============
export const PATHS = {
    PRELOAD: 'src/ui/preload.js',
    PRELOAD_AUTH: 'src/auth/preload.js',
    INDEX_HTML: 'src/ui/index.html',
    COOKIES: 'cookies.json',
    TOKENS: 'tokens.json',
    CONFIG: 'config.json',
} as const;

// ============== User Agent ==============
export const USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) StreamlabsDesktop/1.17.0 Chrome/122.0.6261.156 Electron/29.3.1 Safari/537.36';

// ============== API Query Parameters ==============
export const QUERY_PARAMS = {
    DEFAULT_CATEGORY: 'gaming',
    MAX_CATEGORY_LENGTH: 25,
    DEFAULT_AUDIENCE_TYPE: '0',
    DEFAULT_LIMIT_CATEGORIES: 20,
} as const;

// ============== WebPreferences ==============
export const WEB_PREFERENCES = {
    NODE_INTEGRATION: false,
    CONTEXT_ISOLATION: true,
    SANDBOX: false,
} as const;

// ============== Error Messages ==============
export const ERROR_MESSAGES = {
    AUTH_FAILED: 'Unknown error during login',
    NO_STREAM_API: 'Stream API not initialized',
    NO_STREAM_ID: 'No stream ID provided to end the stream',
    JSON_PARSE_ERROR: 'JSON Parse Error',
    NO_CODE_VERIFIER: 'No CodeVerifier found',
    WINDOW_CLOSED: 'Window closed by user',
    FETCH_FAILED: 'Fetch failed',
} as const;

// ============== Console Messages ==============
export const CONSOLE_MESSAGES = {
    AUTH_START: '[Main] Starting authentication flow...',
    AUTH_SUCCESS: '[Main] Authentication successful',
    AUTH_FAILED: '[Main] Authentication failed:',
    API_SEARCH: (query: string) => `[StreamAPI] Searching for category: "${query}"`,
    API_SEARCH_TRUNCATED: (original: string, truncated: string) => 
        `[StreamAPI] Truncating search query from "${original}" to "${truncated}" due to API limits.`,
    API_SEARCH_RESULTS: (query: string, count: number) => 
        `[StreamAPI] Found ${count} matches for "${query}"`,
    API_START_ERROR: 'Error starting stream, unexpected response:',
    API_END_ERROR: 'Error ending stream:',
    API_INFO_ERROR: 'Error getting info:',
    AUTH_SAVED_TOKEN: '[AuthManager] Using saved token from tokens.json',
    AUTH_LOAD_FAIL: '[AuthManager] Failed to load saved tokens:',
    AUTH_START_FLOW: '[AuthManager] Starting authentication via internal Electron window...',
    AUTH_SAVED: '[AuthManager] Tokens saved to tokens.json',
    ELECTRON_PRELOAD: (path: string) => `[Electron-Login] Preload path: ${path}`,
    ELECTRON_NAVIGATE: '[Electron-Login] Navigating to TikTok login...',
    ELECTRON_LOGIN_DETECTED: '[Electron-Login] Login detected. Preparing to navigate to Streamlabs Auth...',
    ELECTRON_FORCE_NAVIGATE: (url: string) => `[Electron-Login] Navigating to Auth URL: ${url}`,
    ELECTRON_SUCCESS: (url: string) => `[Electron-Login] Success URL detected: ${url}`,
    ELECTRON_CODE: (code: string) => `[Electron-Login] Authorization code extracted: ${code}`,
    ELECTRON_FETCH_START: '[Electron-Login] Starting token fetch...',
    ELECTRON_FETCHING: '[Electron-Login] Fetching token from browser context...',
    ELECTRON_NO_VERIFIER: '[Electron-Login] No CodeVerifier found!',
    ELECTRON_RESULT: (result: string) => `[Electron-Login] Token fetch result: ${result}`,
    ELECTRON_AUTH_SUCCESS: '[Electron-Login] Auth data received successfully',
    ELECTRON_ERROR_RESULT: (result: string) => `[Electron-Login] Error in fetch result: ${result}`,
    ELECTRON_COOKIES_LOADED: '[Electron-Login] Cookies loaded.',
    ELECTRON_COOKIES_SAVE_ERROR: '[Electron-Login] Failed to save cookies:',
    ELECTRON_JS_ERROR: (message: string) => `[Electron-Login] executeJavaScript error: ${message}`,
} as const;

// ============== Type Aliases ==============
export type IpcChannel = typeof IPC_CHANNELS[keyof typeof IPC_CHANNELS];
export type ApiEndpoint = typeof API_ENDPOINTS[keyof typeof API_ENDPOINTS];
