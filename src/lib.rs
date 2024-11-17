#![feature(iter_intersperse)]

use std::{collections::BTreeMap, io::Cursor};

use chrono::NaiveDate;
use polars::prelude::*;
use sxd_xpath::nodeset::DocOrder;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn diff(extranet_csv: &[u8], operoo_xls: &[u8]) -> Result<JsValue, String> {
    check_report_types(extranet_csv, operoo_xls)?;

    let mut tables: BTreeMap<String, DataFrame> = BTreeMap::new();

    let extranet = CsvReadOptions::default()
        .with_ignore_errors(true)
        .into_reader_with_file_handle(Cursor::new(extranet_csv))
        .finish()
        .map_err(|e| format!("Error while reading extranet data: {e}"))?
        .lazy();

    let mut operoo =
        read_operoo(operoo_xls).map_err(|e| format!("Error while reading operoo data: {e}"))?;

    operoo = operoo.with_column(col("Profile Id").cast(DataType::Int64));

    let missing_from_operoo = extranet
        .clone()
        .join(
            operoo.clone(),
            [col("RegID")],
            [col("Profile Id")],
            JoinArgs::new(JoinType::Anti),
        )
        .select([col("RegID"), col("FullName").alias("Name")])
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if missing_from_operoo.shape().0 != 0 {
        tables.insert("Missing from Operoo".to_string(), missing_from_operoo);
    }

    let missing_from_extranet = operoo
        .clone()
        .join(
            extranet.clone(),
            [col("Profile Id")],
            [col("RegID")],
            JoinArgs::new(JoinType::Anti),
        )
        .select([
            col("Profile Id").alias("RegID"),
            col("Person Name").alias("Name"),
        ])
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if missing_from_extranet.shape().0 != 0 {
        tables.insert("Missing from Extranet".to_string(), missing_from_extranet);
    }

    let joined = extranet.join(
        operoo,
        [col("RegID")],
        [col("Profile Id")],
        JoinArgs::new(JoinType::Inner),
    );

    let different_names = joined
        .clone()
        .select([
            col("RegID"),
            col("FullName").alias("Extranet"),
            col("Person Name").alias("Operoo"),
        ])
        .filter(
            col("Extranet")
                .map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| {
                                name.map(|x| {
                                    x.to_lowercase()
                                        .split_whitespace()
                                        .intersperse(" ")
                                        .collect::<String>()
                                })
                            })
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                )
                .eq(col("Operoo").map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| {
                                name.map(|x| {
                                    x.to_lowercase()
                                        .split_whitespace()
                                        .intersperse(" ")
                                        .collect::<String>()
                                })
                            })
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                ))
                .not(),
        )
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if different_names.shape().0 != 0 {
        tables.insert("Different Names".to_string(), different_names);
    }

    let different_primary_contact_name = joined
        .clone()
        .filter(col("ClassID").is_in(lit(Series::new(
            "".into(),
            ["JOEY", "CUB", "SCOUT", "VENT"],
        ))))
        .select([
            col("RegID"),
            concat_str(
                [
                    col("Primary Contact Firstname"),
                    col("Primary Contact Surname"),
                ],
                " ",
                false,
            )
            .alias("Extranet"),
            col("Profile Owner's Name ").alias("Operoo"),
        ])
        .filter(
            col("Extranet")
                .map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| {
                                name.map(|x| {
                                    x.to_lowercase()
                                        .split_whitespace()
                                        .intersperse(" ")
                                        .collect::<String>()
                                })
                            })
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                )
                .eq(col("Operoo").map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| {
                                name.map(|x| {
                                    x.to_lowercase()
                                        .split_whitespace()
                                        .intersperse(" ")
                                        .collect::<String>()
                                })
                            })
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                ))
                .not(),
        )
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if different_primary_contact_name.shape().0 != 0 {
        tables.insert(
            "Different Primary Contact Names".to_string(),
            different_primary_contact_name,
        );
    }

    let different_primary_contact_email = joined
        .clone()
        .filter(col("ClassID").is_in(lit(Series::new(
            "".into(),
            ["JOEY", "CUB", "SCOUT", "VENT"],
        ))))
        .select([
            col("RegID"),
            col("Primary Contact Email").alias("Extranet"),
            col("Profile Owner's Email").alias("Operoo"),
        ])
        .filter(
            col("Extranet")
                .map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| name.map(str::to_lowercase))
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                )
                .eq(col("Operoo").map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| name.map(str::to_lowercase))
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                ))
                .not(),
        )
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if different_primary_contact_email.shape().0 != 0 {
        tables.insert(
            "Different Primary Contact Emails".to_string(),
            different_primary_contact_email,
        );
    }

    let different_primary_contact_mobile = joined
        .clone()
        .filter(col("ClassID").is_in(lit(Series::new(
            "".into(),
            ["JOEY", "CUB", "SCOUT", "VENT"],
        ))))
        .select([
            col("RegID"),
            col("Primary Contact Mobile")
                .alias("Extranet")
                .cast(DataType::String),
            col("Profile Owner's Mobile Phone")
                .alias("Operoo")
                .cast(DataType::String),
        ])
        .filter(
            col("Extranet")
                .map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| name.map(|x| &x[(x.len() - 9)..]))
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                )
                .eq(col("Operoo").map(
                    |col| {
                        let out: StringChunked = col
                            .str()?
                            .into_iter()
                            .map(|name| name.map(|x| &x[(x.len() - 9)..]))
                            .collect();
                        Ok(Some(out.into_column()))
                    },
                    GetOutput::from_type(DataType::String),
                ))
                .not(),
        )
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if different_primary_contact_mobile.shape().0 != 0 {
        tables.insert(
            "Different Primary Contact Mobile Numbers".to_string(),
            different_primary_contact_mobile,
        );
    }

    let different_date_of_birth = joined
        .select([
            col("RegID"),
            col("DOB").alias("Extranet").map(
                |col| {
                    let out: StringChunked = col
                        .str()?
                        .into_iter()
                        .map(|name| {
                            name.map(|x| {
                                NaiveDate::parse_from_str(x, "%F")
                                    .map(|date| date.to_string())
                                    .unwrap_or_default()
                            })
                        })
                        .collect();
                    Ok(Some(out.into_column()))
                },
                GetOutput::from_type(DataType::String),
            ),
            col("Person Birth Date").alias("Operoo").map(
                |col| {
                    let out: StringChunked = col
                        .str()?
                        .into_iter()
                        .map(|name| {
                            name.map(|x| {
                                NaiveDate::parse_from_str(x, "%e %B %Y")
                                    .map(|date| date.to_string())
                                    .unwrap_or_default()
                            })
                        })
                        .collect();
                    Ok(Some(out.into_column()))
                },
                GetOutput::from_type(DataType::String),
            ),
        ])
        .filter(col("Extranet").eq(col("Operoo")).not())
        .collect()
        .map_err(|e| format!("Error while processing data: {e}"))?;

    if different_date_of_birth.shape().0 != 0 {
        tables.insert(
            "Different Date of Birth".to_string(),
            different_date_of_birth,
        );
    }

    serde_wasm_bindgen::to_value(&tables).map_err(|e| format!("Error while reporting data: {e}"))
}

