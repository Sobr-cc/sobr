//! The exact module list of each persona bundle and the canonical orders.

use sobria_core::{
    ModuleId::{self, *},
    Persona,
};

#[test]
fn the_personas_come_in_the_canonical_order() {
    assert_eq!(
        Persona::all(),
        [
            Persona::Student,
            Persona::ProTech,
            Persona::Enterprise,
            Persona::PublicSector,
            Persona::Researcher
        ]
    );
}

#[test]
fn the_modules_come_in_the_canonical_order_without_m4() {
    assert_eq!(
        ModuleId::all(),
        [
            M1, M2, M3, M5, M6, M7, M8, M9, M10, M11, M12, M13, M14, M15, M16, M17, M18, M19, M20,
            M21, M22, M23, M24, M25
        ]
    );
}

#[test]
fn each_persona_bundle_is_pinned_in_order() {
    let bundles: [(Persona, Vec<ModuleId>); 5] = [
        (Persona::Student, vec![M1, M8, M13, M15, M25]),
        (Persona::ProTech, vec![M1, M3, M7, M8, M9, M13]),
        (
            Persona::Enterprise,
            vec![M1, M7, M12, M15, M17, M20, M22, M25],
        ),
        (Persona::PublicSector, vec![M1, M8, M12, M17, M20, M22]),
        (Persona::Researcher, vec![M1, M3, M7, M8, M9, M17]),
    ];

    for (persona, bundle) in bundles {
        assert_eq!(persona.default_modules(), bundle, "{persona:?}");
    }
}
