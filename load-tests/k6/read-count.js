import http from 'k6/http';
import { check, sleep } from 'k6';

// Target: 10k rps, p99 < 10ms
export const options = {
  stages: [
    { duration: '10s', target: 100 },   // Ramp up
    { duration: '30s', target: 1000 },  // Sustain
    { duration: '10s', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<10'],     // p99 < 10ms
    http_req_failed: ['rate<0.01'],      // <1% error rate
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const CONTENT_ID = '731b0395-4888-4822-b516-05b4b7bf2089';

export default function () {
  const res = http.get(
    `${BASE_URL}/v1/likes/post/${CONTENT_ID}/count`
  );

  check(res, {
    'status is 200': (r) => r.status === 200,
    'has count field': (r) => JSON.parse(r.body).count !== undefined,
  });
}
