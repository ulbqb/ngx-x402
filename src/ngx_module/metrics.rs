use prometheus::{Histogram, HistogramOpts, IntCounter, Registry, TextEncoder};
use std::sync::OnceLock;

static METRICS: OnceLock<X402Metrics> = OnceLock::new();

pub struct X402Metrics {
    pub requests_total: IntCounter,
    pub verification_attempts: IntCounter,
    pub verification_success: IntCounter,
    pub verification_failed: IntCounter,
    pub responses_402: IntCounter,
    pub facilitator_errors: IntCounter,
    pub verification_duration: Histogram,
    pub payment_amount: Histogram,
    registry: Registry,
}

impl X402Metrics {
    pub fn get() -> &'static Self {
        METRICS.get_or_init(|| {
            let registry = Registry::new();

            let requests_total =
                IntCounter::new("x402_requests_total", "Total requests processed").unwrap();
            let verification_attempts =
                IntCounter::new("x402_payment_verifications_total", "Verification attempts")
                    .unwrap();
            let verification_success = IntCounter::new(
                "x402_payment_verifications_success_total",
                "Successful verifications",
            )
            .unwrap();
            let verification_failed = IntCounter::new(
                "x402_payment_verifications_failed_total",
                "Failed verifications",
            )
            .unwrap();
            let responses_402 =
                IntCounter::new("x402_responses_402_total", "402 responses sent").unwrap();
            let facilitator_errors =
                IntCounter::new("x402_facilitator_errors_total", "Facilitator errors").unwrap();
            let verification_duration = Histogram::with_opts(
                HistogramOpts::new("x402_verification_duration_seconds", "Verification latency")
                    .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            )
            .unwrap();
            let payment_amount = Histogram::with_opts(
                HistogramOpts::new("x402_payment_amount", "Payment amount")
                    .buckets(vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0]),
            )
            .unwrap();

            registry.register(Box::new(requests_total.clone())).ok();
            registry
                .register(Box::new(verification_attempts.clone()))
                .ok();
            registry
                .register(Box::new(verification_success.clone()))
                .ok();
            registry
                .register(Box::new(verification_failed.clone()))
                .ok();
            registry.register(Box::new(responses_402.clone())).ok();
            registry.register(Box::new(facilitator_errors.clone())).ok();
            registry
                .register(Box::new(verification_duration.clone()))
                .ok();
            registry.register(Box::new(payment_amount.clone())).ok();

            Self {
                requests_total,
                verification_attempts,
                verification_success,
                verification_failed,
                responses_402,
                facilitator_errors,
                verification_duration,
                payment_amount,
                registry,
            }
        })
    }

    pub fn record_request(&self) {
        self.requests_total.inc();
    }

    pub fn record_verification_attempt(&self) {
        self.verification_attempts.inc();
    }

    pub fn record_verification_success(&self) {
        self.verification_success.inc();
    }

    pub fn record_verification_failed(&self) {
        self.verification_failed.inc();
    }

    pub fn record_402_response(&self) {
        self.responses_402.inc();
    }

    pub fn record_facilitator_error(&self) {
        self.facilitator_errors.inc();
    }

    pub fn record_verification_duration(&self, duration: f64) {
        self.verification_duration.observe(duration);
    }

    pub fn record_payment_amount(&self, amount: f64) {
        self.payment_amount.observe(amount);
    }
}

pub fn collect_metrics() -> String {
    let metrics = X402Metrics::get();
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry.gather();
    let mut buffer = String::new();
    encoder.encode_utf8(&metric_families, &mut buffer).ok();
    if buffer.trim().is_empty() {
        buffer = "# HELP x402_module_info Module information\n# TYPE x402_module_info gauge\nx402_module_info 1\n".to_string();
    }
    buffer
}
