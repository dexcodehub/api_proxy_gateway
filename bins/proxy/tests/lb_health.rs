use pingora_load_balancing::{health_check, selection::RoundRobin, LoadBalancer};
use std::net::SocketAddr;
use std::time::Duration;

#[test]
fn round_robin_selects_upstream() {
    let peers: Vec<SocketAddr> = vec![
        "127.0.0.1:8080".parse().unwrap(),
        "127.0.0.1:8080".parse().unwrap(),
    ];
    let lb = LoadBalancer::<RoundRobin>::try_from_iter(peers.clone()).expect("create lb");
    let sel = lb.select(b"", 256);
    assert!(sel.is_some(), "expected some upstream to be selected");
}

#[test]
fn tcp_health_check_frequency_configured() {
    let peers: Vec<SocketAddr> = vec!["127.0.0.1:8080".parse().unwrap()];
    let mut lb = LoadBalancer::<RoundRobin>::try_from_iter(peers).expect("create lb");
    let tcp_hc = health_check::TcpHealthCheck::new();
    lb.set_health_check(tcp_hc);
    lb.health_check_frequency = Some(Duration::from_secs(1));
    assert_eq!(lb.health_check_frequency, Some(Duration::from_secs(1)));
}