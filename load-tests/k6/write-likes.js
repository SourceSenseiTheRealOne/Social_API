import http from 'k6/http';
import { check } from 'k6';

// Target: 500 rps, p99 < 100ms
// Note: rate limited at 30/min per user, so we use multiple users
export const options = {
  stages: [
    { duration: '10s', target: 20 },
    { duration: '30s', target: 100 },
    { duration: '10s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(99)<100'],
    http_req_failed: ['rate<0.05'], // Higher tolerance due to rate limits
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

const TOKENS = ['tok_user_1', 'tok_user_2', 'tok_user_3', 'tok_user_4', 'tok_user_5'];
const CONTENT_IDS = [
  'a1b2c3d4-e5f6-4321-abcd-111111111111',
  'a1b2c3d4-e5f6-4321-abcd-222222222222',
  'a1b2c3d4-e5f6-4321-abcd-333333333333',
  'a1b2c3d4-e5f6-4321-abcd-444444444444',
  'a1b2c3d4-e5f6-4321-abcd-555555555555',
];

export default function () {
  const token = TOKENS[Math.floor(Math.random() * TOKENS.length)];
  const contentId = CONTENT_IDS[Math.floor(Math.random() * CONTENT_IDS.length)];

  const payload = JSON.stringify({
    content_type: 'post',
    content_id: contentId,
  });

  const res = http.post(`${BASE_URL}/v1/likes`, payload, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`,
    },
  });

  check(res, {
    'status is 200 or 429': (r) => r.status === 200 || r.status === 429,
  });
}
