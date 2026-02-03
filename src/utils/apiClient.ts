import axios, { type AxiosInstance, type AxiosError } from 'axios';
import { USER_AGENT } from '../constants';

/**
 * Base API client with common patterns
 */
export abstract class BaseApiClient {
    protected client: AxiosInstance;
    protected baseURL: string;

    constructor(baseURL: string, token?: string, customUserAgent?: string) {
        this.baseURL = baseURL;
        this.client = axios.create({
            baseURL,
            headers: {
                'User-Agent': customUserAgent || USER_AGENT,
                ...(token && { 'Authorization': `Bearer ${token}` }),
            },
        });
    }

    /**
     * Safe GET request with error handling
     */
    protected async get<T = any>(endpoint: string): Promise<T | null> {
        try {
            const response = await this.client.get(endpoint);
            return response.data;
        } catch (error) {
            this.handleError('GET', endpoint, error);
            return null;
        }
    }

    /**
     * Safe POST request with error handling
     */
    protected async post<T = any>(endpoint: string, data?: any): Promise<T | null> {
        try {
            const response = await this.client.post(endpoint, data);
            return response.data;
        } catch (error) {
            this.handleError('POST', endpoint, error);
            return null;
        }
    }

    /**
     * Safe request with custom config
     */
    protected async request<T = any>(config: { method: string; url: string; data?: any }): Promise<T | null> {
        try {
            const response = await this.client.request(config);
            return response.data;
        } catch (error) {
            this.handleError(config.method, config.url, error);
            return null;
        }
    }

    /**
     * Standardized error handling
     */
    protected handleError(method: string, endpoint: string, error: unknown): void {
        const axiosError = error as AxiosError;
        console.error(`[API] ${method} ${endpoint}:`, axiosError.response?.status || 'Network Error', 
            axiosError.message);
    }

    /**
     * Get the axios client for custom requests
     */
    getClient(): AxiosInstance {
        return this.client;
    }
}

/**
 * Helper to create URL with query params
 */
export function buildUrl(base: string, path: string, params?: Record<string, string>): string {
    const url = new URL(path, base);
    if (params) {
        Object.entries(params).forEach(([key, value]) => {
            url.searchParams.append(key, value);
        });
    }
    return url.toString();
}

/**
 * Create form data from object
 */
export function toFormData(data: Record<string, string>): FormData {
    const formData = new FormData();
    Object.entries(data).forEach(([key, value]) => {
        formData.append(key, value);
    });
    return formData;
}

/**
 * Truncate string to max length
 */
export function truncate(text: string, maxLength: number): string {
    return text.length > maxLength ? text.substring(0, maxLength) : text;
}
