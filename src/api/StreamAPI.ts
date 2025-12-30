import axios, { type AxiosInstance } from 'axios';

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
      baseURL: 'https://streamlabs.com/api/v5/slobs/tiktok',
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) StreamlabsDesktop/1.17.0 Chrome/122.0.6261.156 Electron/29.3.1 Safari/537.36',
        'Authorization': `Bearer ${token}`
      }
    });
  }

  async search(game: string): Promise<StreamCategory[]> {
    if (!game) return this.getInitialCategories();

    // TikTok/Streamlabs API has a strict limit on the category parameter length.
    // Exceeding it often results in a 500 error.
    const query = game.trim();
    const truncatedGame = query.length > 25 ? query.substring(0, 25) : query;

    if (query.length > 25) {
      console.log(`[StreamAPI] Truncating search query from "${query}" to "${truncatedGame}" due to API limits.`);
    }

    try {
      console.log(`[StreamAPI] Searching for category: "${truncatedGame}"`);
      const response = await this.client.get(`/info?category=${encodeURIComponent(truncatedGame)}`);
      const results = response.data.categories || [];
      console.log(`[StreamAPI] Found ${results.length} matches for "${truncatedGame}"`);
      return results;
    } catch (error: any) {
      console.error('[StreamAPI] Search failed:', error.response?.status, error.message);
      return [];
    }
  }

  async getInitialCategories(): Promise<StreamCategory[]> {
    try {
      // Try to fetch 'gaming' by default to have something to show
      const response = await this.client.get('/info?category=gaming');
      return (response.data.categories || []).slice(0, 20);
    } catch (error) {
      return [{ full_name: 'Other', game_mask_id: '', id: 'other' }];
    }
  }

  async start(title: string, category: string, audienceType: string = '0'): Promise<StreamInfo | null> {
    try {
      // The Python snippet uses multipart/form-data via the 'files' parameter
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
        console.error('Error starting stream, unexpected response:', response.data);
        return null;
      }
    } catch (error: any) {
      console.error('Error starting stream:', error.response?.data || error.message);
      return null;
    }
  }

  async end(streamId?: string): Promise<boolean> {
    const id = streamId || this.currentStreamId;
    if (!id) {
      console.error('No stream ID provided to end the stream.');
      return false;
    }

    try {
      const response = await this.client.post(`/stream/${id}/end`);
      return response.data && response.data.success;
    } catch (error: any) {
      console.error('Error ending stream:', error.response?.data || error.message);
      return false;
    }
  }

  async getInfo(): Promise<any> {
    try {
      const response = await this.client.get('/info');
      console.log('[StreamAPI] Info response:', JSON.stringify(response.data));
      return response.data;
    } catch (error: any) {
      console.error('Error getting info:', error.response?.data || error.message);
      throw error;
    }
  }

  /**
   * Fetches the current user's profile and TikTok status
   */
  async getUserProfile(): Promise<any> {
    try {
      // In many unofficial Streamlabs TikTok integrations, the /info endpoint
      // contains the 'user' object with username, avatar, etc.
      const data = await this.getInfo();
      return data.user || null;
    } catch (error) {
      return null;
    }
  }

  /**
   * Fetches current stream status if any
   */
  async getCurrentStream(): Promise<any> {
    try {
      const response = await this.client.get('/stream/current');
      return response.data;
    } catch (error) {
      return null;
    }
  }
}
