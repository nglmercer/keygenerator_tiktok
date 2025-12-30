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

  constructor(token: string) {
    this.client = axios.create({
      baseURL: 'https://streamlabs.com/api/v5/slobs/tiktok',
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 StreamlabsDesktop/1.17.0',
        'Authorization': `Bearer ${token}`
      }
    });
  }

  async search(game: string): Promise<StreamCategory[]> {
    if (!game) return [];
    // TikTok API often expects truncated game names for search
    const truncatedGame = game.substring(0, 25);
    try {
      const response = await this.client.get(`/info?category=${encodeURIComponent(truncatedGame)}`);
      return response.data.categories || [];
    } catch (error) {
      console.error('Error searching for game:', error);
      return [];
    }
  }

  async start(title: string, category: string, audienceType: string = '0'): Promise<StreamInfo | null> {
    const formData = new FormData();
    formData.append('title', title);
    formData.append('device_platform', 'win32');
    formData.append('category', category);
    formData.append('audience_type', audienceType);
    
    try {
      const response = await this.client.post('/stream/start', formData);
      return {
        rtmpUrl: response.data.rtmp,
        streamKey: response.data.key,
        id: response.data.id
      };
    } catch (error) {
      console.error('Error starting stream:', error);
      return null;
    }
  }

  async end(streamId: string): Promise<boolean> {
    try {
      const response = await this.client.post(`/stream/${streamId}/end`);
      return response.data.success;
    } catch (error) {
      console.error('Error ending stream:', error);
      return false;
    }
  }

  async getInfo(): Promise<any> {
    try {
      const response = await this.client.get('/info');
      return response.data;
    } catch (error) {
      console.error('Error getting info:', error);
      throw error;
    }
  }
}
