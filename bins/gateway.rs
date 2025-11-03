use dotenvy::dotenv;
use gateway::bootstrap;
use tracing::{ error, info};
use common::utils::logging::init_logging_json;
use uuid::Uuid;

fn init_logging() {
    // 加载 .env（允许使用 RUST_LOG 配置日志级别）
    dotenv().ok();
    // 使用统一的JSON结构化日志初始化
    init_logging_json();
    info!(service = "gateway", event = "logger_init", "tracing subscriber initialized");
}

fn main() {
    init_logging();

    // 生成服务实例上下文（不含敏感信息）
    let service_id = Uuid::new_v4();
    let pid = std::process::id();
    let version = env!("CARGO_PKG_VERSION");

    // Panic 钩子：捕获异常并输出错误日志
    std::panic::set_hook(Box::new({
        let service_id = service_id;
        move |info| {
            error!(
                service = "gateway",
                event = "panic",
                %service_id,
                pid,
                message = %info,
                "unhandled panic occurred"
            );
        }
    }));

    // 服务启动事件
    info!(
        service = "gateway",
        event = "start",
        %service_id,
        pid,
        version,
        "gateway service starting"
    );

    // 关键状态变更：开始委托到业务启动流程
    info!(
        service = "gateway",
        event = "bootstrap",
        config_path = "config.json",
        "delegating startup to gateway::bootstrap"
    );

    // 仅作为启动入口，将执行流程委托给 crates/gateway
    bootstrap::run();

    // 服务停止事件（当 run 返回时记录；正常情况为永不返回）
    info!(
        service = "gateway",
        event = "stop",
        %service_id,
        pid,
        "gateway service stopped"
    );
}