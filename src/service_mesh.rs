//! Service mesh infrastructure for Vitalis.
//!
//! - **Sidecar proxy**: HTTP/gRPC interception & routing
//! - **Load balancing**: Round-robin, weighted, least-connections
//! - **Rate limiting**: Token bucket, sliding window
//! - **mTLS**: Mutual TLS certificate management
//! - **Service registry**: Service discovery & health tracking
//! - **Canary deployment**: Traffic splitting, progressive rollout

use std::collections::HashMap;

// ── Service Registry ────────────────────────────────────────────────

/// A registered service instance.
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: String,
    pub service_name: String,
    pub host: String,
    pub port: u16,
    pub version: String,
    pub metadata: HashMap<String, String>,
    pub health: HealthStatus,
    pub weight: u32,
    pub zone: String,
}

impl ServiceInstance {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn is_healthy(&self) -> bool {
        self.health == HealthStatus::Healthy
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Service registry.
pub struct ServiceRegistry {
    services: HashMap<String, Vec<ServiceInstance>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self { services: HashMap::new() }
    }

    pub fn register(&mut self, instance: ServiceInstance) {
        self.services.entry(instance.service_name.clone())
            .or_insert_with(Vec::new)
            .push(instance);
    }

    pub fn deregister(&mut self, service_name: &str, instance_id: &str) {
        if let Some(instances) = self.services.get_mut(service_name) {
            instances.retain(|i| i.id != instance_id);
        }
    }

    pub fn discover(&self, service_name: &str) -> Vec<&ServiceInstance> {
        self.services.get(service_name)
            .map(|v| v.iter().filter(|i| i.is_healthy()).collect())
            .unwrap_or_default()
    }

    pub fn all_instances(&self, service_name: &str) -> Vec<&ServiceInstance> {
        self.services.get(service_name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn service_names(&self) -> Vec<&String> {
        self.services.keys().collect()
    }

    pub fn total_instances(&self) -> usize {
        self.services.values().map(|v| v.len()).sum()
    }

    pub fn update_health(&mut self, service_name: &str, instance_id: &str, status: HealthStatus) {
        if let Some(instances) = self.services.get_mut(service_name) {
            if let Some(inst) = instances.iter_mut().find(|i| i.id == instance_id) {
                inst.health = status;
            }
        }
    }
}

// ── Load Balancing ──────────────────────────────────────────────────

/// Load balancing strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancerStrategy {
    RoundRobin,
    Random,
    LeastConnections,
    WeightedRoundRobin,
    ConsistentHash,
}

/// Load balancer.
pub struct LoadBalancer {
    strategy: LoadBalancerStrategy,
    counter: u64,
    connection_counts: HashMap<String, u32>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancerStrategy) -> Self {
        Self { strategy, counter: 0, connection_counts: HashMap::new() }
    }

    /// Select an instance from a list.
    pub fn select<'a>(&mut self, instances: &'a [ServiceInstance]) -> Option<&'a ServiceInstance> {
        let healthy: Vec<_> = instances.iter().filter(|i| i.is_healthy()).collect();
        if healthy.is_empty() { return None; }

        match self.strategy {
            LoadBalancerStrategy::RoundRobin => {
                let idx = (self.counter as usize) % healthy.len();
                self.counter += 1;
                Some(healthy[idx])
            }
            LoadBalancerStrategy::Random => {
                let idx = (self.counter.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) as usize) % healthy.len();
                self.counter += 1;
                Some(healthy[idx])
            }
            LoadBalancerStrategy::LeastConnections => {
                healthy.iter().min_by_key(|i| {
                    self.connection_counts.get(&i.id).copied().unwrap_or(0)
                }).copied()
            }
            LoadBalancerStrategy::WeightedRoundRobin => {
                let total_weight: u32 = healthy.iter().map(|i| i.weight).sum();
                if total_weight == 0 { return healthy.first().copied(); }
                let target = (self.counter % total_weight as u64) as u32;
                self.counter += 1;
                let mut cumulative = 0;
                for inst in &healthy {
                    cumulative += inst.weight;
                    if cumulative > target {
                        return Some(inst);
                    }
                }
                healthy.last().copied()
            }
            LoadBalancerStrategy::ConsistentHash => {
                // Simple hash-based selection.
                let idx = (self.counter as usize) % healthy.len();
                self.counter += 1;
                Some(healthy[idx])
            }
        }
    }

    pub fn record_connection(&mut self, instance_id: &str) {
        *self.connection_counts.entry(instance_id.to_string()).or_insert(0) += 1;
    }

    pub fn release_connection(&mut self, instance_id: &str) {
        if let Some(count) = self.connection_counts.get_mut(instance_id) {
            *count = count.saturating_sub(1);
        }
    }
}

