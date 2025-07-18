use glam::{U16Vec3, Vec3Swizzles as _};
use thiserror::Error;

use crate::{
    Aabb3d, AreaType, CompactHeightfield, RegionId,
    contours::{ContourSet, RegionVertexId},
    math::{next, prev},
};

/// Represents a polygon mesh suitable for use in building a navigation mesh.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PolygonMesh {
    /// The mesh vertices.
    vertices: Vec<U16Vec3>,
    /// Polygon and neighbor data. [Length: [`Self::max_polygons`] * 2 * [`Self::vertices_per_polygon`]
    polygons: Vec<u8>,
    /// The region id assigned to each polygon.
    regions: Vec<RegionId>,
    /// The flags assigned to each polygon.
    flags: Vec<u16>,
    /// The area id assigned to each polygon.
    areas: Vec<AreaType>,
    /// The number of allocated polygons
    max_polygons: usize,
    /// The maximum number of vertices per polygon
    vertices_per_polygon: usize,
    /// The bounding box of the mesh in world space.
    aabb: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    cell_height: f32,
    /// The AABB border size used to generate the source data from which the mesh was derived.
    border_size: u16,
    /// The max error of the polygon edges in the mesh.
    max_edge_error: f32,
}

impl ContourSet {
    /// Builds a polygon mesh from the provided contours.
    pub fn into_polygon_mesh(
        self,
        max_vertices_per_polygon: usize,
    ) -> Result<PolygonMesh, PolygonMeshError> {
        let mut mesh = PolygonMesh {
            aabb: self.aabb,
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            border_size: self.border_size,
            max_edge_error: self.max_error,
            vertices_per_polygon: max_vertices_per_polygon,
            ..Default::default()
        };

        let mut max_vertices = 0;
        let mut max_tris = 0;
        let mut max_verts_per_cont = 0;
        for contour in &self.contours {
            /// Skip null contours.
            if contour.vertices.len() < 3 {
                continue;
            }
            max_vertices += contour.vertices.len();
            max_tris += contour.vertices.len() - 2;
            max_verts_per_cont = max_verts_per_cont.max(contour.vertices.len());
        }

        if max_vertices > u16::MAX as usize {
            // Jan: Is this sensible? It's the original, but I suspect u32 is fine
            return Err(PolygonMeshError::TooManyVertices {
                actual: max_vertices,
                max: u16::MAX as usize,
            });
        }

        let mut vflags = vec![false; max_vertices];
        mesh.vertices = vec![U16Vec3::ZERO; max_vertices];
        mesh.polygons = vec![u8::MAX; max_tris * max_vertices_per_polygon * 2];
        mesh.regions = vec![RegionId::default(); max_tris];
        mesh.areas = vec![AreaType::default(); max_tris];

        let mut next_vert = vec![0; max_vertices / 3];
        let mut first_vec: [Option<u16>; 1 << 12] = [None; 1 << 12];
        let mut indices = vec![0; max_verts_per_cont];
        let mut tris = vec![U16Vec3::ZERO; max_verts_per_cont];
        let mut polys = vec![0; (max_verts_per_cont + 1) * max_vertices_per_polygon];
        let mut temp_poly = &mut polys[max_verts_per_cont * max_vertices_per_polygon];

        for cont in &self.contours {
            // Skip null contours.
            if cont.vertices.len() < 3 {
                continue;
            }

            // Triangulate contour
            for j in 0..cont.vertices.len() {
                indices[j] = j;
            }

            // Jan: we treat an invalid triangulation as an error instead of a warning.
            let ntris = triangulate(&cont.vertices, &mut indices, &mut tris)?;

            // Add and merge vertices.
            for j in 0..cont.vertices.len() {
                let (v, region) = &cont.vertices[j];
                indices[j] = todo!();
                if (region & RegionVertexId::BORDER_VERTEX.bits() as usize) != 0 {
                    // This vertex should be removed.
                    vflags[indices[j]] = true;
                }
            }
            todo!();
        }
        todo!();

        Ok(mesh)
    }
}