fn read_operoo(raw_xls: &[u8]) -> Result<LazyFrame, String> {
    // for some reason the operoo export has two extra spaces at the start
    let data = std::str::from_utf8(&raw_xls[2..]).map_err(|e| e.to_string())?;

    let package = sxd_document::parser::parse(data).map_err(|e| e.to_string())?;
    let document = package.as_document();
    let docorder = DocOrder::new(document);

    let mut context = sxd_xpath::Context::new();
    context.set_namespace("doc", "urn:schemas-microsoft-com:office:spreadsheet");

    let factory = sxd_xpath::Factory::new();

    let header_xpath = factory
        .build("doc:Workbook/doc:Worksheet/doc:Table/doc:Row[1]/doc:Cell/doc:Data/text()")
        .map_err(|e| e.to_string())?;

    let sxd_xpath::Value::Nodeset(headers) = header_xpath
        .evaluate(&context, document.root())
        .map_err(|e| e.to_string())?
    else {
        return Err("Expected row to be nodeset".to_string());
    };

    const NEEDED_COLUMNS: &[&str] = &[
        "Profile Id",
        "Person Name",
        "Profile Owner's Name ",
        "Profile Owner's Email",
        "Profile Owner's Mobile Phone",
        "Person Birth Date",
    ];

    let mut column_length = None;
    let mut columns = Vec::new();
    for (i, column_name) in headers
        .document_order(Some(&docorder))
        .iter()
        .filter_map(|node| node.text().map(|node| node.text()))
        .enumerate()
        .filter(|(_, column)| NEEDED_COLUMNS.contains(column))
    {
        let column_xpath = factory
            .build(&format!(
                "doc:Workbook/doc:Worksheet/doc:Table/doc:Row[position()>1]/doc:Cell[{}]/doc:Data",
                i + 1
            ))
            .map_err(|e| e.to_string())?;

        let sxd_xpath::Value::Nodeset(column_nodes) = column_xpath
            .evaluate(&context, document.root())
            .map_err(|e| e.to_string())?
        else {
            return Err("Expected column to be nodeset".to_string());
        };

        let mut column: Vec<_> = column_nodes
            .document_order(Some(&docorder))
            .iter()
            .map(|node| {
                node.children()
                    .first()
                    .and_then(|node| node.text())
                    .map(|node| node.text())
            })
            .collect();

        match column_length {
            Some(length) => column.resize(length, None),
            None => column_length = Some(column.len()),
        }

        columns.push(Series::new(column_name.into(), column).into_column());
    }

    Ok(DataFrame::new(columns).map_err(|e| e.to_string())?.lazy())
}

fn check_report_types(extranet_csv: &[u8], operoo_xls: &[u8]) -> Result<(), String> {
    if !extranet_csv.starts_with(b"RegionID,") {
        return Err("The provided Extranet report appears to be invalid.".to_string());
    }

    if !operoo_xls.starts_with(b"  <?xml") {
        return Err("The provided Operoo report appears to be invalid.".to_string());
    }

    Ok(())
}
