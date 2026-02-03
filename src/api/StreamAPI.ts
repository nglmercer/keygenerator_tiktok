import axios, { type AxiosInstance } from 'axios';
import { 
    API_ENDPOINTS, 
    USER_AGENT, 
    QUERY_PARAMS, 
    CONSOLE_MESSAGES 
} from '../constants.ts';

export interface StreamInfo {
    rtmpUrl: string;
    streamKey: string;
    id: string;
}

export interface StreamCategory {
    id: string;
    full_name: string;
    game_mask_id: string;
}

export class StreamAPI {
    private client: AxiosInstance;
    private currentStreamId: string | null = null;

    constructor(token: string) {
        this.client = axios.create({
            baseURL: API_ENDPOINTS.TIKTOK_BASE,
            headers: {
                'User-Agent': USER_AGENT,
                'Authorization': `Bearer ${token}`
            }
        });
    }

    async search(game: string): Promise<StreamCategory[]> {
        if (!game) return this.getInitialCategories();

        const query = game.trim();
        const truncatedGame = query.length > QUERY_PARAMS.MAX_CATEGORY_LENGTH 
            ? query.substring(0, QUERY_PARAMS.MAX_CATEGORY_LENGTH) 
            : query;

        if (query.length > QUERY_PARAMS.MAX_CATEGORY_LENGTH) {
            console.log(CONSOLE_MESSAGES.API_SEARCH_TRUNCATED(query, truncatedGame));
        }

        try {
            console.log(CONSOLE_MESSAGES.API_SEARCH(truncatedGame));
            const response = await this.client.get(`/info?category=${encodeURIComponent(truncatedGame)}`);
            const results = response.data.categories || [];
            console.log(CONSOLE_MESSAGES.API_SEARCH_RESULTS(truncatedGame, results.length));
            return results;
        } catch (error: any) {
            console.error('[StreamAPI] Search failed:', error.response?.status, error.message);
            return [];
        }
    }

    async getInitialCategories(): Promise<StreamCategory[]> {
        try {
            const response = await this.client.get(`/info?category=${QUERY_PARAMS.DEFAULT_CATEGORY}`);
            return (response.data.categories || []).slice(0, QUERY_PARAMS.DEFAULT_LIMIT_CATEGORIES);
        } catch (error) {
            return [{ full_name: 'Other', game_mask_id: '', id: 'other' }];
        }
    }

    async start(title: string, category: string, audienceType: string = QUERY_PARAMS.DEFAULT_AUDIENCE_TYPE): Promise<StreamInfo | null> {
        try {
            const formData = new FormData();
            formData.append('title', title);
            formData.append('device_platform', 'win32');
            formData.append('category', category);
            formData.append('audience_type', audienceType);

            const response = await this.client.post('/stream/start', formData);

            if (response.data && response.data.id) {
                this.currentStreamId = response.data.id;
                return {
                    rtmpUrl: response.data.rtmp,
                    streamKey: response.data.key,
                    id: response.data.id
                };
            } else {
                console.error(CONSOLE_MESSAGES.API_START_ERROR, response.data);
                return null;
            }
        } catch (error: any) {
            console.error(CONSOLE_MESSAGES.API_END_ERROR, error.response?.data || error.message);
            return null;
        }
    }

    async end(streamId?: string): Promise<boolean> {
        const id = streamId || this.currentStreamId;
        if (!id) {
            console.error('[StreamAPI] No stream ID provided to end the stream.');
            return false;
        }

        try {
            const response = await this.client.post(`/stream/${id}/end`);
            return response.data && response.data.success;
        } catch (error: any) {
            console.error(CONSOLE_MESSAGES.API_END_ERROR, error.response?.data || error.message);
            return false;
        }
    }

    async getInfo(): Promise<any> {
        try {
            const response = await this.client.get('/info');
            console.log('[StreamAPI] Info response:', JSON.stringify(response.data));
            return response.data;
        } catch (error: any) {
            console.error(CONSOLE_MESSAGES.API_INFO_ERROR, error.response?.data || error.message);
            throw error;
        }
    }

    async getUserProfile(): Promise<any> {
        try {
            const data = await this.getInfo();
            return data.user || null;
        } catch (error) {
            return null;
        }
    }

    async getCurrentStream(): Promise<any> {
        try {
            const response = await this.client.get('/stream/current');
            return response.data;
        } catch (error) {
            return null;
        }
    }
}
