import http from 'k6/http';
import { check, group } from 'k6';

// Mixed: 80% reads, 15% batch, 5% writes
export const options = {
  stages: [
    { duration: '10s', target: 50 },
    { duration: '60s', target: 500 },
    { duration: '10s', target: 0 },
  ],
  thresholds: {
    'http_req_duration{type:read}': ['p(99)<10'],
    'http_req_duration{type:batch}': ['p(99)<50'],
    'http_req_duration{type:write}': ['p(99)<100'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const CONTENT_ID = '731b0395-4888-4822-b516-05b4b7bf2089';

export default function () {
  const roll = Math.random();

  if (roll < 0.80) {
    // 80% reads
    group('read', () => {
      const res = http.get(
        `${BASE_URL}/v1/likes/post/${CONTENT_ID}/count`,
        { tags: { type: 'read' } }
      );
      check(res, { 'read 200': (r) => r.status === 200 });
    });
  } else if (roll < 0.95) {
    // 15% batch
    group('batch', () => {
      const res = http.post(
        `${BASE_URL}/v1/likes/batch/counts`,
        JSON.stringify({
          items: [
            { content_type: 'post', content_id: CONTENT_ID },
            { content_type: 'post', content_id: 'a1b2c3d4-e5f6-4321-abcd-111111111111' },
          ],
        }),
        {
          headers: { 'Content-Type': 'application/json' },
          tags: { type: 'batch' },
        }
      );
      check(res, { 'batch 200': (r) => r.status === 200 });
    });
  } else {
    // 5% writes
    group('write', () => {
      const res = http.post(
        `${BASE_URL}/v1/likes`,
        JSON.stringify({
          content_type: 'post',
          content_id: CONTENT_ID,
        }),
        {
          headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer tok_user_1',
          },
          tags: { type: 'write' },
        }
      );
      check(res, { 'write 200 or 429': (r) => r.status === 200 || r.status === 429 });
    });
  }
}
