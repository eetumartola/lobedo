use std::collections::HashMap;

use crate::attributes::AttributeDomain;
use crate::mesh::Mesh;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandMode {
    Expand,
    Contract,
}

pub fn mesh_adjacency(mesh: &Mesh, domain: AttributeDomain) -> Vec<Vec<usize>> {
    match domain {
        AttributeDomain::Point => point_neighbors(mesh),
        AttributeDomain::Vertex => vertex_neighbors(mesh),
        AttributeDomain::Primitive => primitive_neighbors(mesh),
        AttributeDomain::Detail => Vec::new(),
    }
}

pub fn expand_mask(
    mask: &[bool],
    neighbors: &[Vec<usize>],
    iterations: usize,
    mode: ExpandMode,
) -> Vec<bool> {
    if iterations == 0 || mask.is_empty() {
        return mask.to_vec();
    }
    let len = mask.len().min(neighbors.len());
    if len == 0 {
        return mask.to_vec();
    }
    let mut current = mask.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            match mode {
                ExpandMode::Expand => {
                    if current[i] {
                        continue;
                    }
                    if list.iter().any(|&n| current.get(n).copied().unwrap_or(false)) {
                        next[i] = true;
                    }
                }
                ExpandMode::Contract => {
                    if !current[i] {
                        continue;
                    }
                    if list.is_empty() {
                        next[i] = false;
                        continue;
                    }
                    if list.iter().any(|&n| !current.get(n).copied().unwrap_or(false)) {
                        next[i] = false;
                    }
                }
            }
        }
        current = next;
    }
    current
}

fn face_counts(mesh: &Mesh) -> Vec<u32> {
    if !mesh.face_counts.is_empty() {
        mesh.face_counts.clone()
    } else if mesh.indices.len().is_multiple_of(3) {
        vec![3; mesh.indices.len() / 3]
    } else if mesh.indices.is_empty() {
        Vec::new()
    } else {
        vec![mesh.indices.len() as u32]
    }
}

fn point_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); mesh.positions.len()];
    let counts = face_counts(mesh);
    let mut cursor = 0usize;
    for count in counts {
        let count = count as usize;
        if count < 2 || cursor + count > mesh.indices.len() {
            cursor = cursor.saturating_add(count);
            continue;
        }
        let face = &mesh.indices[cursor..cursor + count];
        for i in 0..count {
            let a = face[i] as usize;
            let b = face[(i + 1) % count] as usize;
            if a < neighbors.len() && b < neighbors.len() {
                neighbors[a].push(b);
                neighbors[b].push(a);
            }
        }
        cursor += count;
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }
    neighbors
}

fn vertex_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); mesh.indices.len()];
    let counts = face_counts(mesh);
    let mut cursor = 0usize;
    for count in counts {
        let count = count as usize;
        if count < 2 || cursor + count > mesh.indices.len() {
            cursor = cursor.saturating_add(count);
            continue;
        }
        for i in 0..count {
            let a = cursor + i;
            let b = cursor + (i + 1) % count;
            if a < neighbors.len() && b < neighbors.len() {
                neighbors[a].push(b);
                neighbors[b].push(a);
            }
        }
        cursor += count;
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }
    neighbors
}

fn primitive_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let counts = face_counts(mesh);
    let face_count = counts.len();
    let mut neighbors = vec![Vec::new(); face_count];
    let mut edge_map: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    let mut cursor = 0usize;
    for (face_idx, count) in counts.iter().enumerate() {
        let count = *count as usize;
        if count < 2 || cursor + count > mesh.indices.len() {
            cursor = cursor.saturating_add(count);
            continue;
        }
        let face = &mesh.indices[cursor..cursor + count];
        for i in 0..count {
            let a = face[i];
            let b = face[(i + 1) % count];
            if a == b {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            edge_map.entry(key).or_default().push(face_idx);
        }
        cursor += count;
    }
    for faces in edge_map.values() {
        if faces.len() < 2 {
            continue;
        }
        for &a in faces {
            for &b in faces {
                if a != b {
                    neighbors[a].push(b);
                }
            }
        }
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }
    neighbors
}
