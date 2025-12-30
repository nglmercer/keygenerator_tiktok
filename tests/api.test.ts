import { describe, it, expect, mock, beforeAll } from 'bun:test';
import { StreamAPI } from '../src/api/StreamAPI';

// Mock axios
const mockPost = mock(() => Promise.resolve({ data: { rtmp: 'rtmp://test', key: 'key123', id: '123' } }));
const mockGet = mock(() => Promise.resolve({ data: { categories: [{ id: '1', full_name: 'Test Game', game_mask_id: '100' }] } }));

mock.module('axios', () => {
    return {
        default: {
            create: () => ({
                post: mockPost,
                get: mockGet
            })
        }
    };
});

describe('StreamAPI', () => {
    const api = new StreamAPI('fake-token');

    it('should search for games', async () => {
        const results = await api.search('Test');
        expect(results).toHaveLength(1);
        expect(results[0]!.full_name).toBe('Test Game');
        expect(mockGet).toHaveBeenCalled();
    });

    it('should start a stream', async () => {
        const result = await api.start('My Stream', '1');
        expect(result).not.toBeNull();
        expect(result?.rtmpUrl).toBe('rtmp://test');
        expect(result?.streamKey).toBe('key123');
        expect(mockPost).toHaveBeenCalled();
    });
});
