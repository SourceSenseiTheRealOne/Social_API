import http from 'k6/http';
import { check } from 'k6';

// Target: 1k rps, p99 < 50ms
export const options = {
  stages: [
    { duration: '10s', target: 50 },
    { duration: '30s', target: 200 },
    { duration: '10s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(99)<50'],
    http_req_failed: ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

const BATCH_PAYLOAD = JSON.stringify({
  items: [
    { content_type: 'post', content_id: '731b0395-4888-4822-b516-05b4b7bf2089' },
    { content_type: 'post', content_id: 'a1b2c3d4-e5f6-4321-abcd-111111111111' },
    { content_type: 'post', content_id: 'a1b2c3d4-e5f6-4321-abcd-222222222222' },
    { content_type: 'bonus_hunter', content_id: 'b1b2c3d4-e5f6-4321-abcd-111111111111' },
    { content_type: 'top_picks', content_id: 'c1b2c3d4-e5f6-4321-abcd-111111111111' },
  ],
});

export default function () {
  const res = http.post(
    `${BASE_URL}/v1/likes/batch/counts`,
    BATCH_PAYLOAD,
    { headers: { 'Content-Type': 'application/json' } }
  );

  check(res, {
    'status is 200': (r) => r.status === 200,
    'has counts': (r) => JSON.parse(r.body).counts.length > 0,
  });
}
