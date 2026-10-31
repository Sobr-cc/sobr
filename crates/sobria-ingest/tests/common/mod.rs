//! Helpers shared by the characterization tests of `sobria-ingest` (SOB-222):
//! a local stand-in for the two upstream sources, and readers that turn the
//! files the pipeline writes (Silver Parquet, Gold SQLite) into JSON values.
//!
//! Each test file uses a different part of this module.
#![allow(dead_code)]

#[path = "../../../../test-support/golden.rs"]
pub mod golden;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use polars::prelude::*;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Map, Value};
use sobria_ingest::{
    sources::{ComparIASource, RteIrisSource},
    Context, Downloader, LayerRegistry,
};
use wiremock::{
    matchers::{method, path as wm_path},
    Mock, MockServer, ResponseTemplate,
};

/// Routes served by [`Upstream`] and the Copper file name of each.
pub const COMPARIA_ROUTES: [(&str, &str, &str); 3] = [
    (
        "/comparia/conversations",
        "conversations.parquet",
        "comparia_conversations",
    ),
    ("/comparia/votes", "votes.parquet", "comparia_votes"),
    (
        "/comparia/reactions",
        "reactions.parquet",
        "comparia_reactions",
    ),
];

pub const CSV_RTE: &str = "code_iris,conso_elec_mwh,conso_gaz_mwh,nb_sites,annee
751010101,12500.5,4200.0,3,2023
920400404,33000.0,0.0,5,2023
";

/// Two small IRIS squares, one in Paris and one in Marseille.
pub const GEOJSON_RTE: &str = r#"{"type":"FeatureCollection","features":[
{"type":"Feature","properties":{"code_iris":"751010101"},"geometry":{"type":"Polygon","coordinates":[[[2.3472,48.8516],[2.3572,48.8516],[2.3572,48.8616],[2.3472,48.8616],[2.3472,48.8516]]]}},
{"type":"Feature","properties":{"code_iris":"132010101"},"geometry":{"type":"Polygon","coordinates":[[[5.3648,43.2915],[5.3748,43.2915],[5.3748,43.3015],[5.3648,43.3015],[5.3648,43.2915]]]}}
]}"#;

/// An IRIS file with no feature: the Gold link table stays empty, so a golden
/// file does not depend on the datacenter list of `sobria-geoloc`.
pub const GEOJSON_EMPTY: &str = r#"{"type":"FeatureCollection","features":[]}"#;

fn parquet(mut df: DataFrame) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    ParquetWriter::new(std::io::Cursor::new(&mut buf))
        .finish(&mut df)
        .expect("write a synthetic parquet");
    buf
}

pub fn comparia_conversations() -> Vec<u8> {
    parquet(
        df![
            "id" => [1u32, 2, 3],
            "model" => ["gpt-4o-mini", "claude-3-5", "mistral-large"],
        ]
        .expect("conversations frame"),
    )
}

pub fn comparia_votes() -> Vec<u8> {
    parquet(df!["id" => [1u32, 2], "choice" => ["a", "b"]].expect("votes frame"))
}

pub fn comparia_reactions() -> Vec<u8> {
    parquet(df!["id" => [1u32], "reaction" => ["like"]].expect("reactions frame"))
}

/// A local server that plays data.gouv.fr: it serves the five upstream files
/// and counts the requests it receives.
pub struct Upstream {
    pub server: MockServer,
}

impl Upstream {
    pub async fn start() -> Self {
        let upstream = Self {
            server: MockServer::start().await,
        };
        upstream.serve_defaults().await;
        upstream
    }

    /// Like [`Upstream::start`], with [`GEOJSON_EMPTY`] as the IRIS file.
    pub async fn start_without_geometries() -> Self {
        let upstream = Self {
            server: MockServer::start().await,
        };
        upstream.serve_all(GEOJSON_EMPTY).await;
        upstream
    }

    async fn serve_all(&self, geojson: &str) {
        self.serve("/comparia/conversations", comparia_conversations())
            .await;
        self.serve("/comparia/votes", comparia_votes()).await;
        self.serve("/comparia/reactions", comparia_reactions())
            .await;
        self.serve("/rte/csv", CSV_RTE.as_bytes().to_vec()).await;
        self.serve("/rte/geojson", geojson.as_bytes().to_vec())
            .await;
    }

