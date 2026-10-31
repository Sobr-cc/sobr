//! Characterization test of the datacenter-to-IRIS join (SOB-222).
//!
//! `iris_link.rs:139` links every datacenter to the nearest IRIS, whatever its
//! country and however far it is. The audit counted 24 meaningless links for
//! the datacenters outside France. SOB-21 decides what to do with them.

mod common;

use common::GEOJSON_RTE;
use sobria_geoloc::DatacenterRecord;
use sobria_ingest::iris_link::build_links;

fn datacenter(id: &str, country: &str, lat: f64, lon: f64) -> DatacenterRecord {
    DatacenterRecord {
        id: id.into(),
        name: format!("DC {id}"),
        operator: "test".into(),
        country_iso: country.into(),
        city: "somewhere".into(),
        lat,
        lon,
        pue: 1.5,
        wue_l_per_kwh: None,
        if_electrical_g_per_kwh: 60.0,
        capacity_mw: None,
        sources: vec!["test".into()],
        hourly_profile_24h: vec![],
    }
}

#[test]
fn iris_link_quirk_a_datacenter_outside_france_is_linked_to_the_nearest_french_iris() {
    let datacenters = [
        datacenter("paris", "FR", 48.8566, 2.3522),
        datacenter("frankfurt", "DE", 50.1109, 8.6821),
        datacenter("sydney", "AU", -33.8688, 151.2093),
    ];

    let links = build_links(&datacenters, GEOJSON_RTE).expect("links");

    assert_eq!(links.len(), 3, "one link per datacenter, none refused");
    let link = |id: &str| {
        links
            .iter()
            .find(|l| l.datacenter_id == id)
            .unwrap_or_else(|| panic!("no link for {id}"))
    };
    assert_eq!(link("paris").code_iris, "751010101");
    assert!(
        link("paris").distance_km < 1.0,
        "{}",
        link("paris").distance_km
    );

    // Frankfurt is about 480 km from Paris: still linked to the Paris IRIS.
    let frankfurt = link("frankfurt");
    assert_eq!(frankfurt.code_iris, "751010101");
    assert!(
        (450.0..510.0).contains(&frankfurt.distance_km),
        "{}",
        frankfurt.distance_km
    );

    // Sydney is about 17,000 km away: there is no distance limit either.
    assert!(
        link("sydney").distance_km > 15_000.0,
        "{}",
        link("sydney").distance_km
    );
}
