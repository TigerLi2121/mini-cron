use time::macros::{format_description, offset};
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::time::OffsetTime;

pub async fn init_log() {
    // 输出到文件中
    let file_appender = rolling::never("logs", "app.log");
    let (non_blocking_appender, _guard) = non_blocking(file_appender);
    // 日期格式化
    let timer = OffsetTime::new(
        offset!(+8),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    );
    let f = fmt().event_format(
        fmt::format()
            .with_ansi(false)
            .with_timer(timer)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_line_number(true)
            .compact(),
    );
    // release 输出到日志文件
    if cfg!(debug_assertions) {
        f.init();
    } else {
        f.with_writer(non_blocking_appender).init();
    }
}
