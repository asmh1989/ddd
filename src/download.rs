use log::info;

use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    cell::RefCell,
    cmp::max,
    fs,
    io::Cursor,
    os::unix::prelude::MetadataExt,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    db::{Db, COLLECTION_CID_NOT_FOUND},
    filter_cid,
    model::PubChemNotFound,
};

/// 一个最大并发量
const IP_THREADS: usize = 4;

static HTTP_PROXYS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| {
    let m = [
        ("127.0.0.1:7890"),
        ("127.0.0.1:7891"),
        ("127.0.0.1:7892"),
        ("127.0.0.1:7893"),
        ("192.168.2.228:7890"),
        // ("106.12.88.204:8888"),
        // ("173.82.20.11:8889"),
        // (""),
    ]
    .iter()
    .cloned()
    .collect();

    Mutex::new(m)
});

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub next: Option<Rc<RefCell<TreeNode>>>,
    pub ip: String,
    pub visits: usize,
}

#[inline]
fn ptr_node(n: &Rc<RefCell<TreeNode>>) -> &mut TreeNode {
    let t = n.as_ref().as_ptr();
    unsafe { &mut *t }
}

impl TreeNode {
    pub fn new(next: Option<Rc<RefCell<TreeNode>>>, ip: String) -> Self {
        TreeNode {
            next,
            visits: 0,
            ip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PubDownload {
    ips: Rc<RefCell<TreeNode>>,
}

impl PubDownload {
    pub fn get_instance() -> Arc<Mutex<PubDownload>> {
        static mut CONFIG: Option<Arc<Mutex<PubDownload>>> = None;

        unsafe {
            // Rust中使用可变静态变量都是unsafe的
            CONFIG
                .get_or_insert_with(|| {
                    // 初始化单例对象的代码
                    Arc::new(Mutex::new(PubDownload::new()))
                })
                .clone()
        }
    }

    fn new() -> Self {
        let ips = Rc::new(RefCell::new(TreeNode::new(None, "".to_string())));
        let ips2 = &mut ips.clone();
        let proxys = HTTP_PROXYS.lock().unwrap().clone();
        proxys.iter().for_each(|&f| set_next_node(ips2, f));
        Self { ips }
    }

    #[inline]
    pub fn get_node() -> Rc<RefCell<TreeNode>> {
        let n = PubDownload::get_instance().lock().unwrap().ips.clone();
        n
    }
}

fn set_next_node(node: &mut Rc<RefCell<TreeNode>>, ip: &str) {
    let next = TreeNode::new(None, ip.to_string());

    // info!("node = {:?}", node);
    loop {
        let n = ptr_node(node);
        if n.ip.is_empty() {
            n.ip = ip.to_string();
            break;
        }
        if n.next.is_none() {
            n.next = Some(Rc::new(RefCell::new(next)));
            break;
        }
        *node = n.next.clone().unwrap();
    }
}

fn get_use_ip() -> Option<String> {
    let node = &mut PubDownload::get_node();
    loop {
        let n = ptr_node(node);
        if n.visits < IP_THREADS {
            n.visits += 1;
            return Some(n.ip.clone());
        }
        if n.next.is_none() {
            break;
        }
        *node = n.next.clone().unwrap();
    }

    None
}

fn release_ip(ip: &str) {
    let node = &mut PubDownload::get_node();
    loop {
        let n = ptr_node(node);

        if n.ip == ip {
            if n.visits > 0 {
                n.visits -= 1;
            }
            info!("release ip = {}", ip);

            break;
        }

        if n.next.is_none() {
            info!("ip = {}, not use", ip);
            break;
        }
        *node = n.next.clone().unwrap();
    }
}

fn fetch_url(f: usize, file_name: String, usb_db: bool, ip: &str) -> Result<(), String> {
    info!(
        "start download id = {}, path = {}, ip = {}",
        f, file_name, ip
    );

    let url = get_url(f);
    let path = std::path::Path::new(&file_name);
    let prefix = path.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();

    let client = if ip.is_empty() {
        reqwest::blocking::Client::new()
    } else {
        reqwest::blocking::Client::builder()
            .proxy(reqwest::Proxy::http(ip).expect("http proxy set error"))
            .build()
            .map_err(|e| e.to_string())?
    };

    // let headers = HEADERS.get().expect("header not init");

    let response = client
        .get(url)
        // .headers(headers.clone())
        .send()
        .map_err(|e| e.to_string())?;
    let code = response.status().as_u16();

    if !response.status().is_success() {
        if code == 404 && usb_db {
            let d = PubChemNotFound::new(&f.to_string());
            let _ = d.save_db();
        }

        return Err(format!("请求失败! code = {}", response.status()));
    }
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    if bytes.len() < 1024 {
        return Err("文件大小不对".to_string());
    }
    let mut content = Cursor::new(bytes);
    let mut file = std::fs::File::create(file_name).map_err(|e| e.to_string())?;
    std::io::copy(&mut content, &mut file).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_path_by_id(id: usize) -> String {
    let million: usize = 1000000;
    let thousand: usize = 1000;

    let first = id / million;

    let second = (id - first * million) / thousand;

    return format!(
        "{}/{}/{}.json",
        (first + 1) * million,
        (second + 1) * thousand,
        id
    );
}

pub fn file_exist(path: &str) -> bool {
    let meta = fs::metadata(path);
    if let Ok(m) = meta {
        if m.is_file() && m.size() > 1024 {
            return true;
        } else {
            if m.is_dir() {
                let _ = fs::remove_dir_all(path);
            }
            return false;
        }
    } else {
        return false;
    }
}

#[inline]
fn get_url(f: usize) -> String {
    format!("https://pubchem.ncbi.nlm.nih.gov/rest/pug_view/data/compound/{}/JSON/?response_type=save&response_basename=compound_CID_{}", f, f)
}

fn get_chem(f: usize, use_db: bool) {
    let path = format!("data/{}", get_path_by_id(f as usize));

    if !file_exist(&path) {
        if !use_db || !Db::contians(COLLECTION_CID_NOT_FOUND, filter_cid!(&f.to_string())) {
            loop {
                let ip = get_use_ip();
                if let Some(i) = ip {
                    let result = fetch_url(f, path.clone(), use_db, &i);
                    if result.is_err() {
                        info!("id = {}, result = {:?}", f, result);
                    }
                    release_ip(&i);
                    break;
                } else {
                    info!("need sleep ...");
                    thread::sleep(Duration::from_millis(2000))
                }
            }
        }
    } else {
        // info!("already download {}", f);
    }
}

pub fn download_chems(start: usize, use_db: bool) {
    let step = 10000000;
    let count = HTTP_PROXYS.lock().unwrap().len();

    rayon::ThreadPoolBuilder::new()
        .num_threads(count * IP_THREADS)
        .build_global()
        .unwrap();

    (max(1, start * step)..(start + 1) * step)
        .into_par_iter()
        .for_each(|f| {
            get_chem(f, use_db);
        });
}

#[cfg(test)]
mod tests {
    use crate::db;

    use super::*;

    fn init() {
        db::init_db("mongodb://192.168.2.25:27017");
        crate::config::init_config();
    }

    #[test]
    fn test_proxy_download() {
        init();
        download_chems(1, true);
    }

    #[test]
    fn test_file_exist() {
        crate::config::init_config();

        let meta = fs::metadata("data/1000000/1000/1.json");

        assert!(meta.is_ok());

        let mm = meta.unwrap();

        assert!(mm.is_file());

        info!("size = {}", mm.size());

        // assert!(file_exist("data/1000000/1000/1.json"));
    }

    #[test]
    fn test_request_proxy() {
        init();

        let s = "".to_string();
        let s2 = "";
        if s == s2 {
            info!("true");
        }

        assert!(s == s2);

        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", get_use_ip());
        info!("d = {:?}", release_ip(""));

        info!("d = {:?}", PubDownload::get_instance().lock().unwrap());

        let cid = 22222222;

        let path = format!("data/{}", get_path_by_id(cid as usize));

        let t = fetch_url(cid, path, true, "192.168.2.228:7890");
        info!("t = {:?}", t);

        // assert!(t.is_ok());
    }

    #[test]
    fn test_download() {
        init();
        download_chems(0, true);
    }

    #[test]
    fn test_download_not_found() {
        init();
        get_chem(25928, true);
    }
}
