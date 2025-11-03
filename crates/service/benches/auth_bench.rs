use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use service::auth::service::{AuthService, AuthConfig};
use service::auth::repository::mock::MockAuthRepository;
use service::auth::domain::{RegisterInput, LoginInput};

fn bench_login(c: &mut Criterion) {
    let repo = Arc::new(MockAuthRepository::default());
    let svc = AuthService::new(repo.clone(), AuthConfig { jwt_secret: Some("secret".into()), password_algorithm: "argon2".into() });
    let tid = uuid::Uuid::new_v4();

    // pre-create user outside of the benchmark using a tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(svc.register(RegisterInput { tenant_id: tid, email: "bench@example.com".into(), name: "Bench".into(), password: "Benchmark1".into() }));

    c.bench_function("auth_login_verify", |mut b| {
        b.iter(|| {
            let _ = rt.block_on(svc.login(LoginInput { tenant_id: tid, email: "bench@example.com".into(), password: "Benchmark1".into() })).unwrap();
        });
    });
}

criterion_group!(benches, bench_login);
criterion_main!(benches);