fn triangulate(
    verts: &[(U16Vec3, usize)],
    indices: &mut [usize],
    tris: &mut [U16Vec3],
) -> Result<usize, PolygonMeshError> {
    let mut n = verts.len();
    let mut ntris = 0;

    // The last bit of the index is used to indicate if the vertex can be removed.
    for i in 0..n {
        let i1 = next(i, n);
        let i2 = next(i1, n);
        if is_diagonal(i, i2, &verts, indices) {
            indices[i1] |= CAN_REMOVE;
        }
    }
    while n > 3 {
        let mut min_len = None;
        let mut mini = None;
        for i in 0..n {
            let i1 = next(i, n);
            if (indices[i1] & CAN_REMOVE) != 0 {
                let p0 = verts[indices[i] & INDEX_MASK].0;
                let p2 = verts[indices[next(i1, n)] & INDEX_MASK].0;

                let d = p2 - p0;
                let len = d.xz().length_squared();
                if min_len.is_none() || !min_len.is_some_and(|min| len >= min) {
                    min_len = Some(len);
                    mini = Some(i);
                }
            }
        }
        if mini.is_none() {
            // We might get here because the contour has overlapping segments, like this:
            //
            //  A o-o=====o---o B
            //   /  |C   D|    \.
            //  o   o     o     o
            //  :   :     :     :
            // We'll try to recover by loosing up the inCone test a bit so that a diagonal
            // like A-B or C-D can be found and we can continue.
            min_len = None;
            for i in 0..n {
                let i1 = next(i, n);
                let i2 = next(i1, n);
                if is_diagonal_loose(i, i2, &verts, &indices) {
                    let p0 = verts[indices[i] & INDEX_MASK].0;
                    let p2 = verts[indices[next(i2, n)] & INDEX_MASK].0;
                    let d = p2 - p0;
                    let len = d.xz().length_squared();
                    if min_len.is_none() || !min_len.is_some_and(|min| len >= min) {
                        min_len = Some(len);
                        mini = Some(i);
                    }
                }
            }
        }

        let Some(mini) = mini else {
            // The contour is messed up. This sometimes happens
            // if the contour simplification is too aggressive.
            return Err(PolygonMeshError::InvalidContour);
        };

        let mut i = mini;
        let mut i1 = next(i, n);
        let i2 = next(i1, n);

        tris[ntris].x = (indices[i] & CAN_REMOVE) as u16;
        tris[ntris].y = (indices[i1] & CAN_REMOVE) as u16;
        tris[ntris].z = (indices[i2] & CAN_REMOVE) as u16;
        ntris += 1;

        // Removes P[i1] by copying P[i+1]...P[n-1] left one index.
        n -= 1;
        for k in i1..n {
            indices[k] = indices[k + 1];
        }

        if i1 >= n {
            i1 = 0;
        }
        i = prev(i1, n);
        // Update diagonal flags.
        if is_diagonal(prev(i, n), i1, verts, indices) {
            indices[i] |= CAN_REMOVE;
        } else {
            indices[i] &= INDEX_MASK;
        }

        if is_diagonal(i, next(i1, n), verts, indices) {
            indices[i1] |= CAN_REMOVE;
        } else {
            indices[i1] &= INDEX_MASK;
        }
    }
    // Append the remaining triangle.
    tris[ntris].x = (indices[0] & INDEX_MASK) as u16;
    tris[ntris].y = (indices[1] & INDEX_MASK) as u16;
    tris[ntris].z = (indices[2] & INDEX_MASK) as u16;
    ntris += 1;

    Ok(ntris)
}

const CAN_REMOVE: usize = 0x80000000;

/// Returns true iff (v_i, v_j) is a proper internal diagonal of P.
fn is_diagonal(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    in_cone(i, j, verts, indices) && is_diagonal_internal_or_external(i, j, verts, indices)
}

/// Returns true iff the diagonal (i,j) is strictly internal to the
/// polygon P in the neighborhood of the i endpoint.
fn in_cone(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    let n = verts.len();
    let pi = verts[indices[i] & INDEX_MASK].0;
    let pj = verts[indices[j] & INDEX_MASK].0;
    let pi1 = verts[indices[next(i, n)] & INDEX_MASK].0;
    let pin1 = verts[indices[prev(i, n)] & INDEX_MASK].0;

    // If P[i] is a convex vertex [ i+1 left or on (i-1,i) ].
    if is_left_on(pin1, pi, pi1) {
        is_left(pi, pj, pin1) && is_left(pj, pi, pi1)
    } else {
        // Assume (i-1,i,i+1) not collinear.
        // else P[i] is reflex.
        !(is_left_on(pi, pj, pi1) && is_left_on(pj, pi, pin1))
    }
}

#[inline]
fn is_left_on(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) <= 0
}

/// Returns true iff c is strictly to the left of the directed line through a to b.
#[inline]
fn is_left(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) < 0
}

