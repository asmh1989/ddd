#![allow(dead_code)]

use log::info;
use structopt::StructOpt;

use crate::{args::Opt, filter::start_filter};

mod args;
mod chem;
mod config;
mod db;
mod download;
mod filter;
mod model;

fn main() {
    // println!("Hello, world!");

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    let mut opt: Opt = Opt::from_args();

    // 打印版本
    if opt.version {
        println!("{}", VERSION);
        return;
    }

    config::Config::get_instance();

    if !opt.enable_filter {
        if opt.jobs > 12 {
            opt.jobs = 12
        }
    } else {
        opt.enable_db = true;
    }

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_enable_db(opt.enable_db);

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_enable_filter(opt.enable_filter);

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_sql(&opt.sql);

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_filter_name(&opt.filter_name);

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_jobs(opt.jobs);

    config::Config::get_instance()
        .lock()
        .unwrap()
        .set_download_start(opt.download_start);

    info!("{:#?}", opt);

    if opt.enable_db {
        db::init_db(&format!("mongodb://{}", opt.sql));
    }

    if opt.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(opt.jobs)
            .build_global()
            .unwrap();
    }

    let start = chrono::Utc::now();
    if opt.enable_filter {
        info!(
            "start filter data , path = {}, threads = {}",
            opt.data_path, opt.jobs
        );

        start_filter(&opt.filter_name, &opt.data_path);
    } else {
        info!(
            "start download = {}, threads = {}",
            opt.download_start, opt.jobs
        );

        download::download_chems(opt.download_start, opt.enable_db);
    }

    let time = chrono::Utc::now() - start;

    info!("finish, time: {} ", time);
}
