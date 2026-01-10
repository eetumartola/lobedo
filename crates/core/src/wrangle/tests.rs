use super::{apply_wrangle, apply_wrangle_splats};
use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::mesh::Mesh;
use crate::splat::SplatGeo;
use crate::volume::{Volume, VolumeKind};

#[test]
fn wrangle_ptnum_sets_point_attribute() {
    let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let mut mesh = Mesh::with_positions_indices(positions, Vec::new());
    apply_wrangle(
        &mut mesh,
        AttributeDomain::Point,
        "@id = @ptnum;",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let Some(AttributeStorage::Float(values)) =
        mesh.attributes.get(AttributeDomain::Point, "id")
    else {
        panic!("Expected float attribute 'id' on points");
    };
    assert_eq!(values, &vec![0.0, 1.0, 2.0]);
}

#[test]
fn wrangle_point_query_secondary_mesh() {
    let mut mesh = Mesh::with_positions_indices(
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        Vec::new(),
    );
    let secondary = Mesh::with_positions_indices(
        vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
        Vec::new(),
    );

    apply_wrangle(
        &mut mesh,
        AttributeDomain::Point,
        "@P = point(1, P, @ptnum);",
        None,
        Some(&secondary),
        None,
        None,
        None,
        None,
    )
    .unwrap();

    assert_eq!(mesh.positions, secondary.positions);
}

#[test]
fn wrangle_point_query_secondary_splats() {
    let mut splats = SplatGeo::with_len(2);
    splats.positions = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let mut secondary = SplatGeo::with_len(2);
    secondary.positions = vec![[3.0, 0.0, 0.0], [6.0, 0.0, 0.0]];

    apply_wrangle_splats(
        &mut splats,
        AttributeDomain::Point,
        "@P = splat(1, P, @ptnum);",
        None,
        Some(&secondary),
        None,
        None,
    )
    .unwrap();

    assert_eq!(splats.positions, secondary.positions);
}

#[test]
fn wrangle_splat_query_secondary_from_mesh() {
    let mut mesh =
        Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], Vec::new());
    let mut secondary = SplatGeo::with_len(2);
    secondary.positions = vec![[5.0, 0.0, 0.0], [7.0, 0.0, 0.0]];

    apply_wrangle(
        &mut mesh,
        AttributeDomain::Point,
        "@P = splat(1, P, @ptnum);",
        None,
        None,
        None,
        Some(&secondary),
        None,
        None,
    )
    .unwrap();

    assert_eq!(mesh.positions, secondary.positions);
}

#[test]
fn wrangle_sample_secondary_volume() {
    let mut mesh = Mesh::with_positions_indices(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        Vec::new(),
    );
    let volume = Volume::new(VolumeKind::Density, [0.0, 0.0, 0.0], [2, 1, 1], 1.0, vec![0.1, 0.9]);

    apply_wrangle(
        &mut mesh,
        AttributeDomain::Point,
        "@val = sample(1, @P);",
        None,
        None,
        None,
        None,
        None,
        Some(&volume),
    )
    .unwrap();

    let Some(AttributeStorage::Float(values)) =
        mesh.attributes.get(AttributeDomain::Point, "val")
    else {
        panic!("Expected float attribute 'val' on points");
    };
    assert_eq!(values.len(), 2);
    assert!((values[0] - 0.1).abs() < 1.0e-4);
    assert!((values[1] - 0.9).abs() < 1.0e-4);
}
