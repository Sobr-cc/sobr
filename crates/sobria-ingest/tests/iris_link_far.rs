//! Characterization test of the datacenter-to-IRIS join (SOB-222).
//!
//! `build_links` links every datacenter to the nearest IRIS, whatever its
//! country and however far it is. The audit counted 24 meaningless links for
//! the datacenters outside France. SOB-21 decides what to do with them.

use serde_json::json;
use sobria_geoloc::DatacenterRecord;
use sobria_ingest::iris_link::build_links;

/// One small IRIS square in Paris.
const PARIS_IRIS: &str = r#"{"type":"FeatureCollection","features":[
{"type":"Feature","properties":{"code_iris":"751010101"},"geometry":{"type":"Polygon","coordinates":[[[2.3472,48.8516],[2.3572,48.8516],[2.3572,48.8616],[2.3472,48.8616],[2.3472,48.8516]]]}}
]}"#;

/// Only the fields `DatacenterRecord` requires; the others take their default.
fn datacenter(id: &str, country: &str, lat: f64, lon: f64) -> DatacenterRecord {
    serde_json::from_value(json!({
        "id": id, "name": id, "operator": "test", "country_iso": country, "city": "somewhere",
        "lat": lat, "lon": lon, "pue": 1.5, "if_electrical_g_per_kwh": 60.0,
        "sources": ["test"],
    }))
    .expect("a datacenter record")
}

#[test]
fn iris_link_quirk_a_datacenter_outside_france_is_linked_to_the_nearest_french_iris() {
    let datacenters = [
        datacenter("paris", "FR", 48.8566, 2.3522),
        datacenter("frankfurt", "DE", 50.1109, 8.6821),
        datacenter("sydney", "AU", -33.8688, 151.2093),
    ];

    let links = build_links(&datacenters, PARIS_IRIS).expect("links");

    assert_eq!(links.len(), 3, "one link per datacenter, none refused");
    let link = |id: &str| {
        links
            .iter()
            .find(|l| l.datacenter_id == id)
            .unwrap_or_else(|| panic!("no link for {id}"))
    };
    let (paris, frankfurt, sydney) = (link("paris"), link("frankfurt"), link("sydney"));

    assert_eq!(paris.code_iris, "751010101");
    assert!(paris.distance_km < 1.0, "{}", paris.distance_km);

    // Frankfurt is about 480 km from Paris: still linked to the Paris IRIS.
    assert_eq!(frankfurt.code_iris, "751010101");
    assert!(
        (450.0..510.0).contains(&frankfurt.distance_km),
        "{}",
        frankfurt.distance_km
    );

    // Sydney is about 17,000 km away: there is no distance limit either.
    assert_eq!(sydney.code_iris, "751010101");
    assert!(sydney.distance_km > 15_000.0, "{}", sydney.distance_km);
}
