use crate::config::{init_config, CONFIG, DEFAULT_CACHE_INTERVAL, DEFAULT_CONFIG};
use async_mutex::Mutex;
use async_rwlock::RwLock;
use lazy_static::lazy_static;
use log4rs::Handle;
use lru::LruCache;
use signal_hook::{
    consts::{SIGHUP, SIGINT},
    iterator::Signals,
};
use std::{
    fs::remove_file,
    num::NonZero,
    path::PathBuf,
    process,
    {env::set_current_dir, io, num::NonZeroUsize, sync::Arc, thread},
};

#[cfg(feature = "log")]
use {
    crate::config::Config,
    log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Logger, Root},
        encode::pattern::PatternEncoder,
    },
    std::{
        fs::{create_dir_all, File},
        ops::Deref,
        path::Path,
    },
};

lazy_static! {
    pub static ref PID_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub static ref T: Arc<RwLock<Option<i32>>> = Arc::new(RwLock::new(None));
    pub static ref LOGGER_HANDLE: Mutex<Option<Handle>> = Mutex::new(None);
}

#[cfg(feature = "lru_cache")]
lazy_static! {
    pub static ref INDEX_CACHE: RwLock<LruCache<String, String>> = {
        let cache = LruCache::new(
            NonZeroUsize::new(
                DEFAULT_CONFIG
                    .server
                    .cache
                    .clone()
                    .unwrap_or_default()
                    .index_capacity
                    .unwrap_or(16),
            )
            .unwrap(),
        );
        RwLock::new(cache)
    };
    pub static ref FILE_CACHE: RwLock<LruCache<String, Vec<u8>>> = {
        let cache = LruCache::new(
            NonZeroUsize::new(
                DEFAULT_CONFIG
                    .server
                    .cache
                    .clone()
                    .unwrap_or_default()
                    .file_capacity
                    .unwrap_or(32),
            )
            .unwrap(),
        );
        RwLock::new(cache)
    };
}

pub const DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

#[cfg(feature = "log")]
const LOG_FORMAT: &str = "[{d(%Y-%m-%dT%H:%M:%SZ)} {h({l})}  zest] {m}\n";

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
        let parent = access_log_path.parent().unwrap();
        if !parent.exists() {
            create_dir_all(parent).unwrap();
        }
        File::create(access_log_path).unwrap();
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
        let parent = error_log_path.parent().unwrap();
        if !parent.exists() {
            create_dir_all(parent).unwrap();
        }
        File::create(error_log_path).unwrap();
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
pub async fn init_logger<C>(config: C) -> Result<(), log::SetLoggerError>
where
    C: Deref<Target = Config>,
{
    let config = build_logger_config(config).await;
    *LOGGER_HANDLE.lock().await = Some(log4rs::init_config(config)?);

    Ok(())
}

pub async fn init_signal() -> io::Result<()> {
    let mut signals = Signals::new([SIGHUP, SIGINT])?;

    tokio::spawn(async move {
        for sig in signals.forever() {
            if sig == SIGHUP {
                let config: crate::config::Config = init_config();

                CONFIG.store(Arc::new(config.clone()));

                set_current_dir(config.clone().server.root).unwrap();

                #[cfg(feature = "log")]
                {
                    let mut _handle = LOGGER_HANDLE.lock().await;
                    if let Some(handle) = _handle.take() {
                        handle.set_config(build_logger_config(&config.clone()).await);
                    }
                }

                let cache = config.server.cache.unwrap_or_default();
                let (index_capacity, file_capacity) =
                    (cache.index_capacity.unwrap(), cache.file_capacity.unwrap());

                INDEX_CACHE
                    .write()
                    .await
                    .resize(NonZero::new(index_capacity).unwrap());

                FILE_CACHE
                    .write()
                    .await
                    .resize(NonZero::new(file_capacity).unwrap());

                let mut t = T.write().await;
                *t = None;
                drop(t);
            } else if sig == SIGINT {
                remove_file(PID_FILE.try_lock().unwrap().clone().unwrap().as_path()).unwrap();
                process::exit(0);
            }
        }
    });

    Ok(())
}

#[cfg(feature = "lru_cache")]
pub async fn init_cache() -> io::Result<()> {
    let config = CONFIG.load();
    let interval = config
        .server
        .cache
        .clone()
        .unwrap_or_default()
        .interval
        .unwrap_or(*DEFAULT_CACHE_INTERVAL);

    let mut _b: bool = false;
    tokio::spawn(async move {
        loop {
            if _b {
                if let Some(mut index_cache) = INDEX_CACHE.try_write() {
                    index_cache.clear();
                }
            } else if let Some(mut file_cache) = FILE_CACHE.try_write() {
                file_cache.clear();
            }
            _b = !_b;
            thread::sleep(interval);
        }
    });

    Ok(())
}
