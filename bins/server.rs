use dotenvy::dotenv;
use tracing::{error, info};
use uuid::Uuid;

fn init_logging() {
    // 提前加载 .env，使得 RUST_LOG 等环境变量生效
    dotenv().ok();
    // 复用公共日志初始化工具，统一日志格式与级别处理
    common::utils::logging::init_logging_default();
    info!(service = "server", event = "logger_init", "tracing subscriber initialized");
}

fn main() -> std::process::ExitCode {
    init_logging();

    // 基础服务上下文（不含敏感信息）
    let service_id = Uuid::new_v4();
    let pid = std::process::id();
    let version = env!("CARGO_PKG_VERSION");

    // Panic 钩子：捕获异常并输出错误日志，便于排查问题
    std::panic::set_hook(Box::new({
        let service_id = service_id;
        move |info| {
            error!(
                service = "server",
                event = "panic",
                %service_id,
                pid,
                message = %info,
                "unhandled panic occurred"
            );
        }
    }));

    // 读取线程配置（优先 config.toml，其次环境变量 TOKIO_WORKER_THREADS）
    let worker_threads = match configs::AppConfig::load_and_validate() {
        Ok(cfg) => cfg.server.worker_threads,
        Err(_) => std::env::var("TOKIO_WORKER_THREADS").ok().and_then(|v| v.parse::<usize>().ok()),
    };

    // 构建 Tokio 运行时（允许根据配置调整线程数）
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(w) = worker_threads { builder.worker_threads(w); }

    let rt = match builder.build() {
        Ok(rt) => rt,
        Err(e) => {
            error!(service = "server", event = "runtime_build_failed", error = %e, "failed to build tokio runtime");
            return std::process::ExitCode::FAILURE;
        }
    };

    // 服务启动事件
    info!(
        service = "server",
        event = "start",
        %service_id,
        pid,
        version,
        threads = worker_threads.unwrap_or_default(),
        "server service starting"
    );

    // 在独立任务中运行服务，并监听 Ctrl+C 优雅停机
    let exit_code = rt.block_on(async move {
        let server_task = tokio::spawn(async move {
            if let Err(e) = server::run().await {
                error!(service = "server", event = "run_failed", error = %e, "server::run returned error");
                Err(e)
            } else {
                Ok(())
            }
        });

        tokio::select! {
            res = server_task => {
                match res {
                    Ok(Ok(())) => {
                        info!(service = "server", event = "stop", %service_id, pid, "server stopped normally");
                        std::process::ExitCode::SUCCESS
                    }
                    Ok(Err(_)) => {
                        // 错误已在上面记录
                        std::process::ExitCode::FAILURE
                    }
                    Err(e) => {
                        error!(service = "server", event = "task_join_error", error = %e, "server task join error");
                        std::process::ExitCode::FAILURE
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!(service = "server", event = "shutdown_signal", %service_id, pid, "received Ctrl+C, shutting down");
                // 当前 server::run 不支持外部优雅停机信号，这里中止任务以尽快退出
                // 如需更优雅的停机，应在服务内部支持 with_graceful_shutdown。
                std::process::ExitCode::SUCCESS
            }
        }
    });

    exit_code
}
