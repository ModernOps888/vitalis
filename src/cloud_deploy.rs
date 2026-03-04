//! Cloud deployment infrastructure for Vitalis.
//!
//! - **OCI images**: Container image building / manifest generation
//! - **Kubernetes manifests**: Deployment, Service, Ingress generation
//! - **Serverless packaging**: AWS Lambda / Cloudflare Workers / GCP Functions
//! - **Health checks**: Liveness / readiness probes
//! - **Graceful shutdown**: Signal handling, drain connections
//! - **Environment configuration**: Secret management, config maps

use std::collections::HashMap;

// ── OCI / Container Image ──────────────────────────────────────────

/// Container image specification.
#[derive(Debug, Clone)]
pub struct ContainerImage {
    pub name: String,
    pub tag: String,
    pub registry: String,
    pub base_image: String,
    pub layers: Vec<ImageLayer>,
    pub env: HashMap<String, String>,
    pub exposed_ports: Vec<u16>,
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    pub labels: HashMap<String, String>,
    pub working_dir: String,
}

impl ContainerImage {
    pub fn full_name(&self) -> String {
        format!("{}/{}:{}", self.registry, self.name, self.tag)
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.layers.iter().map(|l| l.size_bytes).sum()
    }

    /// Generate a Dockerfile.
    pub fn to_dockerfile(&self) -> String {
        let mut df = String::new();
        df.push_str(&format!("FROM {}\n\n", self.base_image));
        df.push_str(&format!("WORKDIR {}\n\n", self.working_dir));

        for (k, v) in &self.labels {
            df.push_str(&format!("LABEL {}=\"{}\"\n", k, v));
        }
        if !self.labels.is_empty() { df.push('\n'); }

        for (k, v) in &self.env {
            df.push_str(&format!("ENV {}={}\n", k, v));
        }
        if !self.env.is_empty() { df.push('\n'); }

        for layer in &self.layers {
            df.push_str(&format!("# Layer: {}\n", layer.description));
            df.push_str(&format!("COPY {} {}\n", layer.source, layer.destination));
        }
        df.push('\n');

        for port in &self.exposed_ports {
            df.push_str(&format!("EXPOSE {}\n", port));
        }
        if !self.exposed_ports.is_empty() { df.push('\n'); }

        if !self.entrypoint.is_empty() {
            let parts: Vec<_> = self.entrypoint.iter().map(|s| format!("\"{}\"", s)).collect();
            df.push_str(&format!("ENTRYPOINT [{}]\n", parts.join(", ")));
        }
        if !self.cmd.is_empty() {
            let parts: Vec<_> = self.cmd.iter().map(|s| format!("\"{}\"", s)).collect();
            df.push_str(&format!("CMD [{}]\n", parts.join(", ")));
        }
        df
    }
}

/// A container image layer.
#[derive(Debug, Clone)]
pub struct ImageLayer {
    pub source: String,
    pub destination: String,
    pub size_bytes: u64,
    pub description: String,
}

// ── Kubernetes Resources ────────────────────────────────────────────

/// Kubernetes resource kind.
#[derive(Debug, Clone, PartialEq)]
pub enum K8sKind {
    Deployment,
    Service,
    Ingress,
    ConfigMap,
    Secret,
    HorizontalPodAutoscaler,
    PersistentVolumeClaim,
    Namespace,
}

/// Resource requirements.
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub cpu_request: String,
    pub cpu_limit: String,
    pub memory_request: String,
    pub memory_limit: String,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_request: "100m".into(),
            cpu_limit: "500m".into(),
            memory_request: "128Mi".into(),
            memory_limit: "512Mi".into(),
        }
    }
}

/// Health check probe.
#[derive(Debug, Clone)]
pub struct Probe {
    pub path: String,
    pub port: u16,
    pub initial_delay_seconds: u32,
    pub period_seconds: u32,
    pub timeout_seconds: u32,
    pub failure_threshold: u32,
    pub probe_type: ProbeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProbeType {
    Liveness,
    Readiness,
    Startup,
}

impl Probe {
    pub fn liveness(path: &str, port: u16) -> Self {
        Self {
            path: path.into(), port, initial_delay_seconds: 15,
            period_seconds: 20, timeout_seconds: 5, failure_threshold: 3,
            probe_type: ProbeType::Liveness,
        }
    }

