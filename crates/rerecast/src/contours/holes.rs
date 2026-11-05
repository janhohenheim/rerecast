// Copyright (c) 2009-2010 Mikko Mononen memon@inside.org
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.
//

// Rust adaptation of Recast/Source/RecastContour.cpp's rcBuildContours,
// mergeRegionHoles, mergeContours, and visibility predicates from
// https://github.com/recastnavigation/recastnavigation/commit/
// bd98d84c274ee06842bf51a4088ca82ac71f8c2d (Zlib license).
// Alterations: i64 geometry, transactional region merges, input validation,
// cones at both endpoints, strict overlap rejection, and bridge edge flags;
// the repair keeps physical obstacles out of walkable geometry.

use super::{Contour, RegionVertexId, signed_area_xz};
use alloc::vec::Vec;
use glam::U16Vec3;

type Vertex = (U16Vec3, u32);

pub(super) fn merge_holes(contours: &mut [Contour]) {
    let areas: Vec<_> = contours
        .iter()
        .map(|c| signed_area_xz(&c.vertices))
        .collect();
    if !areas.iter().any(|&area| area > 0) {
        return;
    }

    let mut order: Vec<_> = (0..contours.len()).collect();
    order.sort_unstable_by_key(|&i| (contours[i].region.bits(), contours[i].area.0, i));
    let mut start = 0;
    while start < order.len() {
        let first = &contours[order[start]];
        let end = start
            + order[start..]
                .iter()
                .take_while(|&&i| {
                    contours[i].region == first.region && contours[i].area == first.area
                })
                .count();
        let group = &order[start..end];
        if group.iter().any(|&i| areas[i] > 0) && !merge_region_holes(contours, group, &areas) {
            // Keep every original loop on failure. In particular, a positive
            // hole must reach the polygonizer's InvalidContour winding guard.
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Unable to merge contour holes for region {:?}, area {:?}",
                contours[group[0]].region,
                contours[group[0]].area
            );
        }
        start = end;
    }
}

fn merge_region_holes(contours: &mut [Contour], group: &[usize], areas: &[i64]) -> bool {
    // As in Recast, a region has one outline and zero or more holes. Do not
    // guess an owner for ambiguous, degenerate or self-overlapping outlines.
    let mut outlines = group.iter().copied().filter(|&i| areas[i] < 0);
    let Some(outline) = outlines.next() else {
        return false;
    };
    if outlines.next().is_some()
        || group
            .iter()
            .any(|&i| areas[i] == 0 || !is_simple(&contours[i].vertices))
    {
        return false;
    }

    let mut holes: Vec<_> = group
        .iter()
        .copied()
        .filter(|&i| areas[i] > 0)
        .map(|i| (i, leftmost(&contours[i].vertices)))
        .collect();
    holes.sort_unstable_by_key(|&(i, v)| {
        let p = contours[i].vertices[v].0;
        (p.x, p.z, i)
    });

    let outer = &contours[outline].vertices;
    for (h, &(i, _)) in holes.iter().enumerate() {
        let hole = &contours[i].vertices;
        if rings_intersect(outer, hole) || !contains(outer, hole[0].0) {
            return false;
        }
        for &(j, _) in &holes[..h] {
            let other = &contours[j].vertices;
            if rings_intersect(hole, other)
                || contains(hole, other[0].0)
                || contains(other, hole[0].0)
            {
                return false;
            }
        }
    }

    let mut merged = outer.clone();
    let mut candidates = Vec::new();
    for (h, &(i, leftmost)) in holes.iter().enumerate() {
        let hole = &contours[i].vertices;
        let mut bridge = None;
        for offset in 0..hole.len() {
            let corner_index = (leftmost + offset) % hole.len();
            let corner = hole[corner_index].0;
            candidates.clear();
            for (j, &(point, _)) in merged.iter().enumerate() {
                if in_cone(&merged, j, corner) && in_cone(hole, corner_index, point) {
                    let dx = i64::from(point.x) - i64::from(corner.x);
                    let dz = i64::from(point.z) - i64::from(corner.z);
                    candidates.push((dx * dx + dz * dz, j));
                }
            }
            candidates.sort_unstable();
            for &(_, j) in &candidates {
                let point = merged[j].0;
                if !blocks_bridge(&merged, point, corner)
                    && holes[h..]
                        .iter()
                        .all(|&(k, _)| !blocks_bridge(&contours[k].vertices, point, corner))
                {
                    bridge = Some((j, corner_index));
                    break;
                }
            }
            if bridge.is_some() {
                break;
            }
        }
        let Some((a, b)) = bridge else {
            return false;
        };
        merged = stitch(&merged, a, hole, b);
    }

    // Commit only once every hole is represented. Raw loops and contour
    // metadata stay available, including on the consumed (empty) contours.
    contours[outline].vertices = merged;
    for (i, _) in holes {
        contours[i].vertices.clear();
    }
    true
}

