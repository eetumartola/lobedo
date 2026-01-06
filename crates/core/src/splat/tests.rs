use glam::{Mat4, Quat, Vec3};

use super::SplatGeo;

#[test]
fn transform_updates_positions_and_scales() {
    let mut splats = SplatGeo::with_len(1);
    splats.positions[0] = [1.0, 2.0, 3.0];
    splats.scales[0] = [0.0, 0.0, 0.0];
    splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];

    let matrix = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 3.0, 4.0),
        Quat::IDENTITY,
        Vec3::new(1.0, 0.0, 0.0),
    );
    splats.transform(matrix);

    let pos = splats.positions[0];
    assert!((pos[0] - 3.0).abs() < 1.0e-4);
    assert!((pos[1] - 6.0).abs() < 1.0e-4);
    assert!((pos[2] - 12.0).abs() < 1.0e-4);

    let scale = splats.scales[0];
    assert!((scale[0] - 2.0_f32.ln()).abs() < 1.0e-4);
    assert!((scale[1] - 3.0_f32.ln()).abs() < 1.0e-4);
    assert!((scale[2] - 4.0_f32.ln()).abs() < 1.0e-4);
}

#[test]
fn transform_preserves_log_scale_encoding() {
    let mut splats = SplatGeo::with_len(1);
    let log_half = 0.5f32.ln();
    splats.scales[0] = [log_half, log_half, log_half];
    splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];

    splats.transform(Mat4::from_scale(Vec3::splat(2.0)));

    let scale = splats.scales[0];
    assert!(scale[0].abs() < 1.0e-4);
    assert!(scale[1].abs() < 1.0e-4);
    assert!(scale[2].abs() < 1.0e-4);
}

#[test]
fn transform_rotates_sh_l1() {
    let mut splats = SplatGeo::with_len_and_sh(1, 3);
    splats.sh_rest[0] = [0.0, 0.0, 0.0];
    splats.sh_rest[1] = [0.0, 0.0, 0.0];
    splats.sh_rest[2] = [-1.0, 0.0, 0.0];

    let matrix = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    splats.transform(matrix);

    let coeffs = &splats.sh_rest[0..3];
    assert!((coeffs[0][0] + 1.0).abs() < 1.0e-4);
    assert!(coeffs[1][0].abs() < 1.0e-4);
    assert!(coeffs[2][0].abs() < 1.0e-4);
}

#[test]
fn transform_rotates_sh_l2() {
    let mut splats = SplatGeo::with_len_and_sh(1, 8);
    splats.sh_rest[4] = [1.0, 0.0, 0.0];

    let matrix = Mat4::from_rotation_z(std::f32::consts::PI);
    splats.transform(matrix);

    let coeff = splats.sh_rest[4][0];
    assert!((coeff + 1.0).abs() < 2.0e-3);
}

#[test]
fn transform_rotates_sh_l3() {
    let mut splats = SplatGeo::with_len_and_sh(1, 15);
    splats.sh_rest[13] = [1.0, 0.0, 0.0];

    let matrix = Mat4::from_rotation_z(std::f32::consts::PI);
    splats.transform(matrix);

    let coeff = splats.sh_rest[13][0];
    assert!((coeff - 1.0).abs() < 2.0e-3);
}

#[test]
fn validate_rejects_nan_positions() {
    let mut splats = SplatGeo::with_len(1);
    splats.positions[0][1] = f32::NAN;
    assert!(splats.validate().is_err());
}

#[test]
fn validate_rejects_nan_sh_coeffs() {
    let mut splats = SplatGeo::with_len_and_sh(1, 3);
    splats.sh_rest[1] = [f32::NAN, 0.0, 0.0];
    assert!(splats.validate().is_err());
}