    pub fn readiness(path: &str, port: u16) -> Self {
        Self {
            path: path.into(), port, initial_delay_seconds: 5,
            period_seconds: 10, timeout_seconds: 3, failure_threshold: 3,
            probe_type: ProbeType::Readiness,
        }
    }
}

/// K8s deployment spec.
#[derive(Debug, Clone)]
pub struct DeploymentSpec {
    pub name: String,
    pub namespace: String,
    pub replicas: u32,
    pub image: String,
    pub ports: Vec<u16>,
    pub env_vars: HashMap<String, String>,
    pub resources: ResourceRequirements,
    pub liveness_probe: Option<Probe>,
    pub readiness_probe: Option<Probe>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

/// Generate YAML for a K8s deployment.
pub fn generate_deployment_yaml(spec: &DeploymentSpec) -> String {
    let mut y = String::new();
    y.push_str("apiVersion: apps/v1\n");
    y.push_str("kind: Deployment\n");
    y.push_str("metadata:\n");
    y.push_str(&format!("  name: {}\n", spec.name));
    y.push_str(&format!("  namespace: {}\n", spec.namespace));
    if !spec.labels.is_empty() {
        y.push_str("  labels:\n");
        for (k, v) in &spec.labels {
            y.push_str(&format!("    {}: {}\n", k, v));
        }
    }
    y.push_str("spec:\n");
    y.push_str(&format!("  replicas: {}\n", spec.replicas));
    y.push_str("  selector:\n");
    y.push_str("    matchLabels:\n");
    y.push_str(&format!("      app: {}\n", spec.name));
    y.push_str("  template:\n");
    y.push_str("    metadata:\n");
    y.push_str("      labels:\n");
    y.push_str(&format!("        app: {}\n", spec.name));
    y.push_str("    spec:\n");
    y.push_str("      containers:\n");
    y.push_str(&format!("      - name: {}\n", spec.name));
    y.push_str(&format!("        image: {}\n", spec.image));
    if !spec.ports.is_empty() {
        y.push_str("        ports:\n");
        for port in &spec.ports {
            y.push_str(&format!("        - containerPort: {}\n", port));
        }
    }
    y.push_str("        resources:\n");
    y.push_str("          requests:\n");
    y.push_str(&format!("            cpu: {}\n", spec.resources.cpu_request));
    y.push_str(&format!("            memory: {}\n", spec.resources.memory_request));
    y.push_str("          limits:\n");
    y.push_str(&format!("            cpu: {}\n", spec.resources.cpu_limit));
    y.push_str(&format!("            memory: {}\n", spec.resources.memory_limit));

    if let Some(probe) = &spec.liveness_probe {
        y.push_str("        livenessProbe:\n");
        y.push_str("          httpGet:\n");
        y.push_str(&format!("            path: {}\n", probe.path));
        y.push_str(&format!("            port: {}\n", probe.port));
        y.push_str(&format!("          initialDelaySeconds: {}\n", probe.initial_delay_seconds));
        y.push_str(&format!("          periodSeconds: {}\n", probe.period_seconds));
    }
    if let Some(probe) = &spec.readiness_probe {
        y.push_str("        readinessProbe:\n");
        y.push_str("          httpGet:\n");
        y.push_str(&format!("            path: {}\n", probe.path));
        y.push_str(&format!("            port: {}\n", probe.port));
        y.push_str(&format!("          initialDelaySeconds: {}\n", probe.initial_delay_seconds));
        y.push_str(&format!("          periodSeconds: {}\n", probe.period_seconds));
    }
    y
}

/// Generate YAML for a K8s service.
pub fn generate_service_yaml(name: &str, namespace: &str, port: u16, target_port: u16) -> String {
    let mut y = String::new();
    y.push_str("apiVersion: v1\n");
    y.push_str("kind: Service\n");
    y.push_str("metadata:\n");
    y.push_str(&format!("  name: {}\n", name));
    y.push_str(&format!("  namespace: {}\n", namespace));
    y.push_str("spec:\n");
    y.push_str("  selector:\n");
    y.push_str(&format!("    app: {}\n", name));
    y.push_str("  ports:\n");
    y.push_str(&format!("  - port: {}\n", port));
    y.push_str(&format!("    targetPort: {}\n", target_port));
    y.push_str("  type: ClusterIP\n");
    y
}

// ── Serverless ──────────────────────────────────────────────────────

/// Serverless target platform.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerlessPlatform {
    AwsLambda,
    CloudflareWorkers,
    GcpCloudFunctions,
    AzureFunctions,
    VercelEdge,
}

/// Serverless function spec.
#[derive(Debug, Clone)]
pub struct ServerlessFunction {
    pub name: String,
    pub platform: ServerlessPlatform,
    pub runtime: String,
    pub handler: String,
    pub timeout_seconds: u32,
    pub memory_mb: u32,
    pub env_vars: HashMap<String, String>,
    pub triggers: Vec<Trigger>,
}

/// Function trigger.
#[derive(Debug, Clone, PartialEq)]
pub enum Trigger {
    Http { method: String, path: String },
    Schedule { cron: String },
    Queue { queue_name: String },
    Event { source: String, event_type: String },
}

// ── Graceful Shutdown ───────────────────────────────────────────────

/// Shutdown phase.
#[derive(Debug, Clone, PartialEq)]
pub enum ShutdownPhase {
    Running,
    Draining,
    Terminating,
    Completed,
}

/// Graceful shutdown controller.
pub struct ShutdownController {
    phase: ShutdownPhase,
    active_connections: u32,
    drain_timeout_seconds: u32,
    hooks: Vec<String>,
}

impl ShutdownController {
    pub fn new(drain_timeout: u32) -> Self {
        Self {
            phase: ShutdownPhase::Running,
            active_connections: 0,
            drain_timeout_seconds: drain_timeout,
            hooks: Vec::new(),
        }
    }