    pub async fn serve(&self, route: &str, body: Vec<u8>) {
        Mock::given(method("GET"))
            .and(wm_path(route))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&self.server)
            .await;
    }

    pub async fn serve_defaults(&self) {
        self.serve_all(GEOJSON_RTE).await;
    }

    /// Replaces every route by a `404` (not retried by the downloader).
    pub async fn refuse_everything(&self) {
        self.server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }

    /// Number of requests received since the server started or was reset.
    pub async fn request_count(&self) -> usize {
        self.server
            .received_requests()
            .await
            .map_or(0, |requests| requests.len())
    }

    fn downloader() -> Downloader {
        // A proxy variable of the machine must not take the requests away.
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client without proxy");
        Downloader::with_client(client)
    }

    pub fn comparia(&self) -> ComparIASource {
        let urls = COMPARIA_ROUTES
            .iter()
            .map(|(route, file, silver)| {
                (
                    (*file).to_string(),
                    format!("{}{route}", self.server.uri()),
                    (*silver).to_string(),
                )
            })
            .collect();
        ComparIASource::for_test(Self::downloader(), urls)
    }

    pub fn rte_iris(&self) -> RteIrisSource {
        RteIrisSource::for_test(
            Self::downloader(),
            format!("{}/rte/csv", self.server.uri()),
            format!("{}/rte/geojson", self.server.uri()),
        )
    }

    /// Both sources, like `LayerRegistry::standard()` but on the local server.
    pub fn registry(&self) -> LayerRegistry {
        let mut registry = LayerRegistry::new();
        registry.register(Arc::new(self.comparia()));
        registry.register(Arc::new(self.rte_iris()));
        registry
    }

    pub fn comparia_registry(&self) -> LayerRegistry {
        let mut registry = LayerRegistry::new();
        registry.register(Arc::new(self.comparia()));
        registry
    }
}

pub fn context(data_root: &Path) -> Context {
    Context {
        data_root: data_root.to_path_buf(),
        incremental: false,
        seed: 42,
    }
}

/// Runs the full pipeline of both sources into `data_root`, then stops the
/// local server: what is left on disk is a normal Copper, Silver and Gold.
pub async fn populate(data_root: &Path) {
    let upstream = Upstream::start().await;
    let report = upstream
        .registry()
        .run_full_pipeline(&context(data_root))
        .await
        .expect("pipeline");
    assert!(report.failed_sources().is_empty(), "{report:?}");
}

/// The only snapshot folder of a source under `data/copper/<source>/`.
pub fn only_snapshot(data_root: &Path, source: &str) -> PathBuf {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(data_root.join("copper").join(source))
        .expect("copper folder")
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(dirs.len(), 1, "one snapshot folder expected: {dirs:?}");
    dirs.pop().expect("one folder")
}

pub fn gold_sqlite(data_root: &Path) -> Connection {
    Connection::open(data_root.join("gold").join("referentiel.sqlite")).expect("open the Gold")
}

pub fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count")
}

/// A Silver Parquet as `{columns: [{name, dtype}], rows: [{column: text}]}`.
/// Every value is cast to text so the golden file does not depend on how a
/// float or a timestamp prints.
pub fn parquet_dump(path: &Path) -> Value {
    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .expect("scan")
        .collect()
        .expect("collect");
    let columns: Vec<Value> = df
        .get_columns()
        .iter()
        .map(|c| json!({"name": c.name().as_str(), "dtype": c.dtype().to_string()}))
        .collect();
    let as_text: Vec<(String, Column)> = df
        .get_columns()
        .iter()
        .map(|c| {
            (
                c.name().to_string(),
                c.cast(&DataType::String).expect("cast to text"),
            )
        })
        .collect();
    let rows: Vec<Value> = (0..df.height())
        .map(|i| {
            let mut row = Map::new();
            for (name, column) in &as_text {
                let cell = column.str().expect("text column").get(i);
                row.insert(name.clone(), cell.map_or(Value::Null, |s| json!(s)));
            }
            Value::Object(row)
        })
        .collect();
    json!({"columns": columns, "rows": rows})
}

fn cell(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(x) => json!(x),
        ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
        ValueRef::Blob(_) => json!("<blob>"),
    }
}