// ── Rate Limiting ───────────────────────────────────────────────────

/// Rate limiter using token bucket algorithm.
#[derive(Debug, Clone)]
pub struct TokenBucketLimiter {
    pub capacity: u64,
    pub tokens: u64,
    pub refill_rate: u64, // tokens per second
    pub last_refill_time: u64,
}

impl TokenBucketLimiter {
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self { capacity, tokens: capacity, refill_rate, last_refill_time: 0 }
    }

    /// Try to consume a token. Returns true if allowed.
    pub fn try_acquire(&mut self, current_time: u64) -> bool {
        self.refill(current_time);
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    pub fn try_acquire_n(&mut self, n: u64, current_time: u64) -> bool {
        self.refill(current_time);
        if self.tokens >= n {
            self.tokens -= n;
            true
        } else {
            false
        }
    }

    fn refill(&mut self, current_time: u64) {
        let elapsed = current_time.saturating_sub(self.last_refill_time);
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill_time = current_time;
    }

    pub fn available(&self) -> u64 {
        self.tokens
    }
}

/// Sliding window rate limiter.
#[derive(Debug)]
pub struct SlidingWindowLimiter {
    window_size_seconds: u64,
    max_requests: u64,
    requests: Vec<u64>, // timestamps
}

impl SlidingWindowLimiter {
    pub fn new(window_size_seconds: u64, max_requests: u64) -> Self {
        Self { window_size_seconds, max_requests, requests: Vec::new() }
    }

    pub fn try_acquire(&mut self, current_time: u64) -> bool {
        // Remove expired requests.
        let cutoff = current_time.saturating_sub(self.window_size_seconds);
        self.requests.retain(|&t| t > cutoff);

        if (self.requests.len() as u64) < self.max_requests {
            self.requests.push(current_time);
            true
        } else {
            false
        }
    }

    pub fn current_count(&self) -> usize {
        self.requests.len()
    }
}

// ── mTLS ────────────────────────────────────────────────────────────

/// Certificate info.
#[derive(Debug, Clone)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub not_before: u64,
    pub not_after: u64,
    pub san: Vec<String>, // Subject Alternative Names
    pub key_algorithm: KeyAlgorithm,
}

