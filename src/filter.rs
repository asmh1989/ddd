use std::{
    fs::{self, File},
    io::BufWriter,
    os::linux::fs::MetadataExt,
    sync::{Arc, Mutex},
};

use log::info;
use mongodb::{
    bson::{self, doc, Document},
    options::FindOptions,
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    chem::{parse_json, Chem},
    db::{Db, COLLECTION_FILTER_SMILES_SOLUBILITY, COLLECTION_FILTER_WATER_SOLUBILITY},
    filter_cid,
};

const BUFFER_SIZE: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,
    pub cid: i64,
    pub smiles: String,
    pub inchi: String,
    pub molecular_weight: String,
    pub solubility: Vec<String>,
    pub melting_point: Vec<String>,
    pub logp: Vec<String>,
}

impl Filter {
    pub fn new(
        cid: i64,
        smiles: String,
        molecular_weight: String,
        inchi: String,
        solubility: Vec<String>,
        melting_point: Vec<String>,
        logp: Vec<String>,
    ) -> Self {
        Self {
            id: None,
            cid,
            smiles,
            molecular_weight,
            solubility,
            inchi,
            melting_point,
            logp,
        }
    }

    pub fn document(&self) -> Result<Document, String> {
        match bson::to_bson(&self) {
            Ok(d) => return Ok(d.as_document().unwrap().clone()),
            Err(e) => {
                info!("to_bson err {}", e);
                return Err(format!("to_bson error : {}", e));
            }
        };
    }

    pub fn save_db(&self) -> Result<(), String> {
        let doc = match bson::to_bson(&self) {
            Ok(d) => d.as_document().unwrap().clone(),
            Err(e) => {
                info!("to_bson err {}", e);
                return Err(format!("to_bson error : {}", e));
            }
        };

        if let Err(e) = Db::save(
            COLLECTION_FILTER_SMILES_SOLUBILITY,
            filter_cid!(self.cid.clone()),
            doc.clone(),
        ) {
            info!("db save error {} ", e);
            return Err(format!("db save error {} ", e));
        }
        Ok(())
    }
}

