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

/// `(route, Copper file, Silver entity)` of the three ComparIA files.
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

/// `(source, Silver entity, Copper file it comes from)`: every Silver file of
/// the two sources.
pub const SILVER: [(&str, &str, &str); 4] = [
    (
        "comparia",
        "comparia_conversations",
        "conversations.parquet",
    ),
    ("comparia", "comparia_votes", "votes.parquet"),
    ("comparia", "comparia_reactions", "reactions.parquet"),
    ("rte-iris", "rte_iris_consommation", "consommation_iris.csv"),
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

fn comparia_bodies() -> [Vec<u8>; 3] {
    [
        parquet(
            df![
                "id" => [1u32, 2, 3],
                "model" => ["gpt-4o-mini", "claude-3-5", "mistral-large"],
            ]
            .expect("conversations frame"),
        ),
        parquet(df!["id" => [1u32, 2], "choice" => ["a", "b"]].expect("votes frame")),
        parquet(df!["id" => [1u32], "reaction" => ["like"]].expect("reactions frame")),
    ]
}

/// A local server that plays data.gouv.fr: it serves the five upstream files
/// and counts the requests it receives.
pub struct Upstream {
    server: MockServer,
}

impl Upstream {
    pub async fn start() -> Self {
        Self::start_with(GEOJSON_RTE).await
    }

    /// Like [`Upstream::start`], with [`GEOJSON_EMPTY`] as the IRIS file.
    pub async fn start_without_geometries() -> Self {
        Self::start_with(GEOJSON_EMPTY).await
    }

    async fn start_with(geojson: &str) -> Self {
        let upstream = Self {
            server: MockServer::start().await,
        };
        for ((route, _, _), body) in COMPARIA_ROUTES.iter().zip(comparia_bodies()) {
            upstream.serve(route, body).await;
        }
        upstream.serve_rte(CSV_RTE, geojson).await;
        upstream
    }

    async fn serve(&self, route: &str, body: impl Into<Vec<u8>>) {
        Mock::given(method("GET"))
            .and(wm_path(route))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&self.server)
            .await;
    }

    async fn serve_rte(&self, csv: impl Into<Vec<u8>>, geojson: &str) {
        self.serve("/rte/csv", csv).await;
        self.serve("/rte/geojson", geojson).await;
    }

    /// Serves these two RTE files and nothing else.
    pub async fn replace_rte(&self, csv: impl Into<Vec<u8>>, geojson: &str) {
        self.server.reset().await;
        self.serve_rte(csv, geojson).await;
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

    pub fn comparia_registry(&self) -> LayerRegistry {
        let mut registry = LayerRegistry::new();
        registry.register(Arc::new(self.comparia()));
        registry
    }

    /// Both sources, like `LayerRegistry::standard()` but on the local server.
    pub fn registry(&self) -> LayerRegistry {
        let mut registry = self.comparia_registry();
        registry.register(Arc::new(self.rte_iris()));
        registry
    }
}

pub fn context(data_root: &Path) -> Context {
    Context {
        data_root: data_root.to_path_buf(),
        ..Context::default()
    }
}

/// Runs the full pipeline of both sources against `upstream` into `data_root`.
pub async fn populate_from(upstream: &Upstream, data_root: &Path) {
    let report = upstream
        .registry()
        .run_full_pipeline(&context(data_root))
        .await
        .expect("pipeline");
    assert!(report.failed_sources().is_empty(), "{report:?}");
}

/// Copper, Silver and Gold of both sources, then the local server stops: what
/// is left on disk is a normal data folder.
pub async fn populate(data_root: &Path) {
    populate_from(&Upstream::start().await, data_root).await;
}

/// Only the Copper of both sources.
pub async fn populate_copper(data_root: &Path) {
    let copper = Upstream::start()
        .await
        .registry()
        .run_copper(&context(data_root))
        .await;
    assert!(copper.iter().all(|step| step.is_ok()), "Copper failed");
}

/// The only snapshot folder of a source under `data/copper/<source>/`.
pub fn only_snapshot(data_root: &Path, source: &str) -> PathBuf {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(context(data_root).copper_root(source))
        .expect("copper folder")
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(dirs.len(), 1, "one snapshot folder expected: {dirs:?}");
    dirs.pop().expect("one folder")
}

pub fn silver_path(data_root: &Path, source: &str, entity: &str) -> PathBuf {
    context(data_root)
        .silver_root(source)
        .join(format!("{entity}-v1.parquet"))
}

pub fn gold_sqlite(data_root: &Path) -> Connection {
    Connection::open(context(data_root).gold_root().join("referentiel.sqlite"))
        .expect("open the Gold")
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
    let as_text: Vec<Column> = df
        .get_columns()
        .iter()
        .map(|c| c.cast(&DataType::String).expect("cast to text"))
        .collect();
    let rows: Vec<Value> = (0..df.height())
        .map(|i| {
            let mut row = Map::new();
            for column in &as_text {
                let cell = column.str().expect("text column").get(i);
                row.insert(
                    column.name().to_string(),
                    cell.map_or(Value::Null, |s| json!(s)),
                );
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

/// The Gold SQLite as `{tables: {name: {sql, rows}}, indexes}`, masked.
/// The shadow tables of the full-text index are left out, the SQL text is
/// read without its indentation, and the rows are ordered by their masked
/// text: the order must not depend on the value of a hash.
pub fn sqlite_dump(conn: &Connection, data_root: &Path) -> Value {
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
        let mut select = conn
            .prepare(&format!("SELECT * FROM {name}"))
            .expect("select all");
        let width = select.column_count();
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
        out.insert(name, json!({"sql": one_line(&sql), "rows": rows}));
    }

    let mut statement = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master \
             WHERE type = 'index' AND sql IS NOT NULL ORDER BY name",
        )
        .expect("list indexes");
    let indexes: Vec<Value> = statement
        .query_map([], |row| {
            Ok(json!({"name": row.get::<_, String>(0)?, "sql": one_line(&row.get::<_, String>(1)?)}))
        })
        .expect("indexes")
        .map(|r| r.expect("index row"))
        .collect();

    let mut dump = json!({"tables": out, "indexes": indexes});
    mask_volatile(&mut dump, data_root);
    for table in dump["tables"].as_object_mut().expect("tables").values_mut() {
        table["rows"]
            .as_array_mut()
            .expect("rows")
            .sort_by_key(ToString::to_string);
    }
    dump
}

fn one_line(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Masks what changes from run to run in a value read back from the pipeline:
/// timestamps and SHA-256 hashes (alone or inside a longer string), the size
/// of generated files, the temp data root and the crate version.
pub fn mask_volatile(value: &mut Value, data_root: &Path) {
    let root = data_root.to_string_lossy().replace('\\', "/");
    mask(value, &root);
}

fn mask(value: &mut Value, root: &str) {
    match value {
        Value::String(text) => *text = mask_text(text, root),
        Value::Array(items) => items.iter_mut().for_each(|v| mask(v, root)),
        Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                if key == "schema:contentSize" {
                    *v = json!("<size>");
                } else {
                    mask(v, root);
                }
            }
        },
        _ => {},
    }
}

fn mask_text(text: &str, root: &str) -> String {
    if text == env!("CARGO_PKG_VERSION") {
        return "<crate-version>".to_string();
    }
    let normalized = text.replace('\\', "/");
    let text = if normalized.contains(root) {
        normalized.replace(root, "<data_root>")
    } else {
        text.to_string()
    };
    mask_time(&mask_hash(&text))
}

/// Replaces each run of exactly 64 hexadecimal digits.
fn mask_hash(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    // A final non-hex sentinel flushes the last run.
    for c in text.chars().chain(std::iter::once('\0')) {
        if c.is_ascii_hexdigit() {
            run.push(c);
            continue;
        }
        if run.len() == 64 {
            out.push_str("<sha256>");
        } else {
            out.push_str(&run);
        }
        run.clear();
        if c != '\0' {
            out.push(c);
        }
    }
    out
}

/// Replaces each RFC 3339 timestamp.
fn mask_time(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((start, end)) = find_time(rest) {
        out.push_str(&rest[..start]);
        out.push_str("<time>");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Byte range of the first RFC 3339 timestamp in `text`.
fn find_time(text: &str) -> Option<(usize, usize)> {
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
        let len = rest
            .find(|c: char| !(c.is_ascii_digit() || "T:.+-Z".contains(c)))
            .unwrap_or(rest.len());
        if chrono::DateTime::parse_from_rfc3339(&rest[..len]).is_ok() {
            return Some((start, start + len));
        }
    }
    None
}
