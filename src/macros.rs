#[cfg(feature = "log")]
#[macro_export]
macro_rules! info {
    ($fmt:expr) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Info)
                .target("access")
                .args(format_args!($fmt))
                .build(),
        );
    };
    ($fmt:expr, $($args:tt)*) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Info)
                .target("access")
                .args(format_args!($fmt, $($args)*))
                .build(),
        );
    };
}

#[cfg(feature = "log")]
#[macro_export]
macro_rules! error {
    ($fmt:expr) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Error)
                .target("error")
                .args(format_args!($fmt))
                .build(),
        );
    };
    ($fmt:expr, $($args:tt)*) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Error)
                .target("error")
                .args(format_args!($fmt, $($args)*))
                .build(),
        );
    };
}

#[cfg(feature = "log")]
#[macro_export]
macro_rules! warn {
    ($fmt:expr) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Warn)
                .target("access")
                .args(format_args!($fmt))
                .build(),
        );
    };
    ($fmt:expr, $($args:tt)*) => {
        logger().log(
            &log::Record::builder()
                .level(log::Level::Warn)
                .target("access")
                .args(format_args!($fmt, $($args)*))
                .build(),
        );
    };
}
