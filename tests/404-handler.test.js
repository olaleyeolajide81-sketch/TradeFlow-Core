const request = require('supertest');
const app = require('../server');

describe('404 Not Found Handler', () => {
    test('GET /nonexistent returns 404 JSON', async () => {
        const res = await request(app).get('/nonexistent');
        expect(res.status).toBe(404);
        expect(res.body.error).toBe('Route not found');
        expect(res.headers['content-type']).toMatch(/application\/json/);
    });

    test('POST /nonexistent returns 404 JSON', async () => {
        const res = await request(app).post('/nonexistent');
        expect(res.status).toBe(404);
        expect(res.body.error).toBe('Route not found');
        expect(res.headers['content-type']).toMatch(/application\/json/);
    });

    test('PUT /api/nonexistent returns 404 JSON', async () => {
        const res = await request(app).put('/api/nonexistent');
        expect(res.status).toBe(404);
        expect(res.body.error).toBe('Route not found');
        expect(res.headers['content-type']).toMatch(/application\/json/);
    });

    test('DELETE /api/v1/nonexistent returns 404 JSON', async () => {
        const res = await request(app).delete('/api/v1/nonexistent');
        expect(res.status).toBe(404);
        expect(res.body.error).toBe('Route not found');
        expect(res.headers['content-type']).toMatch(/application\/json/);
    });
});
