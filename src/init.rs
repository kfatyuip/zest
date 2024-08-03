use crate::{
    config::{init_config, CONFIG},
    zest::T,
};
use async_mutex::Mutex;
use lazy_static::lazy_static;
use log4rs::Handle;
use signal_hook::{consts::SIGHUP, iterator::Signals};
use std::{env::set_current_dir, error::Error};

#[cfg(feature = "log")]
use {
    crate::config::Config,
    log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Logger, Root},
        encode::pattern::PatternEncoder,
    },
    std::{ops::Deref, path::Path},
};

#[cfg(feature = "log")]
const LOG_FORMAT: &str = "[{d(%Y-%m-%dT%H:%M:%SZ)} {h({l})}  zest] {m}\n";

lazy_static! {
    pub static ref LOGGER_HANDLE: Mutex<Option<Handle>> = Mutex::new(None);
}

#[cfg(feature = "log")]
pub async fn build_logger_config<C>(config: C) -> log4rs::Config
where
    C: Deref<Target = Config>,
{
    let mut builder = log4rs::Config::builder();

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
        .target(log4rs::append::console::Target::Stdout)
        .build();

    let stderr = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
        .target(log4rs::append::console::Target::Stderr)
        .build();

    let logging = &config.logging.clone().unwrap_or_default();
    builder = if let Some(access_log) = &logging.access_log {
        let access_log_path = Path::new(&access_log);
        std::fs::File::create(access_log_path).unwrap();
        builder.appender(
            Appender::builder().build(
                "logfile_access",
                Box::new(
                    FileAppender::builder()
                        .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
                        .build(access_log_path)
                        .unwrap(),
                ),
            ),
        )
    } else {
        builder.appender(Appender::builder().build("logfile_access", Box::new(stdout)))
    };

    builder = if let Some(error_log) = &logging.error_log {
        let error_log_path = Path::new(&error_log);
        std::fs::File::create(error_log_path).unwrap();
        builder.appender(
            Appender::builder().build(
                "logfile_error",
                Box::new(
                    FileAppender::builder()
                        .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
                        .build(error_log_path)
                        .unwrap(),
                ),
            ),
        )
    } else {
        builder.appender(Appender::builder().build("logfile_error", Box::new(stderr)))
    };

    builder
        .logger(
            Logger::builder()
                .appender("logfile_access")
                .additive(false)
                .build("access", log::LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("logfile_error")
                .additive(false)
                .build("error", log::LevelFilter::Error),
        )
        .build(Root::builder().build(log::LevelFilter::Off))
        .unwrap()
}

#[cfg(feature = "log")]
pub async fn init_logger<C>(config: C)
where
    C: Deref<Target = Config>,
{
    let config = build_logger_config(config).await;
    *LOGGER_HANDLE.lock().await = Some(log4rs::init_config(config).unwrap())
}

pub async fn init_signal() -> Result<(), Box<dyn Error>> {
    let mut signals = Signals::new([SIGHUP])?;

    tokio::spawn(async move {
        for sig in signals.forever() {
            if sig == SIGHUP {
                let config: crate::config::Config = init_config();

                let mut _c = CONFIG.try_write().unwrap();
                *_c = config.clone();
                drop(_c);

                set_current_dir(config.clone().server.root).unwrap();

                #[cfg(feature = "log")]
                {
                    let mut _handle = LOGGER_HANDLE.lock().await;
                    if let Some(handle) = _handle.take() {
                        handle.set_config(build_logger_config(&config.clone()).await);
                    }
                }

                let mut t = T.write().await;
                *t = None;
                drop(t);
            }
        }
    });

    Ok(())
}
