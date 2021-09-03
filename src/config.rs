use std::sync::{Arc, Mutex};

use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub filter_name: String,
    pub enable_filter: bool,
    pub enable_db: bool,
    pub jobs: usize,
    pub sql: String,
    pub download_start: usize,
}

fn init_log() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build(format!(
            "log/log_{}.log",
            chrono::Utc::now().timestamp_millis()
        ))
        .unwrap();

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appender("stdout")
                // .appender("file")
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();
}

pub fn init_config() {
    // let r = log4rs::init_file("config/log4rs.yaml", Default::default());

    // if r.is_err() {
    //     let _ = log4rs::init_file("rust/config/log4rs.yaml", Default::default());
    // }
    init_log();
}

impl Config {
    pub fn get_instance() -> Arc<Mutex<Config>> {
        static mut CONFIG: Option<Arc<Mutex<Config>>> = None;

        unsafe {
            // Rust中使用可变静态变量都是unsafe的
            CONFIG
                .get_or_insert_with(|| {
                    init_config();
                    // 初始化单例对象的代码
                    Arc::new(Mutex::new(Config {
                        filter_name: "".to_string(),
                        enable_filter: false,
                        enable_db: false,
                        jobs: 1,
                        sql: "192.168.2.25:27017".to_string(),
                        download_start: 1,
                    }))
                })
                .clone()
        }
    }

    pub fn set_filter_name(&mut self, name: &str) {
        self.filter_name = name.to_string();
    }

    pub fn set_sql(&mut self, sql: &str) {
        self.sql = sql.to_string();
    }

    pub fn set_enable_db(&mut self, db: bool) {
        self.enable_db = db;
    }

    pub fn set_enable_filter(&mut self, filter: bool) {
        self.enable_filter = filter;
    }

    pub fn set_jobs(&mut self, jobs: usize) {
        self.jobs = jobs;
    }

    pub fn set_download_start(&mut self, start: usize) {
        self.download_start = start;
    }

    pub fn sql() -> String {
        Config::get_instance().lock().unwrap().sql.clone()
    }

    pub fn filter_name() -> String {
        Config::get_instance().lock().unwrap().filter_name.clone()
    }

    pub fn enable_db() -> bool {
        Config::get_instance().lock().unwrap().enable_db
    }

    pub fn enable_filter() -> bool {
        Config::get_instance().lock().unwrap().enable_filter
    }

    pub fn jobs() -> usize {
        Config::get_instance().lock().unwrap().jobs
    }

    pub fn download_start() -> usize {
        Config::get_instance().lock().unwrap().download_start
    }
}

mod tests {}