fn leftmost(vertices: &[Vertex]) -> usize {
    (0..vertices.len())
        .min_by_key(|&i| (vertices[i].0.x, vertices[i].0.z))
        .unwrap()
}

fn stitch(outline: &[Vertex], a: usize, hole: &[Vertex], b: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(outline.len() + hole.len() + 2);
    for (ring, start) in [(outline, a), (hole, b)] {
        vertices.extend(ring.iter().cycle().skip(start).take(ring.len()).copied());
        let (point, flags) = ring[start];
        // The duplicate departs along the bridge, not the original boundary.
        // Keep vertex flags, but do not label an internal bridge as a portal.
        let edge_flags = (RegionVertexId::REGION_MASK | RegionVertexId::AREA_BORDER).bits();
        vertices.push((point, flags & !edge_flags));
    }
    vertices
}

fn edges(vertices: &[Vertex]) -> impl Iterator<Item = (U16Vec3, U16Vec3)> + '_ {
    vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .map(|(a, b)| (a.0, b.0))
}

fn area2(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> i64 {
    let dx = i64::from(b.x) - i64::from(a.x);
    let dz = i64::from(b.z) - i64::from(a.z);
    dx * (i64::from(c.z) - i64::from(a.z)) - (i64::from(c.x) - i64::from(a.x)) * dz
}

fn same(a: U16Vec3, b: U16Vec3) -> bool {
    a.x == b.x && a.z == b.z
}

fn between(a: U16Vec3, b: U16Vec3, p: U16Vec3) -> bool {
    area2(a, b, p) == 0
        && (a.x.min(b.x)..=a.x.max(b.x)).contains(&p.x)
        && (a.z.min(b.z)..=a.z.max(b.z)).contains(&p.z)
}

fn intersects(a: U16Vec3, b: U16Vec3, c: U16Vec3, d: U16Vec3) -> bool {
    let opposite = |x: i64, y: i64| (x < 0 && y > 0) || (x > 0 && y < 0);
    (opposite(area2(a, b, c), area2(a, b, d)) && opposite(area2(c, d, a), area2(c, d, b)))
        || between(a, b, c)
        || between(a, b, d)
        || between(c, d, a)
        || between(c, d, b)
}

fn blocks_bridge(vertices: &[Vertex], a: U16Vec3, b: U16Vec3) -> bool {
    edges(vertices).any(|(c, d)| {
        // Shared endpoints are allowed (including duplicate bridge vertices),
        // but a collinear overlap with an incident edge is not.
        for (end, other) in [(a, b), (b, a)] {
            if same(end, c) {
                return between(end, other, d) || between(end, d, other);
            }
            if same(end, d) {
                return between(end, other, c) || between(end, c, other);
            }
        }
        intersects(a, b, c, d)
    })
}

fn in_cone(vertices: &[Vertex], i: usize, p: U16Vec3) -> bool {
    let v = vertices[i].0;
    let next = vertices[(i + 1) % vertices.len()].0;
    let prev = vertices[(i + vertices.len() - 1) % vertices.len()].0;
    // The walkable side is right of every directed edge, including hole edges.
    if area2(prev, v, next) <= 0 {
        area2(v, p, prev) < 0 && area2(p, v, next) < 0
    } else {
        !(area2(v, p, next) <= 0 && area2(p, v, prev) <= 0)
    }
}

fn is_simple(vertices: &[Vertex]) -> bool {
    for (i, (a, b)) in edges(vertices).enumerate() {
        if same(a, b) {
            return false;
        }
        let c = vertices[(i + 2) % vertices.len()].0;
        if between(a, b, c) || between(b, c, a) {
            return false;
        }
        for (j, (c, d)) in edges(vertices).enumerate().skip(i + 1) {
            if j == i + 1 || (i == 0 && j == vertices.len() - 1) {
                continue;
            }
            if intersects(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

fn rings_intersect(a: &[Vertex], b: &[Vertex]) -> bool {
    edges(a).any(|(p, q)| edges(b).any(|(r, s)| intersects(p, q, r, s)))
}

fn contains(vertices: &[Vertex], p: U16Vec3) -> bool {
    // Integer ray crossing, used only after ruling out boundary intersections.
    let mut inside = false;
    for (a, b) in edges(vertices) {
        if (a.z > p.z) != (b.z > p.z) && (area2(a, b, p) > 0) == (b.z > a.z) {
            inside = !inside;
        }
    }
    inside
}