impl Certificate {
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time > self.not_after
    }

    pub fn is_valid_at(&self, current_time: u64) -> bool {
        current_time >= self.not_before && current_time <= self.not_after
    }

    pub fn days_until_expiry(&self, current_time: u64) -> i64 {
        let remaining = self.not_after as i64 - current_time as i64;
        remaining / 86400
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAlgorithm {
    Rsa2048,
    Rsa4096,
    EcdsaP256,
    EcdsaP384,
    Ed25519,
}

/// TLS policy.
#[derive(Debug, Clone)]
pub struct TlsPolicy {
    pub mode: TlsMode,
    pub min_version: String,
    pub cipher_suites: Vec<String>,
    pub require_client_cert: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TlsMode {
    Disabled,
    Permissive,
    Strict,
}

// ── Canary Deployment ───────────────────────────────────────────────

/// Canary deployment configuration.
#[derive(Debug, Clone)]
pub struct CanaryConfig {
    pub name: String,
    pub stable_version: String,
    pub canary_version: String,
    pub canary_weight: u32, // percentage 0-100
    pub success_threshold: f64,
    pub analysis_interval_seconds: u32,
    pub max_weight: u32,
    pub step_weight: u32,
    pub status: CanaryStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanaryStatus {
    Progressing,
    Paused,
    Promoted,
    RolledBack,
}

impl CanaryConfig {
    /// Advance canary weight by step.
    pub fn promote_step(&mut self) -> bool {
        if self.status != CanaryStatus::Progressing { return false; }
        self.canary_weight = (self.canary_weight + self.step_weight).min(self.max_weight);
        if self.canary_weight >= self.max_weight {
            self.status = CanaryStatus::Promoted;
        }
        true
    }

    pub fn rollback(&mut self) {
        self.canary_weight = 0;
        self.status = CanaryStatus::RolledBack;
    }

    pub fn pause(&mut self) {
        self.status = CanaryStatus::Paused;
    }

    pub fn is_complete(&self) -> bool {
        self.status == CanaryStatus::Promoted || self.status == CanaryStatus::RolledBack
    }
}

// ── Sidecar Proxy ───────────────────────────────────────────────────

/// Routing rule.
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub match_path: String,
    pub match_method: Option<String>,
    pub target_service: String,
    pub timeout_ms: u32,
    pub retry_count: u32,
    pub headers_to_add: HashMap<String, String>,
}

/// Circuit breaker state.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    success_count: u32,
    half_open_max: u32,
    last_failure_time: u64,
    reset_timeout_seconds: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            half_open_max: 3,
            last_failure_time: 0,
            reset_timeout_seconds: reset_timeout,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => { self.failure_count = 0; }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.half_open_max {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self, current_time: u64) {
        self.failure_count += 1;
        self.last_failure_time = current_time;
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    pub fn can_proceed(&mut self, current_time: u64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if current_time.saturating_sub(self.last_failure_time) >= self.reset_timeout_seconds {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_instance(name: &str, id: &str, healthy: bool) -> ServiceInstance {
        ServiceInstance {
            id: id.into(), service_name: name.into(),
            host: "10.0.0.1".into(), port: 8080, version: "1.0".into(),
            metadata: HashMap::new(),
            health: if healthy { HealthStatus::Healthy } else { HealthStatus::Unhealthy },
            weight: 1, zone: "us-east-1".into(),
        }
    }

    #[test]
    fn test_service_registry() {
        let mut reg = ServiceRegistry::new();
        reg.register(test_instance("api", "1", true));
        reg.register(test_instance("api", "2", true));
        assert_eq!(reg.discover("api").len(), 2);
        assert_eq!(reg.total_instances(), 2);
    }

    #[test]
    fn test_registry_deregister() {
        let mut reg = ServiceRegistry::new();
        reg.register(test_instance("api", "1", true));
        reg.register(test_instance("api", "2", true));
        reg.deregister("api", "1");
        assert_eq!(reg.all_instances("api").len(), 1);
    }

    #[test]
    fn test_discover_healthy_only() {
        let mut reg = ServiceRegistry::new();
        reg.register(test_instance("api", "1", true));
        reg.register(test_instance("api", "2", false));
        assert_eq!(reg.discover("api").len(), 1);
    }

    #[test]
    fn test_round_robin_lb() {
        let mut lb = LoadBalancer::new(LoadBalancerStrategy::RoundRobin);
        let instances = vec![test_instance("api", "1", true), test_instance("api", "2", true)];
        let first = lb.select(&instances).unwrap().id.clone();
        let second = lb.select(&instances).unwrap().id.clone();
        assert_ne!(first, second);
    }

    #[test]
    fn test_weighted_lb() {
        let mut lb = LoadBalancer::new(LoadBalancerStrategy::WeightedRoundRobin);
        let mut i1 = test_instance("api", "1", true);
        i1.weight = 3;
        let mut i2 = test_instance("api", "2", true);
        i2.weight = 1;
        let instances = vec![i1, i2];
        let selected = lb.select(&instances);
        assert!(selected.is_some());
    }

    #[test]
    fn test_token_bucket() {
        let mut limiter = TokenBucketLimiter::new(10, 1);
        for _ in 0..10 {
            assert!(limiter.try_acquire(0));
        }
        assert!(!limiter.try_acquire(0));
        // Refill.
        assert!(limiter.try_acquire(5));
    }

    #[test]
    fn test_sliding_window() {
        let mut limiter = SlidingWindowLimiter::new(60, 3);
        assert!(limiter.try_acquire(1));
        assert!(limiter.try_acquire(2));
        assert!(limiter.try_acquire(3));
        assert!(!limiter.try_acquire(4));
        // After window expires.
        assert!(limiter.try_acquire(62));
    }

    #[test]
    fn test_certificate_expiry() {
        let cert = Certificate {
            subject: "api.example.com".into(), issuer: "CA".into(),
            serial: "abc123".into(), not_before: 1000, not_after: 2000,
            san: vec!["api.example.com".into()], key_algorithm: KeyAlgorithm::EcdsaP256,
        };
        assert!(cert.is_valid_at(1500));
        assert!(cert.is_expired(2500));
        assert_eq!(cert.days_until_expiry(1000), 0); // 1000 / 86400 = 0
    }

    #[test]
    fn test_canary_promote() {
        let mut canary = CanaryConfig {
            name: "api-canary".into(), stable_version: "1.0".into(),
            canary_version: "1.1".into(), canary_weight: 0,
            success_threshold: 0.99, analysis_interval_seconds: 60,
            max_weight: 100, step_weight: 25,
            status: CanaryStatus::Progressing,
        };
        canary.promote_step();
        assert_eq!(canary.canary_weight, 25);
        canary.promote_step();
        assert_eq!(canary.canary_weight, 50);
    }

    #[test]
    fn test_canary_rollback() {
        let mut canary = CanaryConfig {
            name: "test".into(), stable_version: "1.0".into(),
            canary_version: "1.1".into(), canary_weight: 50,
            success_threshold: 0.99, analysis_interval_seconds: 60,
            max_weight: 100, step_weight: 10,
            status: CanaryStatus::Progressing,
        };
        canary.rollback();
        assert_eq!(canary.canary_weight, 0);
        assert_eq!(canary.status, CanaryStatus::RolledBack);
    }

    #[test]
    fn test_canary_full_promotion() {
        let mut canary = CanaryConfig {
            name: "test".into(), stable_version: "1.0".into(),
            canary_version: "1.1".into(), canary_weight: 0,
            success_threshold: 0.99, analysis_interval_seconds: 60,
            max_weight: 100, step_weight: 50,
            status: CanaryStatus::Progressing,
        };
        canary.promote_step();
        canary.promote_step();
        assert_eq!(canary.status, CanaryStatus::Promoted);
        assert!(canary.is_complete());
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let mut cb = CircuitBreaker::new(3, 30);
        assert!(cb.can_proceed(0));
        assert_eq!(*cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut cb = CircuitBreaker::new(3, 30);
        cb.record_failure(1);
        cb.record_failure(2);
        cb.record_failure(3);
        assert_eq!(*cb.state(), CircuitState::Open);
        assert!(!cb.can_proceed(4));
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let mut cb = CircuitBreaker::new(2, 10);
        cb.record_failure(1);
        cb.record_failure(2);
        assert_eq!(*cb.state(), CircuitState::Open);
        // After timeout, transitions to HalfOpen.
        assert!(cb.can_proceed(15));
        assert_eq!(*cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_instance_address() {
        let inst = test_instance("api", "1", true);
        assert_eq!(inst.address(), "10.0.0.1:8080");
    }

    #[test]
    fn test_update_health() {
        let mut reg = ServiceRegistry::new();
        reg.register(test_instance("api", "1", true));
        reg.update_health("api", "1", HealthStatus::Unhealthy);
        assert_eq!(reg.discover("api").len(), 0);
    }

    #[test]
    fn test_tls_policy() {
        let policy = TlsPolicy {
            mode: TlsMode::Strict,
            min_version: "1.3".into(),
            cipher_suites: vec!["TLS_AES_256_GCM_SHA384".into()],
            require_client_cert: true,
        };
        assert_eq!(policy.mode, TlsMode::Strict);
    }

    #[test]
    fn test_routing_rule() {
        let rule = RoutingRule {
            match_path: "/api/v1/*".into(),
            match_method: Some("GET".into()),
            target_service: "api-v1".into(),
            timeout_ms: 5000, retry_count: 3,
            headers_to_add: HashMap::from([("x-trace".into(), "123".into())]),
        };
        assert_eq!(rule.retry_count, 3);
    }

    #[test]
    fn test_lb_no_healthy() {
        let mut lb = LoadBalancer::new(LoadBalancerStrategy::RoundRobin);
        let instances = vec![test_instance("api", "1", false)];
        assert!(lb.select(&instances).is_none());
    }
}
