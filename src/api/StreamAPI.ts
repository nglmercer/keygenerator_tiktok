import { 
    API_ENDPOINTS, 
    QUERY_PARAMS, 
    CONSOLE_MESSAGES 
} from '../constants.ts';
import { 
    BaseApiClient, 
    buildUrl, 
    toFormData, 
    truncate 
} from '../utils/apiClient.ts';

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

export class StreamAPI extends BaseApiClient {
    private currentStreamId: string | null = null;

    constructor(token: string) {
        super(API_ENDPOINTS.TIKTOK_BASE, token);
    }

    async search(game: string): Promise<StreamCategory[]> {
        if (!game) return this.getInitialCategories();

        const truncatedGame = truncate(game.trim(), QUERY_PARAMS.MAX_CATEGORY_LENGTH);

        if (game.trim().length > QUERY_PARAMS.MAX_CATEGORY_LENGTH) {
            console.log(CONSOLE_MESSAGES.API_SEARCH_TRUNCATED(game.trim(), truncatedGame));
        }

        console.log(CONSOLE_MESSAGES.API_SEARCH(truncatedGame));

        const response = await this.get<{ categories?: StreamCategory[] }>(
            `/info?category=${encodeURIComponent(truncatedGame)}`
        );

        const results = response?.categories || [];
        console.log(CONSOLE_MESSAGES.API_SEARCH_RESULTS(truncatedGame, results.length));
        return results;
    }

    async getInitialCategories(): Promise<StreamCategory[]> {
        const response = await this.get<{ categories?: StreamCategory[] }>(
            `/info?category=${QUERY_PARAMS.DEFAULT_CATEGORY}`
        );
        return (response?.categories || []).slice(0, QUERY_PARAMS.DEFAULT_LIMIT_CATEGORIES);
    }

    async start(title: string, category: string, audienceType: string = QUERY_PARAMS.DEFAULT_AUDIENCE_TYPE): Promise<StreamInfo | null> {
        const formData = toFormData({
            title,
            device_platform: 'win32',
            category,
            audience_type: audienceType,
        });

        const response = await this.post<{ id: string; rtmp: string; key: string }>('/stream/start', formData);

        if (response?.id) {
            this.currentStreamId = response.id;
            return {
                rtmpUrl: response.rtmp,
                streamKey: response.key,
                id: response.id
            };
        }

        console.error(CONSOLE_MESSAGES.API_START_ERROR, response);
        return null;
    }

    async end(streamId?: string): Promise<boolean> {
        const id = streamId || this.currentStreamId;
        if (!id) {
            console.error('[StreamAPI] No stream ID provided to end the stream.');
            return false;
        }

        const response = await this.post<{ success: boolean }>(`/stream/${id}/end`);
        return response?.success ?? false;
    }

    async getInfo(): Promise<any> {
        const response = await this.get<any>('/info');
        console.log('[StreamAPI] Info response:', JSON.stringify(response));
        return response;
    }

    async getUserProfile(): Promise<any> {
        const data = await this.getInfo();
        return data?.user || null;
    }

    async getCurrentStream(): Promise<any> {
        return await this.get<any>('/stream/current');
    }
}
