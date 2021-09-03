use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Chem {
    #[serde(rename = "Record")]
    pub record: Record,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Record {
    #[serde(rename = "RecordType")]
    pub record_type: String,
    #[serde(rename = "RecordNumber")]
    pub record_number: i64,
    #[serde(rename = "RecordTitle")]
    pub record_title: String,
    #[serde(rename = "Section")]
    pub section: Vec<Section>,
    // #[serde(rename = "Reference")]
    // pub reference: Vec<Reference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Section {
    #[serde(rename = "TOCHeading")]
    pub tocheading: String,
    // #[serde(rename = "Description")]
    // pub description: Option<String>,
    #[serde(rename = "Section")]
    #[serde(default)]
    pub section: Vec<Section>,
    #[serde(rename = "Information")]
    #[serde(default)]
    pub information: Vec<Information>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    // #[serde(rename = "DisplayControls")]
    // pub display_controls: Option<DisplayControls>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Information {
    #[serde(rename = "ReferenceNumber")]
    pub reference_number: i64,
    // #[serde(rename = "Description")]
    // pub description: Option<String>,
    #[serde(rename = "Value")]
    pub value: Value,
    // #[serde(rename = "Reference")]
    // #[serde(default)]
    // pub reference: Vec<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Value {
    #[serde(rename = "StringWithMarkup")]
    #[serde(default)]
    pub string_with_markup: Vec<StringWithMarkup>,
    #[serde(rename = "Unit")]
    pub unit: Option<String>,
    #[serde(rename = "Number")]
    #[serde(default)]
    pub number: Vec<f64>,
    // #[serde(rename = "ExternalDataURL")]
    // #[serde(default)]
    // pub external_data_url: Vec<String>,
    #[serde(rename = "MimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "ExternalTableName")]
    pub external_table_name: Option<String>,
    #[serde(rename = "ExternalTableNumRows")]
    pub external_table_num_rows: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct StringWithMarkup {
    #[serde(rename = "String")]
    pub string: String,
    #[serde(rename = "Markup")]
    #[serde(default)]
    pub markup: Vec<Markup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Markup {
    #[serde(rename = "Start")]
    pub start: i64,
    #[serde(rename = "Length")]
    pub length: i64,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    #[serde(rename = "Type")]
    pub type_field: Option<String>,
    #[serde(rename = "Extra")]
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct DisplayControls {
    #[serde(rename = "CreateTable")]
    pub create_table: Option<CreateTable>,
    #[serde(rename = "ShowAtMost")]
    pub show_at_most: Option<i64>,
    #[serde(rename = "ListType")]
    pub list_type: Option<String>,
    #[serde(rename = "MoveToTop")]
    pub move_to_top: Option<bool>,
    #[serde(rename = "HideThisSection")]
    pub hide_this_section: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct CreateTable {
    #[serde(rename = "FromInformationIn")]
    pub from_information_in: String,
    #[serde(rename = "NumberOfColumns")]
    pub number_of_columns: i64,
    #[serde(rename = "ColumnContents")]
    #[serde(default)]
    pub column_contents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Reference {
    #[serde(rename = "ReferenceNumber")]
    pub reference_number: i64,
    #[serde(rename = "SourceName")]
    pub source_name: String,
    #[serde(rename = "SourceID")]
    pub source_id: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    #[serde(rename = "LicenseNote")]
    pub license_note: Option<String>,
    #[serde(rename = "LicenseURL")]
    pub license_url: Option<String>,
    #[serde(rename = "ANID")]
    pub anid: Option<i64>,
    #[serde(rename = "IsToxnet")]
    pub is_toxnet: Option<bool>,
}

pub fn parse_json(file: &str) -> Result<Chem, String> {
    let file = std::fs::read(file).map_err(|f| f.to_string())?;
    let str = unsafe { String::from_utf8_unchecked(file) };
    let json: Chem = serde_json::from_str(&str).map_err(|f| f.to_string())?;

    Ok(json)
}

pub fn parse_json2(file: &str) -> Result<Chem, String> {
    let r = std::fs::File::open(file).map_err(|f| f.to_string())?;
    let result = serde_json::from_reader(&r);
    if result.is_ok() {
        Ok(result.unwrap())
    } else {
        log::info!("json reader error {}", file);
        parse_json2(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    #[test]
    fn test_name() {
        crate::config::init_config();

        let file = "data/1000000/1000/549.json";

        let json = parse_json(file);

        info!("json = {:?}", json);
    }

    #[test]
    fn test_unicode() {
        crate::config::init_config();

        let file = "data/1000000/16000/15938.json";

        let json = parse_json(file);

        info!(
            "json = {}",
            serde_json::to_string_pretty(&json.unwrap()).unwrap()
        );
    }

    #[test]
    fn test_json_faster() {
        crate::config::init_config();
        let file = "data/1000000/1000/1.json";
        info!("start...");
        let _json = parse_json(file);
        info!("end ...");
    }

    #[test]
    fn test_json() {
        crate::config::init_config();

        let j = r#"{
            "TOCHeading": "Status",
            "Description": "Current PubChem record status. \"Non-live\" means this compound is not currently linked to any (live) substance. This could be because of changes in deposited structure of a substance, a substance being revoked, or changes in PubChem's chemical structure processing.",
            "DisplayControls": {
              "HideThisSection": true,
              "MoveToTop": true
            },
            "Information": [
              {
                "ReferenceNumber": 1,
                "Name": "Status",
                "Value": {
                  "StringWithMarkup": [
                    {
                      "String": "Non-live",
                      "Markup": [
                        {
                          "Start": 0,
                          "Length": 8,
                          "Type": "Color",
                          "Extra": "Red"
                        }
                      ]
                    }
                  ]
                }
              }
            ]
          }"#;

        let v: Section = serde_json::from_str(j).unwrap();

        info!("json = {}", serde_json::to_string_pretty(&v).unwrap());
    }
}
