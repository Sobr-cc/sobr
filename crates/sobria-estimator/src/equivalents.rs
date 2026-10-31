//! Converts results into relatable equivalents for the UI.
//!
//! Every conversion is sourced (comment with a reference).
//! See CDC §4.2 (relatable equivalents).

use sobria_core::Equivalent;

/// Average emission factor of a European combustion-engine car, in gCO₂eq/km.
/// Source: ADEME Base Empreinte 2024, average petrol passenger car.
const CAR_G_CO2EQ_PER_KM: f64 = 192.0;

/// Typical energy of a 5 min hot shower (8 L/min, ΔT=30°C, ~electric water heater).
/// Source: ADEME — "guide eau chaude sanitaire" 2023.
const SHOWER_WH: f64 = 1_750.0;

/// Typical energy of a 24" screen over one hour (~25 W on average).
/// Source: ADEME — "guide écrans" 2023.
const SCREEN_HOUR_WH: f64 = 25.0;

/// The equivalents shown with every result: car km, shower, screen hours.
pub(crate) fn standard(g_co2eq: f64, wh: f64) -> Vec<Equivalent> {
    vec![
        co2eq_to_car_km(g_co2eq),
        energy_wh_to_shower_seconds(wh),
        energy_wh_to_screen_hours(wh),
    ]
}

/// Converts a CO₂eq result (in grams) into equivalent car kilometres.
#[must_use]
pub fn co2eq_to_car_km(g_co2eq: f64) -> Equivalent {
    Equivalent {
        label: "km en voiture thermique".into(),
        value: g_co2eq / CAR_G_CO2EQ_PER_KM,
        source: "ADEME Base Empreinte 2024 — voiture essence moyenne (192 gCO₂eq/km)".into(),
    }
}

/// Converts an energy result (in Wh) into equivalent seconds of hot shower.
#[must_use]
pub fn energy_wh_to_shower_seconds(wh: f64) -> Equivalent {
    let ratio = wh / SHOWER_WH;
    Equivalent {
        label: "secondes de douche chaude (5 min ≈ 1 750 Wh)".into(),
        value: ratio * 300.0, // 5 min = 300 s
        source: "ADEME — guide ECS 2023, douche 5 min standard".into(),
    }
}

/// Converts an energy result (in Wh) into equivalent hours of a 24" screen.
#[must_use]
pub fn energy_wh_to_screen_hours(wh: f64) -> Equivalent {
    Equivalent {
        label: "heures d'écran 24\" (~25 W)".into(),
        value: wh / SCREEN_HOUR_WH,
        source: "ADEME — guide écrans 2023, 25 W moyens".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_km_for_known_value() {
        // 192 gCO₂eq = 1 km
        let e = co2eq_to_car_km(192.0);
        assert!((e.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn shower_seconds_for_known_value() {
        // 1 750 Wh = 300 s
        let e = energy_wh_to_shower_seconds(1_750.0);
        assert!((e.value - 300.0).abs() < 1e-9);
    }

    #[test]
    fn screen_hours_for_known_value() {
        // 25 Wh = 1 h
        let e = energy_wh_to_screen_hours(25.0);
        assert!((e.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn equivalents_have_non_empty_source() {
        let e = co2eq_to_car_km(100.0);
        assert_ne!(e.source, "");
        assert_ne!(e.label, "");
    }
}