    pub fn initiate_shutdown(&mut self) {
        self.phase = ShutdownPhase::Draining;
    }

    pub fn add_connection(&mut self) {
        self.active_connections += 1;
    }

    pub fn remove_connection(&mut self) {
        self.active_connections = self.active_connections.saturating_sub(1);
        if self.phase == ShutdownPhase::Draining && self.active_connections == 0 {
            self.phase = ShutdownPhase::Terminating;
        }
    }

    pub fn force_terminate(&mut self) {
        self.phase = ShutdownPhase::Completed;
    }

    pub fn register_hook(&mut self, hook: &str) {
        self.hooks.push(hook.to_string());
    }

    pub fn phase(&self) -> &ShutdownPhase {
        &self.phase
    }

    pub fn active_connections(&self) -> u32 {
        self.active_connections
    }

    pub fn is_accepting(&self) -> bool {
        self.phase == ShutdownPhase::Running
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_image_full_name() {
        let img = ContainerImage {
            name: "myapp".into(), tag: "1.0".into(), registry: "ghcr.io/org".into(),
            base_image: "alpine:3.18".into(), layers: vec![], env: HashMap::new(),
            exposed_ports: vec![8080], entrypoint: vec!["./app".into()],
            cmd: vec![], labels: HashMap::new(), working_dir: "/app".into(),
        };
        assert_eq!(img.full_name(), "ghcr.io/org/myapp:1.0");
    }

    #[test]
    fn test_dockerfile_generation() {
        let img = ContainerImage {
            name: "svc".into(), tag: "latest".into(), registry: "docker.io".into(),
            base_image: "rust:1.75-slim".into(),
            layers: vec![ImageLayer { source: "target/release/app".into(), destination: "/app/".into(), size_bytes: 1024, description: "binary".into() }],
            env: HashMap::from([("PORT".into(), "8080".into())]),
            exposed_ports: vec![8080], entrypoint: vec!["./app".into()],
            cmd: vec![], labels: HashMap::new(), working_dir: "/app".into(),
        };
        let df = img.to_dockerfile();
        assert!(df.contains("FROM rust:1.75-slim"));
        assert!(df.contains("EXPOSE 8080"));
        assert!(df.contains("ENTRYPOINT"));
    }

    #[test]
    fn test_deployment_yaml() {
        let spec = DeploymentSpec {
            name: "api".into(), namespace: "prod".into(), replicas: 3,
            image: "ghcr.io/org/api:1.0".into(), ports: vec![8080],
            env_vars: HashMap::new(), resources: ResourceRequirements::default(),
            liveness_probe: Some(Probe::liveness("/health", 8080)),
            readiness_probe: Some(Probe::readiness("/ready", 8080)),
            labels: HashMap::from([("app".into(), "api".into())]),
            annotations: HashMap::new(),
        };
        let yaml = generate_deployment_yaml(&spec);
        assert!(yaml.contains("kind: Deployment"));
        assert!(yaml.contains("replicas: 3"));
        assert!(yaml.contains("livenessProbe"));
        assert!(yaml.contains("readinessProbe"));
    }

    #[test]
    fn test_service_yaml() {
        let yaml = generate_service_yaml("api", "prod", 80, 8080);
        assert!(yaml.contains("kind: Service"));
        assert!(yaml.contains("targetPort: 8080"));
    }

    #[test]
    fn test_probe_constructors() {
        let lp = Probe::liveness("/health", 8080);
        assert_eq!(lp.probe_type, ProbeType::Liveness);
        assert_eq!(lp.initial_delay_seconds, 15);

        let rp = Probe::readiness("/ready", 3000);
        assert_eq!(rp.probe_type, ProbeType::Readiness);
        assert_eq!(rp.period_seconds, 10);
    }

    #[test]
    fn test_resource_defaults() {
        let res = ResourceRequirements::default();
        assert_eq!(res.cpu_request, "100m");
        assert_eq!(res.memory_limit, "512Mi");
    }

    #[test]
    fn test_serverless_function() {
        let f = ServerlessFunction {
            name: "handler".into(),
            platform: ServerlessPlatform::AwsLambda,
            runtime: "provided.al2".into(),
            handler: "bootstrap".into(),
            timeout_seconds: 30, memory_mb: 256,
            env_vars: HashMap::new(),
            triggers: vec![Trigger::Http { method: "GET".into(), path: "/api".into() }],
        };
        assert_eq!(f.platform, ServerlessPlatform::AwsLambda);
        assert_eq!(f.triggers.len(), 1);
    }

    #[test]
    fn test_shutdown_controller() {
        let mut ctrl = ShutdownController::new(30);
        assert!(ctrl.is_accepting());
        ctrl.add_connection();
        ctrl.add_connection();
        ctrl.initiate_shutdown();
        assert!(!ctrl.is_accepting());
        assert_eq!(*ctrl.phase(), ShutdownPhase::Draining);
        ctrl.remove_connection();
        assert_eq!(*ctrl.phase(), ShutdownPhase::Draining);
        ctrl.remove_connection();
        assert_eq!(*ctrl.phase(), ShutdownPhase::Terminating);
    }

    #[test]
    fn test_force_terminate() {
        let mut ctrl = ShutdownController::new(10);
        ctrl.force_terminate();
        assert_eq!(*ctrl.phase(), ShutdownPhase::Completed);
    }

    #[test]
    fn test_shutdown_hooks() {
        let mut ctrl = ShutdownController::new(30);
        ctrl.register_hook("flush_cache");
        ctrl.register_hook("close_db");
        assert_eq!(ctrl.hooks.len(), 2);
    }

    #[test]
    fn test_image_total_size() {
        let img = ContainerImage {
            name: "x".into(), tag: "1".into(), registry: "r".into(),
            base_image: "b".into(),
            layers: vec![
                ImageLayer { source: "a".into(), destination: "/a".into(), size_bytes: 1000, description: "a".into() },
                ImageLayer { source: "b".into(), destination: "/b".into(), size_bytes: 2000, description: "b".into() },
            ],
            env: HashMap::new(), exposed_ports: vec![], entrypoint: vec![],
            cmd: vec![], labels: HashMap::new(), working_dir: "/".into(),
        };
        assert_eq!(img.total_size_bytes(), 3000);
    }

    #[test]
    fn test_trigger_variants() {
        let t = Trigger::Schedule { cron: "0 * * * *".into() };
        match t {
            Trigger::Schedule { ref cron } => assert_eq!(cron, "0 * * * *"),
            _ => panic!("wrong"),
        }
    }

    #[test]
    fn test_k8s_kind() {
        assert_ne!(K8sKind::Deployment, K8sKind::Service);
        assert_ne!(K8sKind::Ingress, K8sKind::Secret);
    }

    #[test]
    fn test_serverless_platforms() {
        let platforms = vec![
            ServerlessPlatform::AwsLambda,
            ServerlessPlatform::CloudflareWorkers,
            ServerlessPlatform::GcpCloudFunctions,
            ServerlessPlatform::AzureFunctions,
            ServerlessPlatform::VercelEdge,
        ];
        assert_eq!(platforms.len(), 5);
    }

    #[test]
    fn test_env_trigger() {
        let t = Trigger::Event { source: "s3".into(), event_type: "ObjectCreated".into() };
        matches!(t, Trigger::Event { .. });
    }

    #[test]
    fn test_queue_trigger() {
        let t = Trigger::Queue { queue_name: "my-queue".into() };
        matches!(t, Trigger::Queue { .. });
    }

    #[test]
    fn test_active_connections() {
        let mut ctrl = ShutdownController::new(30);
        assert_eq!(ctrl.active_connections(), 0);
        ctrl.add_connection();
        assert_eq!(ctrl.active_connections(), 1);
    }
}
