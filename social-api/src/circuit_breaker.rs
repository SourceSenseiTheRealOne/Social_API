use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub failure_rate_threshold: u32,
    pub failure_rate_window: Duration,
    pub open_duration: Duration,
    pub half_open_successes: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_rate_threshold: 50,
            failure_rate_window: Duration::from_secs(30),
            open_duration: Duration::from_secs(30),
            half_open_successes: 3,
        }
    }
}

pub struct CircuitBreaker {
    service_name: String,
    config: CircuitBreakerConfig,
    inner: Mutex<CircuitBreakerInner>,
}

struct CircuitBreakerInner {
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    recent_calls: VecDeque<(Instant, bool)>,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(service_name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            service_name,
            config,
            inner: Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
                recent_calls: VecDeque::new(),
                opened_at: None,
            }),
        }
    }

    pub fn check(&self) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();

        match inner.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                if let Some(opened_at) = inner.opened_at {
                    if opened_at.elapsed() >= self.config.open_duration {
                        tracing::warn!(
                            service = %self.service_name,
                            "Circuit breaker transitioning: Open -> HalfOpen"
                        );
                        inner.state = CircuitState::HalfOpen;
                        inner.consecutive_successes = 0;
                        Ok(())
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    pub fn record_success(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.consecutive_failures = 0;
        inner.recent_calls.push_back((Instant::now(), true));
        self.trim_old_calls(&mut inner);

        if inner.state == CircuitState::HalfOpen {
            inner.consecutive_successes += 1;
            if inner.consecutive_successes >= self.config.half_open_successes {
                tracing::info!(
                    service = %self.service_name,
                    "Circuit breaker transitioning: HalfOpen -> Closed"
                );
                inner.state = CircuitState::Closed;
                inner.consecutive_successes = 0;
            }
        }
    }

    pub fn record_failure(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.consecutive_failures += 1;
        inner.consecutive_successes = 0;
        inner.recent_calls.push_back((Instant::now(), false));
        self.trim_old_calls(&mut inner);

        match inner.state {
            CircuitState::Closed => {
                let should_open = self.should_open(&inner);
                if should_open {
                    tracing::warn!(
                        service = %self.service_name,
                        consecutive_failures = inner.consecutive_failures,
                        "Circuit breaker transitioning: Closed -> Open"
                    );
                    inner.state = CircuitState::Open;
                    inner.opened_at = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!(
                    service = %self.service_name,
                    "Circuit breaker transitioning: HalfOpen -> Open"
                );
                inner.state = CircuitState::Open;
                inner.opened_at = Some(Instant::now());
            }
            _ => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        self.inner.lock().unwrap().state
    }

    fn should_open(&self, inner: &CircuitBreakerInner) -> bool {
        if inner.consecutive_failures >= self.config.failure_threshold {
            return true;
        }

        if inner.recent_calls.len() >= 5 {
            let total = inner.recent_calls.len() as u32;
            let failures = inner
                .recent_calls
                .iter()
                .filter(|(_, success)| !success)
                .count() as u32;
            let rate = (failures * 100) / total;
            if rate > self.config.failure_rate_threshold {
                return true;
            }
        }

        false
    }

    fn trim_old_calls(&self, inner: &mut CircuitBreakerInner) {
        let cutoff = Instant::now() - self.config.failure_rate_window;
        while inner
            .recent_calls
            .front()
            .is_some_and(|(ts, _)| *ts < cutoff)
        {
            inner.recent_calls.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cb() -> CircuitBreaker {
        CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                failure_rate_threshold: 50,
                failure_rate_window: Duration::from_secs(30),
                open_duration: Duration::from_millis(100),
                half_open_successes: 2,
            },
        )
    }

    #[test]
    fn starts_closed() {
        let cb = test_cb();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.check().is_ok());
    }

    #[test]
    fn opens_after_consecutive_failures() {
        let cb = test_cb();

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.check().is_err());
    }

    #[test]
    fn success_resets_consecutive_failures() {
        let cb = test_cb();

        cb.record_failure();
        cb.record_failure();
        cb.record_success();

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn transitions_to_half_open_after_timeout() {
        let cb = test_cb();

        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(150));

        assert!(cb.check().is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_closes_after_successes() {
        let cb = test_cb();

        for _ in 0..3 {
            cb.record_failure();
        }

        std::thread::sleep(Duration::from_millis(150));
        cb.check().ok();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_reopens_on_failure() {
        let cb = test_cb();

        for _ in 0..3 {
            cb.record_failure();
        }
        std::thread::sleep(Duration::from_millis(150));
        cb.check().ok();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