#[inline]
fn area2(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> i32 {
    let a = a.as_ivec3();
    let b = b.as_ivec3();
    let c = c.as_ivec3();
    (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z)
}

// Returns T iff (v_i, v_j) is a proper internal *or* external
// diagonal of P, *ignoring edges incident to v_i and v_j*.
fn is_diagonal_internal_or_external(
    i: usize,
    j: usize,
    verts: &[(U16Vec3, usize)],
    indices: &[usize],
) -> bool {
    let n = verts.len();
    let d0 = verts[indices[i] & INDEX_MASK].0;
    let d1 = verts[indices[j] & INDEX_MASK].0;

    // For each edge (k,k+1) of P
    for k in 0..n {
        let k1 = next(k, n);
        // Skip edges incident to i or j
        if !((k == i) || (k1 == i) || (k == j) || (k1 == j)) {
            let p0 = verts[indices[k] & INDEX_MASK].0;
            let p1 = verts[indices[k1] & INDEX_MASK].0;
            if vequal(d0, p0) || vequal(d1, p0) || vequal(d0, p1) || vequal(d1, p1) {
                continue;
            }
            if intersect(d0, d1, p0, p1) {
                return false;
            }
        }
    }
    true
}

const INDEX_MASK: usize = 0x0fffffff;

#[inline]
fn vequal(a: U16Vec3, b: U16Vec3) -> bool {
    a.xz() == b.xz()
}

/// Returns true iff segments ab and cd intersect, properly or improperly.
#[inline]
fn intersect(a: U16Vec3, b: U16Vec3, c: U16Vec3, d: U16Vec3) -> bool {
    if intersect_prop(a, b, c, d) {
        return true;
    }
    between(a, b, c) || between(a, b, d) || between(c, d, a) || between(c, d, b)
}

///	Returns true iff ab properly intersects cd: they share
///	a point interior to both segments.  The properness of the
///	intersection is ensured by using strict leftness.
#[inline]
fn intersect_prop(a: U16Vec3, b: U16Vec3, c: U16Vec3, d: U16Vec3) -> bool {
    // Eliminate improper cases.
    if collinear(a, b, c) || collinear(a, b, d) || collinear(c, d, a) || collinear(c, d, b) {
        return false;
    }
    (left(a, b, c) ^ left(a, b, d)) && (left(c, d, a) ^ left(c, d, b))
}

#[inline]
fn collinear(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) == 0
}

/// Returns true iff c is strictly to the left of the directed
/// line through a to b.
#[inline]
fn left(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) < 0
}

#[inline]
fn left_on(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) <= 0
}

/// Returns T iff (a,b,c) are collinear and point c lies
/// on the closed segment ab.
#[inline]
fn between(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    if !collinear(a, b, c) {
        return false;
    }
    // If ab not vertical, check betweenness on x; else on z.
    if a.x != b.x {
        (a.x <= c.x && c.x <= b.x) || (a.x >= c.x && c.x >= b.x)
    } else {
        (a.z <= c.z && c.z <= b.z) || (a.z >= c.z && c.z >= b.z)
    }
}

fn is_diagonal_loose(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    in_cone_loose(i, j, verts, indices)
        && is_diagonal_internal_or_external_loose(i, j, verts, indices)
}

fn in_cone_loose(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    let n = verts.len();
    let pi = verts[indices[i] & INDEX_MASK].0;
    let pj = verts[indices[j] & INDEX_MASK].0;
    let pi1 = verts[indices[next(i, n)] & INDEX_MASK].0;
    let pin1 = verts[indices[prev(i, n)] & INDEX_MASK].0;

    // If P[i] is a convex vertex [ i+1 left or on (i-1,i) ].
    if left_on(pin1, pi, pi1) {
        left_on(pi, pj, pin1) && left_on(pj, pi, pi1)
    } else {
        !(left_on(pi, pj, pi1) && left_on(pj, pi, pin1))
    }
}

fn is_diagonal_internal_or_external_loose(
    i: usize,
    j: usize,
    verts: &[(U16Vec3, usize)],
    indices: &[usize],
) -> bool {
    let n = verts.len();
    let d0 = verts[indices[i] & INDEX_MASK].0;
    let d1 = verts[indices[j] & INDEX_MASK].0;

    // For each edge (k,k+1) of P
    for k in 0..n {
        let k1 = next(k, n);
        // Skip edges incident to i or j
        if !(k == i || k1 == i || k == j || k1 == j) {
            let p0 = verts[indices[k] & INDEX_MASK].0;
            let p1 = verts[indices[k1] & INDEX_MASK].0;
            if vequal(d0, p0) || vequal(d1, p0) || vequal(d0, p1) || vequal(d1, p1) {
                continue;
            }
            if intersect_prop(d0, d1, p0, p1) {
                return false;
            }
        }
    }
    true
}

#[derive(Error, Debug)]
pub enum PolygonMeshError {
    #[error("Too many vertices: {actual} > {max}")]
    TooManyVertices { actual: usize, max: usize },

    #[error(
        "Invalid contour. This sometimes happens if the contour simplification is too aggressive."
    )]
    InvalidContour,
}
