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
    if (!game) return [];
    // If the game name exceeds 25 characters, the API will return error 500
    const truncatedGame = game.substring(0, 25);
    try {
      const response = await this.client.get(`/info?category=${encodeURIComponent(truncatedGame)}`);
      const categories = response.data.categories || [];
      // Python snippet adds "Other" category
      categories.push({ full_name: 'Other', game_mask_id: '' });
      return categories;
    } catch (error) {
      console.error('Error searching for game:', error);
      return [];
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
      return response.data;
    } catch (error: any) {
      console.error('Error getting info:', error.response?.data || error.message);
      throw error;
    }
  }
}