pub fn get_json_files(path: &str, vec: &Mutex<Vec<String>>) {
    let paths = fs::read_dir(path).unwrap();
    let mut v = Vec::<String>::with_capacity(1000);

    paths.for_each(|f| {
        if let Ok(d) = f {
            let p = d.path();
            if p.is_dir() {
                // get_json_files(p.to_str().unwrap(), vec);
                v.push(p.to_str().unwrap().to_string());
            } else if let Some(k) = p.extension() {
                if k == "json" {
                    // info!("found json file : {:?}", p);
                    let m = p.metadata().unwrap();
                    let path = p
                        .clone()
                        .into_os_string()
                        .into_string()
                        .unwrap()
                        .to_string();
                    if m.st_size() > 1024 {
                        vec.lock().unwrap().push(path);
                    } else {
                        info!(
                            "remove file = {}, becase size = {} ",
                            path.clone(),
                            m.st_size()
                        );
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    });

    v.into_par_iter().for_each(|f| get_json_files(&f, vec));
}

// #[inline]
fn insert_many(buffer: &Mutex<Vec<Document>>) {
    let d = &mut buffer.lock().unwrap();
    let len = d.len();
    if len > 0 {}
}

fn parse_chem(chem: &Chem, table: &str, buffer: &Arc<Mutex<Vec<Document>>>) {
    let cid = chem.record.record_number;
    let mut vec: Vec<String> = Vec::new();
    let mut melting_v: Vec<String> = Vec::new();
    let mut logp: Vec<String> = Vec::new();
    let mut molecular_weight = "".to_string();
    let mut canonical_smiles = "".to_string();
    let mut isomeric_smiles = "".to_string();
    let mut inchi = "".to_string();
    chem.record
        .section
        .iter()
        .for_each(|s| match &s.tocheading[..] {
            "Names and Identifiers" => {
                s.section.iter().for_each(|s2| match &s2.tocheading[..] {
                    "Computed Descriptors" => {
                        s2.section.iter().for_each(|s3| match &s3.tocheading[..] {
                            "Canonical SMILES" => {
                                if !s3.information.is_empty()
                                    && !s3.information[0].value.string_with_markup.is_empty()
                                {
                                    canonical_smiles = s3.information[0].value.string_with_markup
                                        [0]
                                    .string
                                    .clone();
                                }
                            }
                            "Isomeric SMILES" => {
                                isomeric_smiles =
                                    s3.information[0].value.string_with_markup[0].string.clone();
                            }
                            "InChI" => {
                                inchi =
                                    s3.information[0].value.string_with_markup[0].string.clone();
                            }
                            _ => {}
                        });
                    }
                    _ => {}
                });
            }
            "Chemical and Physical Properties" => {
                s.section.iter().for_each(|s2| match &s2.tocheading[..] {
                    "Experimental Properties" => {
                        s2.section.iter().for_each(|s3| match &s3.tocheading[..] {
                            "Solubility" => {
                                s3.information.iter().for_each(|f| {
                                    if !f.value.string_with_markup.is_empty() {
                                        vec.push(f.value.string_with_markup[0].string.clone());
                                    }
                                });
                            }
                            "Melting Point" => {
                                s3.information.iter().for_each(|f| {
                                    if !f.value.string_with_markup.is_empty() {
                                        melting_v
                                            .push(f.value.string_with_markup[0].string.clone());
                                    }
                                });
                            }
                            "LogP" => {
                                s3.information.iter().for_each(|f| {
                                    if !f.value.string_with_markup.is_empty() {
                                        logp.push(f.value.string_with_markup[0].string.clone());
                                    }
                                });
                            }
                            _ => {}
                        });
                    }
                    "Computed Properties" => {
                        s2.section.iter().for_each(|s3| match &s3.tocheading[..] {
                            "Molecular Weight" => {
                                molecular_weight = format!(
                                    "{} {}",
                                    s3.information[0].value.string_with_markup[0].string.clone(),
                                    s3.information[0].value.unit.clone().unwrap()
                                );
                            }
                            _ => {}
                        });
                    }
                    _ => {}
                });
            }
            _ => {}
        });

    if !vec.is_empty() {
        let f = Filter::new(
            cid,
            canonical_smiles,
            molecular_weight,
            inchi,
            vec,
            melting_v,
            logp,
        );
        // info!("filter = {}", serde_json::to_string_pretty(&f).unwrap())

        let d = &mut buffer.lock().unwrap();

        d.push(f.document().unwrap());
        let len = d.len();
        if len == BUFFER_SIZE {
            let result = Db::insert_many(table, d.to_owned());
            info!("instert {}, is {}", len, result.is_ok());
            d.clear();
        }

        // let _ = f.save_db();
    }
}

pub fn start_parse(dir: &str, table: &str) {
    // info!("remove table : {:?}", Db::delete_table(table));

    let vec2 = Mutex::new(Vec::<String>::with_capacity(512));
    get_json_files(dir, &vec2);

    let vec = vec2.lock().unwrap().to_owned();

    info!("path in dir : {}, found json files : {}", dir, vec.len());

    let data = Arc::new(Mutex::new(Vec::<Document>::with_capacity(BUFFER_SIZE)));

    rayon::scope(|s| {
        let count = Arc::new(Mutex::new(0));
        let c_count = Arc::clone(&count);
        let c_data = Arc::clone(&data);
        let finish = Arc::new(Mutex::new(false));
        let c_finish = Arc::clone(&finish);
        s.spawn(move |_| {
            vec.into_par_iter().for_each(|f| {
                // info!("start parse {}", f);
                *c_count.lock().unwrap() += 1;

                // let name = std::path::PathBuf::from(&f)
                //     .file_stem()
                //     .unwrap()
                //     .to_os_string()
                //     .into_string()
                //     .unwrap();

                // if contains(&name, table) {
                //     return;
                // }
                let result = parse_json(&f);
                if let Ok(chem) = result {
                    parse_chem(&chem, table, &c_data);
                } else {
                    info!("{}, err = {:?}", f, result);
                }
            });

            *c_finish.lock().unwrap() = true;
        });

        let mut times = 0;
        let mut prev = 0;

        s.spawn(move |_| loop {
            if *finish.lock().unwrap() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
            if times == 30 {
                let p = *count.lock().unwrap();
                info!("30 s, finish parse file counter = {}", p - prev);
                prev = p;
                times = 0;
            } else {
                times += 1;
            }
        });
    });

    let d = data.lock().unwrap().to_owned();
    let len = d.len();
    if len > 0 {
        let result = Db::insert_many(table, d);
        info!("instert {}, is {}", len, result.is_ok());
    }
}

pub fn start_filter(name: &str, data: &str) {
    match name {
        _ => start_parse(data, COLLECTION_FILTER_SMILES_SOLUBILITY),
    }
}

fn contains(cid: &str, table: &str) -> bool {
    Db::contians(table, doc! {"cid":cid.parse::<i64>().unwrap()})
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterWater {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,
    pub cid: i64,
    pub smiles: String,
    pub inchi: String,
    pub molecular_weight: String,
    pub solubility: String,
    pub value: String,
}

impl FilterWater {
    pub fn new(
        cid: i64,
        smiles: String,
        molecular_weight: String,
        inchi: String,
        solubility: String,
        value: String,
    ) -> Self {
        Self {
            id: None,
            cid,
            smiles,
            molecular_weight,
            solubility,
            inchi,
            value,
        }
    }

    pub fn save_db(&self) -> Result<(), String> {
        let doc = match bson::to_bson(&self) {
            Ok(d) => d.as_document().unwrap().clone(),
            Err(e) => {
                info!("to_bson err {}", e);
                return Err(format!("to_bson error : {}", e));
            }
        };

        if let Err(e) = Db::save(
            COLLECTION_FILTER_WATER_SOLUBILITY,
            filter_cid!(self.cid.clone()),
            doc.clone(),
        ) {
            info!("db save error {} ", e);
            return Err(format!("db save error {} ", e));
        }
        Ok(())
    }
}

fn find_water_solbility() {
    let find_options = FindOptions::builder()
        .projection(doc! {"cid": 1, "smiles": 1, "molecularWeight": 1, "solubility": 1, "inchi": 1})
        .build();

    let re = Regex::new(r"([0-9][0-9X.+\-,]{0,})").unwrap();
    let re2 = Regex::new(r"([m]?[gG/01 ]{2,}[m]?[l|L])").unwrap();

    let writer = BufWriter::new(File::create("data/output.csv").unwrap());

    let wtr = Mutex::new(csv::Writer::from_writer(writer));

    {
        // We still need to write headers manually.
        wtr.lock()
            .unwrap()
            .write_record(&[
                "cid",
                "smiles",
                "inchi",
                "molecularWeight",
                "solubility",
                "value",
            ])
            .unwrap();
    }

    let _ = Db::find(
        COLLECTION_FILTER_SMILES_SOLUBILITY,
        doc! {"$expr":{"$gte":[{"$size":"$solubility"},1]}},
        find_options,
        &|f: Filter| {
            // info!("find filter = {:?}", f);
            f.solubility.iter().for_each(|s| {
                if s.to_lowercase().contains("water")
                    && s.contains("25")
                    && s.contains("°C")
                    && re2.is_match(&s)
                {
                    let mut v = "".to_string();
                    let mut u = "".to_string();

                    if let Some(t) = re2.captures(&s) {
                        u = t.get(1).unwrap().as_str().to_string();
                    }

                    let s2 = s.replace(&u, "").replace("25 °C", "");

                    if let Some(t) = re.captures(&s2) {
                        v = t.get(1).unwrap().as_str().to_string();
                    }

                    if v.is_empty() || u.is_empty() {
                        v.clear();
                        u.clear();
                    }

                    // let water = FilterWater::new(
                    //     f.cid,
                    //     f.smiles.clone(),
                    //     f.molecular_weight.clone(),
                    //     f.inchi.clone(),
                    //     s.clone(),
                    //     format!("{} {}", v, u),
                    // );
                    // let _ = water.save_db();

                    wtr.lock()
                        .unwrap()
                        .serialize((
                            f.cid,
                            f.smiles.clone(),
                            f.molecular_weight.clone(),
                            f.inchi.clone(),
                            s.clone(),
                            format!("{} {}", v, u),
                        ))
                        .unwrap();
                }
            });
        },
    );

    wtr.lock().unwrap().flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_json_file() {
        crate::config::init_config();
        let vec2 = Mutex::new(Vec::<String>::with_capacity(1024));
        let dir = "data";
        get_json_files(dir, &vec2);

        let vec = vec2.lock().unwrap().to_owned();

        info!("path in dir : {}, found json files : {}", dir, vec.len());
    }

    #[test]
    fn test_filter_solubitily() {
        crate::config::init_config();
        crate::db::init_db("mongodb://192.168.2.25:27017");

        let table = "test_filter_demo";

        rayon::ThreadPoolBuilder::new()
            .num_threads(16)
            .build_global()
            .unwrap();

        start_parse("data/1000000", table);
        // start_parse("data/2000000", table, false);
        // start_parse("data/3000000", table, false);
        // start_parse("data/4000000", table, false);
        // start_parse("data", table);
        // start_parse("data/6000000", table, false);
    }

    #[test]
    fn test_find_water() {
        crate::config::init_config();
        crate::db::init_db("mongodb://192.168.2.25:27017");

        assert_eq!(
            Db::count(
                COLLECTION_FILTER_SMILES_SOLUBILITY,
                doc! {"$expr":{"$gt":[{"$size":"$solubility"},1]}},
            ),
            6328
        );

        find_water_solbility();
    }

    #[test]
    fn test_regex() {
        let re = Regex::new(r"([0-9][0-9X.+\-,]{0,})").unwrap();

        assert_eq!(
            "7.48X10-7",
            re.captures("In water, 7.48X10-7 mg/L at 25 °C (est)")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
        );

        assert_eq!(
            "1",
            re.captures("In water, 1 mg/L at 25 掳C")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
        );
    }
}
