use pingora_load_balancing::{selection::RoundRobin, LoadBalancer};

#[test]
fn round_robin_alternates_between_backends() {
    let backends: Vec<std::net::SocketAddr> = vec![
        "127.0.0.1:8080".parse().expect("parse addr"),
        "127.0.0.1:8081".parse().expect("parse addr"),
    ];

    let lb = LoadBalancer::<RoundRobin>::try_from_iter(backends).expect("lb");

    let a = format!("{:?}", lb.select(b"", 256).unwrap().addr);
    let b = format!("{:?}", lb.select(b"", 256).unwrap().addr);
    let c = format!("{:?}", lb.select(b"", 256).unwrap().addr);

    assert_ne!(a, b, "round robin should rotate to a different backend");
    assert_eq!(c, a, "round robin should cycle back to the first backend");
}