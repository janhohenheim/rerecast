use super::*;
use crate::{CompactCell, CompactSpan, PolygonNavmesh};
use glam::Vec3;

type Rect = [u16; 4];

fn contour(points: &[(u16, u16)]) -> Contour {
    let vertices: Vec<_> = points
        .iter()
        .map(|&(x, z)| (U16Vec3::new(x, 2, z), 0))
        .collect();
    Contour {
        raw_vertices: vertices.iter().map(|&(p, r)| (p, r.into())).collect(),
        vertices,
        region: RegionId::from(1),
        area: AreaType(7),
    }
}

fn rectangle([x0, z0, x1, z1]: Rect, hole: bool) -> Contour {
    let mut c = contour(&[(x0, z0), (x0, z1), (x1, z1), (x1, z0)]);
    if hole {
        c.vertices.reverse();
        c.raw_vertices.reverse();
    }
    c
}

fn set(contours: Vec<Contour>) -> ContourSet {
    ContourSet {
        contours,
        cell_size: 1.0,
        cell_height: 1.0,
        ..Default::default()
    }
}

fn merged(contours: Vec<Contour>) -> ContourSet {
    let mut cset = set(contours);
    holes::merge_holes(&mut cset.contours);
    cset
}

// Independent floating-point clipping oracle: a triangle may neither cover
// nor cut across any part of an obstacle, even if its centroid is outside it.
fn clipped_area(triangle: &[U16Vec3; 3], clip: &[(u16, u16)]) -> f64 {
    let mut polygon: Vec<_> = triangle
        .iter()
        .map(|p| [f64::from(p.x), f64::from(p.z)])
        .collect();
    // The convex clipping polygon is positive-wound on the XZ plane.
    for (&(x0, z0), &(x1, z1)) in clip.iter().zip(clip.iter().cycle().skip(1)) {
        let side = |p: [f64; 2]| {
            (f64::from(x1) - f64::from(x0)) * (p[1] - f64::from(z0))
                - (f64::from(z1) - f64::from(z0)) * (p[0] - f64::from(x0))
        };
        let mut clipped = Vec::new();
        for (&a, &b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
            let sa = side(a);
            let sb = side(b);
            if sa >= 0.0 {
                clipped.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                let t = sa / (sa - sb);
                clipped.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
        polygon = clipped;
    }
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        .abs()
        / 2.0
}

fn assert_mesh(
    cset: ContourSet,
    nvp: u16,
    expected_area: i64,
    excluded: &[Rect],
) -> PolygonNavmesh {
    let mesh = cset
        .into_polygon_mesh(nvp)
        .expect("stitched contours must polygonize");
    let mut twice_area = 0;
    for polygon in mesh.polygons() {
        let vertices: Vec<_> = polygon.map(|i| mesh.vertices[usize::from(i)]).collect();
        for i in 1..vertices.len() - 1 {
            let triangle = [vertices[0], vertices[i], vertices[i + 1]];
            let [a, b, c] = triangle;
            let signed = (i64::from(b.x) - i64::from(a.x)) * (i64::from(c.z) - i64::from(a.z))
                - (i64::from(c.x) - i64::from(a.x)) * (i64::from(b.z) - i64::from(a.z));
            assert!(signed < 0, "degenerate or reversed triangle: {triangle:?}");
            twice_area -= signed;
            for &rect in excluded {
                let [x0, z0, x1, z1] = rect;
                assert!(
                    clipped_area(&triangle, &[(x0, z0), (x1, z0), (x1, z1), (x0, z1)]) < 1e-8,
                    "triangle {triangle:?} fills obstacle {rect:?}"
                );
            }
        }
    }
    assert_eq!(
        twice_area,
        expected_area * 2,
        "walkable area must exclude every hole"
    );
    mesh
}

#[test]
fn outlines_without_holes_are_unchanged() {
    let original = vec![
        rectangle([0, 0, 12, 12], false),
        rectangle([20, 0, 30, 10], false),
    ];
    assert_eq!(merged(original.clone()).contours, original);
}

#[test]
fn square_hole_preserves_obstacle_area_in_triangles_and_polygons() {
    let obstacle = [4, 4, 8, 8];
    let original = vec![rectangle([0, 0, 12, 12], false), rectangle(obstacle, true)];
    // Unmerged hole is skipped
    assert_mesh(set(original.clone()), 6, 144, &[]);
    let cset = merged(original.clone());
    assert_eq!(cset.contours[0].vertices.len(), 10);
    assert!(cset.contours[1].vertices.is_empty());
    for (c, raw) in cset.contours.iter().zip(original) {
        assert_eq!(c.raw_vertices, raw.raw_vertices);
        assert_eq!(c.region, raw.region);
        assert_eq!(c.area, raw.area);
    }
    for nvp in [3, 6] {
        let mesh = assert_mesh(cset.clone(), nvp, 128, &[obstacle]);
        assert!(
            mesh.regions[..mesh.polygon_count()]
                .iter()
                .all(|r| *r == RegionId::from(1))
        );
        assert!(mesh.areas.iter().all(|a| *a == AreaType(7)));
    }
}

#[test]
fn multiple_occluding_holes_are_ordered_independently_of_input_start_vertices() {
    let obstacles = [[3, 3, 7, 7], [9, 3, 13, 7], [9, 10, 13, 14], [16, 3, 20, 7]];
    for rotation in 0..4 {
        for reverse in [false, true] {
            let mut contours = vec![rectangle([0, 0, 24, 20], false)];
            contours.extend(obstacles.map(|r| rectangle(r, true)));
            for (i, c) in contours.iter_mut().enumerate() {
                c.vertices.rotate_left((rotation + i) % 4);
            }
            if reverse {
                contours.reverse();
            }
            let cset = merged(contours);
            assert_eq!(
                cset.contours
                    .iter()
                    .filter(|c| !c.vertices.is_empty())
                    .count(),
                1
            );
            for nvp in [3, 6] {
                assert_mesh(cset.clone(), nvp, 416, &obstacles);
            }
        }
    }
}

#[test]
fn concave_outline_bridges_stay_inside_walkable_space() {
    let outline = contour(&[
        (0, 0),
        (0, 20),
        (20, 20),
        (20, 14),
        (6, 14),
        (6, 6),
        (20, 6),
        (20, 0),
    ]);
    let obstacles = [[2, 8, 4, 12], [12, 2, 16, 4], [12, 16, 16, 18]];
    let mut contours = vec![outline];
    contours.extend(obstacles.map(|r| rectangle(r, true)));
    let cset = merged(contours);
    let mut excluded = obstacles.to_vec();
    excluded.push([6, 6, 20, 14]); // The exterior notch is also forbidden.
    for nvp in [3, 6] {
        assert_mesh(cset.clone(), nvp, 264, &excluded);
    }
}

#[test]
fn triangular_and_diagonal_holes_are_stitched_without_filling_them() {
    let triangle = [(4, 4), (8, 4), (6, 8)];
    let diamond = [(12, 4), (16, 8), (12, 12), (8, 8)];
    let cset = merged(vec![
        rectangle([0, 0, 20, 20], false),
        contour(&triangle),
        contour(&diamond),
    ]);
    for nvp in [3, 6] {
        let mesh = assert_mesh(cset.clone(), nvp, 360, &[]);
        for polygon in mesh.polygons() {
            let vertices: Vec<_> = polygon.map(|i| mesh.vertices[usize::from(i)]).collect();
            for i in 1..vertices.len() - 1 {
                let face = [vertices[0], vertices[i], vertices[i + 1]];
                assert!(clipped_area(&face, &triangle) < 1e-8);
                assert!(clipped_area(&face, &diamond) < 1e-8);
            }
        }
    }
}

#[test]
fn concave_hole_can_surround_another_hole_without_containing_it() {
    let concave = contour(&[
        (4, 4),
        (16, 4),
        (16, 16),
        (12, 16),
        (12, 8),
        (8, 8),
        (8, 16),
        (4, 16),
    ]);
    let cset = merged(vec![
        rectangle([0, 0, 20, 20], false),
        rectangle([9, 10, 11, 12], true),
        concave,
    ]);
    // A disjoint rectangular decomposition of the U-shaped obstacle, plus
    // the small obstacle in its open notch, gives an exact coverage oracle.
    let excluded = [
        [4, 4, 16, 8],
        [4, 8, 8, 16],
        [12, 8, 16, 16],
        [9, 10, 11, 12],
    ];
    for nvp in [3, 6] {
        assert_mesh(cset.clone(), nvp, 284, &excluded);
    }
}

#[test]
fn holes_belong_to_their_exact_region_and_area() {
    for different_area in [false, true] {
        let mut first = rectangle([0, 0, 12, 12], false);
        let mut first_hole = rectangle([4, 4, 8, 8], true);
        let mut second = rectangle([20, 0, 32, 12], false);
        let mut second_hole = rectangle([24, 4, 28, 8], true);
        first.region = RegionId::from(4);
        first_hole.region = first.region;
        second.region = if different_area {
            first.region
        } else {
            RegionId::from(5)
        };
        second_hole.region = second.region;
        second.area = AreaType(9);
        second_hole.area = second.area;
        let cset = merged(vec![second_hole, first, first_hole, second]);
        assert!(cset.contours[0].vertices.is_empty());
        assert!(cset.contours[2].vertices.is_empty());
        let mesh = assert_mesh(cset, 6, 256, &[[4, 4, 8, 8], [24, 4, 28, 8]]);
        for (i, polygon) in mesh.polygons().enumerate() {
            let on_left = polygon
                .map(|v| mesh.vertices[usize::from(v)].x)
                .all(|x| x <= 12);
            assert_eq!(mesh.areas[i], AreaType(if on_left { 7 } else { 9 }));
            assert_eq!(
                mesh.regions[i],
                RegionId::from(if on_left || different_area { 4 } else { 5 })
            );
        }
    }
}

#[test]
fn stitching_preserves_heights_raw_loops_and_boundary_flags() {
    let mut original = vec![
        rectangle([0, 0, 12, 12], false),
        rectangle([4, 4, 8, 8], true),
    ];
    for (i, c) in original.iter_mut().enumerate() {
        for (j, (p, flags)) in c.vertices.iter_mut().enumerate() {
            p.y = (30 + i * 4 + j) as u16;
            *flags = (11 + i * 4 + j) as u32 | RegionVertexId::AREA_BORDER.bits();
            if j % 2 == 0 {
                *flags |= RegionVertexId::BORDER_VERTEX.bits();
            }
        }
    }
    let cset = merged(original.clone());
    let vertices = &cset.contours[0].vertices;
    let mut bridges = Vec::new();
    for (&(a, flags), &(b, _)) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
        let boundary = original.iter().any(|c| {
            c.vertices
                .iter()
                .zip(c.vertices.iter().cycle().skip(1))
                .any(|(&(p, r), &(q, _))| a == p && b == q && flags == r)
        });
        if !boundary {
            assert_eq!(
                flags & (RegionVertexId::REGION_MASK | RegionVertexId::AREA_BORDER).bits(),
                0
            );
            let source = original
                .iter()
                .flat_map(|c| &c.vertices)
                .find(|(p, _)| *p == a)
                .unwrap();
            assert_eq!(flags, source.1 & RegionVertexId::BORDER_VERTEX.bits());
            bridges.push((a, b));
        }
    }
    assert_eq!(bridges.len(), 2);
    assert_eq!(bridges[0], (bridges[1].1, bridges[1].0));
    for c in &original {
        for edge in c.vertices.iter().zip(c.vertices.iter().cycle().skip(1)) {
            assert!(
                vertices
                    .iter()
                    .zip(vertices.iter().cycle().skip(1))
                    .any(|(a, b)| { a == edge.0 && b.0 == edge.1.0 })
            );
        }
    }
    for (c, source) in cset.contours.iter().zip(original) {
        assert_eq!(c.raw_vertices, source.raw_vertices);
    }
}

#[test]
fn high_coordinates_and_full_u16_range_use_wide_bridge_math() {
    let shifted = merged(vec![
        rectangle([65520, 65520, 65535, 65535], false),
        rectangle([65524, 65524, 65528, 65528], true),
    ]);
    assert_mesh(shifted, 6, 209, &[[65524, 65524, 65528, 65528]]);

    // The existing polygonizer has separate i32 limits; this checks the new
    // contour predicates and candidate distances across the entire u16 range.
    let full = merged(vec![
        rectangle([0, 0, u16::MAX, u16::MAX], false),
        rectangle([2, 2, 4, 4], true),
    ]);
    assert!(full.contours[1].vertices.is_empty());
    assert_eq!(
        signed_area_xz(&full.contours[0].vertices),
        -2 * (65535_i64 * 65535 - 4)
    );
}

#[test]
fn invalid_hole_groups_remain_intact_and_are_skipped() {
    let outer = rectangle([0, 0, 20, 20], false);
    let hole = rectangle([4, 4, 8, 8], true);
    let triangle = contour(&[(4, 4), (8, 4), (6, 8)]);
    let mut wrong_region = triangle.clone();
    wrong_region.region = RegionId::from(2);
    let mut wrong_area = triangle.clone();
    wrong_area.area = AreaType(8);
    let cases = [
        vec![triangle.clone()],
        vec![outer.clone(), wrong_region],
        vec![outer.clone(), wrong_area],
        vec![outer.clone(), rectangle([24, 4, 28, 8], true)],
        vec![outer.clone(), rectangle([0, 4, 4, 8], true)],
        vec![outer.clone(), rectangle([18, 4, 22, 8], true)],
        vec![outer.clone(), hole.clone(), rectangle([6, 6, 10, 10], true)],
        vec![outer.clone(), hole.clone(), rectangle([5, 5, 7, 7], true)],
        vec![
            outer.clone(),
            hole.clone(),
            rectangle([24, 0, 40, 20], false),
        ],
        vec![outer.clone(), contour(&[(4, 4), (10, 10), (4, 10), (8, 4)])],
        vec![outer, hole, triangle],
    ];
    for original in cases {
        let cset = merged(original.clone());
        assert_eq!(
            cset.contours, original,
            "failed merges must be transactional"
        );

        // Unmerged holes are skipped, leaving only the outlines behind
        let outlines = original
            .into_iter()
            .filter(|c| signed_area_xz(&c.vertices) < 0)
            .collect();

        assert_eq!(
            cset.into_polygon_mesh(6),
            set(outlines).into_polygon_mesh(6)
        );
    }
}

#[test]
fn polygonizer_skips_unmerged_triangular_holes_and_zero_area() {
    for c in [
        contour(&[(2, 2), (8, 2), (4, 8)]),
        contour(&[(2, 2), (4, 4), (8, 8)]),
    ] {
        assert_mesh(set(vec![rectangle([0, 0, 12, 12], false), c]), 3, 144, &[]);
    }
}

fn flat_heightfield(
    width: u16,
    height: u16,
    border: u16,
    obstacles: &[Rect],
) -> CompactHeightfield {
    let mut chf = CompactHeightfield {
        width,
        height,
        border_size: border,
        cell_size: 1.0,
        cell_height: 0.5,
        aabb: Aabb3d {
            min: Vec3::ZERO,
            max: Vec3::new(f32::from(width), 10.0, f32::from(height)),
        },
        max_region: RegionId::from(1),
        cells: vec![CompactCell::default(); usize::from(width) * usize::from(height)],
        ..Default::default()
    };
    for z in border..height - border {
        for x in border..width - border {
            if obstacles.iter().any(|&[x0, z0, x1, z1]| {
                x - border >= x0 && x - border < x1 && z - border >= z0 && z - border < z1
            }) {
                continue;
            }
            let cell = &mut chf.cells[usize::from(x) + usize::from(z) * usize::from(width)];
            cell.set_index(chf.spans.len() as u32);
            cell.set_count(1);
            chf.spans.push(CompactSpan {
                y: 2,
                region: chf.max_region,
                data: 0,
            });
            chf.areas.push(AreaType(7));
        }
    }
    for z in 0..height {
        for x in 0..width {
            let cell = &chf.cells[usize::from(x) + usize::from(z) * usize::from(width)];
            if cell.count() == 0 {
                continue;
            }
            for dir in 0..4 {
                let nx = i32::from(x) + i32::from(dir_offset_x(dir));
                let nz = i32::from(z) + i32::from(dir_offset_z(dir));
                let connected = nx >= 0
                    && nx < i32::from(width)
                    && nz >= 0
                    && nz < i32::from(height)
                    && chf.cells[nx as usize + nz as usize * usize::from(width)].count() > 0;
                chf.spans[cell.index() as usize].set_con(dir, connected.then_some(0));
            }
        }
    }
    chf
}

#[test]
fn build_contours_stitches_four_holes_after_removing_tile_border_offset() {
    let obstacles = [
        [4, 22, 8, 26],
        [40, 22, 44, 26],
        [22, 4, 26, 8],
        [22, 40, 26, 44],
    ];
    for border in [0, 3] {
        let chf = flat_heightfield(48 + border * 2, 48 + border * 2, border, &obstacles);
        let cset = chf.build_contours(0.0, 0, BuildContoursFlags::empty());
        assert_eq!(cset.width, 48);
        assert_eq!(cset.height, 48);
        assert_eq!(cset.aabb.min.x, f32::from(border));
        assert_eq!(cset.aabb.max.z, f32::from(48 + border));
        assert_eq!(cset.contours.len(), 5);
        assert_eq!(
            cset.contours
                .iter()
                .filter(|c| !c.vertices.is_empty())
                .count(),
            1
        );
        assert!(
            cset.contours
                .iter()
                .flat_map(|c| &c.vertices)
                .all(|(p, _)| p.x <= 48 && p.z <= 48)
        );
        assert_mesh(cset, 6, 48 * 48 - 4 * 16, &obstacles);
    }
}