/// The Gold SQLite as `{tables: {name: {sql, row_count, rows}}, indexes}`.
/// The shadow tables of the full-text index are left out.
pub fn sqlite_dump(conn: &Connection) -> Value {
    let mut statement = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'model_overview_fts_%' ORDER BY name",
        )
        .expect("list tables");
    let tables: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("tables")
        .map(|r| r.expect("table row"))
        .collect();

    let mut out = Map::new();
    for (name, sql) in tables {
        let width = conn
            .prepare(&format!("SELECT * FROM {name}"))
            .expect("select all")
            .column_count();
        let order: Vec<String> = (1..=width).map(|i| i.to_string()).collect();
        let mut select = conn
            .prepare(&format!(
                "SELECT * FROM {name} ORDER BY {}",
                order.join(", ")
            ))
            .expect("select ordered");
        let rows: Vec<Value> = select
            .query_map([], |row| {
                Ok(Value::Array(
                    (0..width)
                        .map(|i| cell(row.get_ref(i).expect("cell")))
                        .collect(),
                ))
            })
            .expect("rows")
            .map(|r| r.expect("row"))
            .collect();
        out.insert(
            name,
            json!({"sql": sql, "row_count": rows.len(), "rows": rows}),
        );
    }

    let mut statement = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master \
             WHERE type = 'index' AND sql IS NOT NULL ORDER BY name",
        )
        .expect("list indexes");
    let indexes: Vec<Value> = statement
        .query_map([], |row| {
            Ok(json!({"name": row.get::<_, String>(0)?, "sql": row.get::<_, String>(1)?}))
        })
        .expect("indexes")
        .map(|r| r.expect("index row"))
        .collect();

    json!({"tables": out, "indexes": indexes})
}

/// Masks what changes from run to run in a value read back from the pipeline:
/// timestamps (alone or inside a sentence), the SHA-256 and the size of
/// generated files, the temp data root and the crate version. Returns the
/// number of masked values, so a test can check that the masking happened.
pub fn mask_volatile(value: &mut Value, data_root: &Path) -> usize {
    let root = data_root.to_string_lossy().replace('\\', "/");
    mask(value, &root)
}

/// [`sqlite_dump`] of the Gold, masked. The rows were read ordered by their
/// raw values, hashes included, so they are ordered again by their masked
/// text: the order must not depend on the value of a hash.
pub fn sqlite_dump_masked(conn: &Connection, data_root: &Path) -> Value {
    let mut dump = sqlite_dump(conn);
    mask_volatile(&mut dump, data_root);
    if let Some(tables) = dump["tables"].as_object_mut() {
        for table in tables.values_mut() {
            if let Some(rows) = table["rows"].as_array_mut() {
                rows.sort_by_key(ToString::to_string);
            }
        }
    }
    dump
}

/// Replaces the first RFC 3339 timestamp found inside `text`.
fn mask_embedded_time(text: &str) -> Option<String> {
    for start in 0..text.len().saturating_sub(18) {
        if !text.is_char_boundary(start) {
            continue;
        }
        let rest = &text[start..];
        let head = rest.as_bytes();
        let looks_like_a_date = head.len() >= 19
            && head[..4].iter().all(u8::is_ascii_digit)
            && head[4] == b'-'
            && head[7] == b'-'
            && head[10] == b'T';
        if !looks_like_a_date {
            continue;
        }
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || "T:.+-Z".contains(c)))
            .unwrap_or(rest.len());
        if chrono::DateTime::parse_from_rfc3339(&rest[..end]).is_ok() {
            return Some(format!("{}<time>{}", &text[..start], &rest[end..]));
        }
    }
    None
}

fn mask(value: &mut Value, root: &str) -> usize {
    match value {
        Value::String(text) => {
            let normalized = text.replace('\\', "/");
            let masked = if chrono::DateTime::parse_from_rfc3339(text).is_ok() {
                Some("<time>".to_string())
            } else if text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
                Some("<sha256>".to_string())
            } else if text == env!("CARGO_PKG_VERSION") {
                Some("<crate-version>".to_string())
            } else if normalized.contains(root) {
                Some(normalized.replace(root, "<data_root>"))
            } else {
                mask_embedded_time(text)
            };
            match masked {
                Some(new) => {
                    *text = new;
                    1
                },
                None => 0,
            }
        },
        Value::Array(items) => items.iter_mut().map(|v| mask(v, root)).sum(),
        Value::Object(map) => map
            .iter_mut()
            .map(|(key, v)| {
                if key == "schema:contentSize" {
                    *v = json!("<size>");
                    1
                } else {
                    mask(v, root)
                }
            })
            .sum(),
        _ => 0,
    }
}
