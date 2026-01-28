# 0.3.0

- Bump deps

# 0.2.0

- Rename "navmesh affector backend" to just "navmesh backend"
- Rename all remaining instances of "affector" to "obstacle"
- Navmesh backends now return a single `TriMesh` that contains all geometry in global coordinates.
If there are no ostacles, return `TriMesh::default()`.

# 0.1.0

Initial release, hurray!
