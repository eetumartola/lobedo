use glam::{Mat3, Mat4, Vec3};

pub(super) fn mat3_is_finite(mat: Mat3) -> bool {
    mat.to_cols_array().iter().all(|value| value.is_finite())
}

pub(super) fn rotation_from_matrix(matrix: Mat4) -> Mat3 {
    rotation_from_linear(Mat3::from_mat4(matrix))
}

pub(super) fn rotation_from_linear(linear: Mat3) -> Mat3 {
    if !mat3_is_finite(linear) {
        return Mat3::IDENTITY;
    }
    let mut x = linear.x_axis;
    let mut y = linear.y_axis;

    if x.length_squared() > 0.0 {
        x = x.normalize();
    } else {
        x = Vec3::X;
    }
    y = (y - x * y.dot(x)).normalize_or_zero();
    if y.length_squared() == 0.0 {
        y = Vec3::Y;
    }
    let mut z = x.cross(y);
    if z.length_squared() == 0.0 {
        z = Vec3::Z;
    } else {
        z = z.normalize();
    }

    let mut rot = Mat3::from_cols(x, y, z);
    if rot.determinant() < 0.0 {
        rot = Mat3::from_cols(x, y, -z);
    }
    if !mat3_is_finite(rot) || rot.determinant().abs() < 1.0e-6 {
        return Mat3::IDENTITY;
    }
    rot
}

#[allow(clippy::needless_range_loop)]
pub(super) fn eigen_decomposition_symmetric(mat: Mat3) -> (Vec3, Mat3) {
    let cols = mat.to_cols_array_2d();
    let mut a = [
        [cols[0][0], cols[1][0], cols[2][0]],
        [cols[0][1], cols[1][1], cols[2][1]],
        [cols[0][2], cols[1][2], cols[2][2]],
    ];
    let mut v = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    const MAX_ITERS: usize = 16;
    const EPS: f32 = 1.0e-6;

    for _ in 0..MAX_ITERS {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut max = a[0][1].abs();

        let a02 = a[0][2].abs();
        if a02 > max {
            max = a02;
            p = 0;
            q = 2;
        }
        let a12 = a[1][2].abs();
        if a12 > max {
            max = a12;
            p = 1;
            q = 2;
        }

        if max < EPS {
            break;
        }

        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];

        if apq.abs() < EPS {
            continue;
        }

        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        for i in 0..3 {
            if i == p || i == q {
                continue;
            }
            let aip = a[i][p];
            let aiq = a[i][q];
            a[i][p] = c * aip - s * aiq;
            a[p][i] = a[i][p];
            a[i][q] = s * aip + c * aiq;
            a[q][i] = a[i][q];
        }

        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for i in 0..3 {
            let vip = v[i][p];
            let viq = v[i][q];
            v[i][p] = c * vip - s * viq;
            v[i][q] = s * vip + c * viq;
        }
    }

    let eigenvalues = Vec3::new(a[0][0], a[1][1], a[2][2]);
    let eigenvectors = Mat3::from_cols(
        Vec3::new(v[0][0], v[1][0], v[2][0]),
        Vec3::new(v[0][1], v[1][1], v[2][1]),
        Vec3::new(v[0][2], v[1][2], v[2][2]),
    );
    (eigenvalues, eigenvectors)
}
