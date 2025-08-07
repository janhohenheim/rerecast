use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_math::bounding::Aabb3d;
use bevy_platform::collections::HashSet;
use bevy_reflect::prelude::*;
use bevy_transform::prelude::*;
use glam::Vec3;
use rerecast::{BuildContoursFlags, ConfigBuilder, ConvexVolume, TriMesh};
use serde::{Deserialize, Serialize};

/// The current backend registered through [`NavmeshApp::set_navmesh_affector_backend`]
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct NavmeshAffectorBackend(
    pub SystemId<In<NavmeshSettings>, Vec<(GlobalTransform, TriMesh)>>,
);

/// Extension used to implement [`NavmeshApp::set_navmesh_affector_backend`] on [`App`]
pub trait NavmeshApp {
    /// Set the backend for generating navmesh affectors. Only one backend can be set at a time.
    /// Setting a backend will replace any existing backend. By default, no backend is set.
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<In<NavmeshSettings>, Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App;
}

impl NavmeshApp for App {
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<In<NavmeshSettings>, Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App {
        let id = self.register_system(system);
        self.world_mut().insert_resource(NavmeshAffectorBackend(id));
        self
    }
}

/// The input passed to the navmesh affector backend system.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct NavmeshSettings {
    /// How many cells should fit in the [`Self::agent_radius`] on the horizontal plane to use for fields. `[Limit: > 0]`.
    ///
    /// The voxelization cell size defines the voxel size along both axes of the ground plane: x and z in Recast.
    /// The resulting value is derived from the character radius r. For example, setting `cell_size_fraction` to 2 will result in the
    /// cell size being r/2, where r is [`Self::agent_radius`].
    ///
    /// A recommended starting value for cell_size is either 2 or 3.
    /// Larger values of cell_size will increase rasterization resolution and navmesh detail, but total generation time will increase exponentially.
    /// In outdoor environments, 2 is often good enough. For indoor scenes with tight spaces you might want the extra precision,
    /// so a value of 3 or higher may give better results.
    ///
    /// The initial instinct is to reduce this value to something very high to maximize the detail of the generated navmesh.
    /// This quickly becomes a case of diminishing returns, however. Beyond a certain point there's usually not much perceptable difference
    /// in the generated navmesh, but huge increases in generation time.
    /// This hinders your ability to quickly iterate on level designs and provides little benefit.
    /// The general recommendation here is to use as small a value for cell_size as you can get away with.
    ///
    /// [`Self::cell_size_fraction`] and [`Self::cell_height_fraction`] define voxel/grid/cell size. So their values have significant side effects on all parameters defined in voxel units.
    ///
    /// The maximum value for this parameter depends on the platform's floating point accuracy,
    /// with the practical maximum usually such that [`Self::agent_radius`] / [`Self::cell_size_fraction`] = 0.05.
    pub cell_size_fraction: f32,
    /// How many cells should fit in the [`Self::agent_height`] on the up-axis to use for fields. `[Limit: > 0]`
    ///
    /// The voxelization cell height is defined separately in order to allow for greater precision in height tests.
    /// A good starting point for [`Self::cell_height_fraction`] is twice the size of [`Self::cell_size_fraction`].
    /// Higher [`Self::cell_height_fraction`] values ensure that the navmesh properly connects areas that are only separated by a small curb or ditch.
    /// If small holes are generated in your navmesh around where there are discontinuities in height (for example, stairs or curbs),
    /// you may want to increase the cell height value to increase the vertical rasterization precision of rerecast.
    ///
    /// [`Self::cell_size_fraction`] and [`Self::cell_height_fraction`] define voxel/grid/cell size. So their values have significant side effects on all parameters defined in voxel units.
    ///
    /// The minimum value for this parameter depends on the platform's floating point accuracy, with the practical minimum usually usually such that [`Self::agent_radius`] / [`Self::cell_height_fraction`] = 0.05.
    pub cell_height_fraction: f32,
    /// The height of the agent. `[Limit: > 0] [Units: wu]`
    ///
    /// It's often a good idea to add a little bit of padding to the height. For example,
    /// an agent that is 1.8 world units tall might want to set this value to 2.0 units.
    pub agent_height: f32,
    /// The radius of the agent. `[Limit: > 0] [Units: wu]`
    pub agent_radius: f32,
    /// Maximum ledge height that is considered to still be traversable. `[Limit: >=0] [Units: wu]`
    ///
    /// The walkable_climb value defines the maximum height of ledges and steps that the agent can walk up.
    ///
    /// Allows the mesh to flow over low lying obstructions such as curbs and up/down stairways.
    /// The value is usually set to how far up/down an agent can step.
    pub walkable_climb: f32,
    /// The maximum slope that is considered walkable. `[Limits: 0 <= value < 0.5*π] [Units: Radians]`
    ///
    /// The parameter walkable_slope_angle is to filter out areas of the world where the ground slope
    /// would be too steep for an agent to traverse.
    /// This value is defined as a maximum angle in degrees that the surface normal of a polygon can differ from the world's up vector.
    /// This value must be within the range `[0, 90.0.to_radians()]`.
    ///
    /// The practical upper limit for this parameter is usually around `85.0.to_radians()`.
    pub walkable_slope_angle: f32,
    /// The minimum number of cells allowed to form isolated island areas along one horizontal axis. `[Limit: >=0] [Units: vx]`
    ///
    /// Watershed partitioning is really prone to noise in the input distance field.
    /// In order to get nicer areas, the areas are merged and small disconnected areas are removed after the water shed partitioning.
    /// The parameter [`Self::min_region_size`] describes the minimum isolated region size that is still kept.
    /// A region is removed if the number of voxels in the region is less than the square of [`Self::min_region_size`].
    ///
    /// Any regions that are smaller than this area will be marked as unwalkable.
    /// This is useful in removing useless regions that can sometimes form on geometry such as table tops, box tops, etc.
    pub min_region_size: u16,
    /// Any regions with a span count smaller than the square of this value will, if possible,
    /// be merged with larger regions. `[Limit: >=0] [Units: vx]`
    ///
    /// The triangulation process works best with small, localized voxel regions.
    /// The parameter [`Self::merge_region_size`] controls the maximum voxel area of a region that is allowed to be merged with another region.
    /// If you see small patches missing here and there, you could lower the [`Self::min_region_size`] value.
    pub merge_region_size: u16,
    /// The maximum allowed length for contour edges along the border of the mesh in terms of [`Self::agent_radius`]. `[Limit: >=0]`
    ///
    /// In certain cases, long outer edges may decrease the quality of the resulting triangulation, creating very long thin triangles.
    /// This can sometimes be remedied by limiting the maximum edge length, causing the problematic long edges to be broken up into smaller segments.
    ///
    /// The parameter [`Self::edge_max_len_factor`] defines the maximum edge length and is defined in terms of world units.
    /// A good value for [`Self::edge_max_len_factor`] is something like 8.
    /// A good way to adjust this value is to first set it really high and see if your data creates long edges.
    /// If it does, decrease [`Self::edge_max_len_factor`] until you find the largest value which improves the resulting tesselation.
    ///
    /// Extra vertices will be inserted as needed to keep contour edges below this length.
    /// A value of zero effectively disables this feature.
    pub edge_max_len_factor: u16,
    /// The maximum distance a simplified contour's border edges should deviate
    /// the original raw contour. `[Limit: >=0] [Units: vx]`
    ///
    /// When the rasterized areas are converted back to a vectorized representation,
    /// the max_simplification_error describes how loosely the simplification is done.
    /// The simplification process uses the Ramer–Douglas-Peucker algorithm, and this value describes the max deviation in voxels.
    ///
    /// Good values for max_simplification_error are in the range `[1.1, 1.5]`.
    /// A value of 1.3 is a good starting point and usually yields good results.
    /// If the value is less than 1.1, some sawtoothing starts to appear at the generated edges.
    /// If the value is more than 1.5, the mesh simplification starts to cut some corners it shouldn't.
    ///
    /// The effect of this parameter only applies to the horizontal plane.
    pub max_simplification_error: f32,
    /// The maximum number of vertices allowed for polygons generated during the
    /// contour to polygon conversion process. `[Limit: >= 3]`
    pub max_vertices_per_polygon: u16,
    /// Sets the sampling distance to use when generating the detail mesh.
    /// (For height detail only.) `[Limits: 0 or >= 0.9] [Units: wu]`
    ///
    /// When this value is below 0.9, it will be clamped to 0.0.
    pub detail_sample_dist: f32,
    /// The maximum distance the detail mesh surface should deviate from heightfield
    /// data. (For height detail only.) `[Limit: >=0] [Units: wu]`
    pub detail_sample_max_error: f32,
    /// The width/height size of tiles on the horizontal plane. `[Limit: >= 0] [Units: vx]`
    ///
    /// This field is only used when building multi-tile meshes, i.e. when [`Self::tiling`] is `true`.
    pub tile_size: u16,
    /// The navmesh's AABB [Units: wu]
    ///
    /// If left at `None`, the AABB will be automatically computed based on the available navmesh affectors.
    pub aabb: Option<Aabb3d>,
    /// Flags controlling the [`ContourSet`](crate::rerecast::ContourSet) generation process.
    pub contour_flags: BuildContoursFlags,
    /// Whether the navmesh should be tiled or not.
    pub tiling: bool,
    /// Volumes that define areas with specific areas IDs.
    pub area_volumes: Vec<ConvexVolume>,
    /// An optional list of entities to generate navmesh affectors for.
    /// If `Some`, the backend is expected to only consider the specified entities when generating affectors.
    /// If `None`, the backend is expected to generate affectors for as many entities as is reasonable.
    pub filter: Option<HashSet<Entity>>,
    /// The direction considered up. The following values are supported:
    /// - [`Vec3::Y`]: Typically used in 3D
    /// - [`Vec3::Z`]: Typically used in 2D
    /// - [`Vec3::X`]
    pub up: Vec3,
}

impl Default for NavmeshSettings {
    fn default() -> Self {
        let cfg = ConfigBuilder::default();
        Self {
            agent_height: cfg.agent_height,
            agent_radius: cfg.agent_radius,
            walkable_climb: cfg.walkable_climb,
            walkable_slope_angle: cfg.walkable_slope_angle,
            min_region_size: cfg.min_region_size,
            merge_region_size: cfg.merge_region_size,
            max_simplification_error: cfg.max_simplification_error,
            max_vertices_per_polygon: cfg.max_vertices_per_polygon,
            detail_sample_dist: cfg.detail_sample_dist,
            detail_sample_max_error: cfg.detail_sample_max_error,
            tile_size: cfg.tile_size,
            aabb: None,
            contour_flags: cfg.contour_flags,
            tiling: cfg.tiling,
            area_volumes: cfg.area_volumes,
            filter: None,
            cell_size_fraction: cfg.cell_size_fraction,
            cell_height_fraction: cfg.cell_height_fraction,
            edge_max_len_factor: cfg.edge_max_len_factor,
            up: Vec3::Y,
        }
    }
}

impl NavmeshSettings {
    /// Creates a new [`NavmeshSettings`] instance from a 3D agent's radius and height.
    pub fn from_agent_3d(radius: f32, height: f32) -> Self {
        Self {
            agent_radius: radius,
            agent_height: height,
            up: Vec3::Y,
            ..Self::default()
        }
    }

    /// Creates a new [`NavmeshSettings`] instance from a 2D agent's radius and height.
    pub fn from_agent_2d(radius: f32, height: f32) -> Self {
        Self {
            agent_radius: radius,
            agent_height: height,
            up: Vec3::Z,
            ..Self::default()
        }
    }

    pub(crate) fn into_rerecast_config(self) -> rerecast::ConfigBuilder {
        rerecast::ConfigBuilder {
            agent_height: self.agent_height,
            agent_radius: self.agent_radius,
            walkable_climb: self.walkable_climb,
            walkable_slope_angle: self.walkable_slope_angle,
            min_region_size: self.min_region_size,
            merge_region_size: self.merge_region_size,
            max_simplification_error: self.max_simplification_error,
            max_vertices_per_polygon: self.max_vertices_per_polygon,
            detail_sample_dist: self.detail_sample_dist,
            detail_sample_max_error: self.detail_sample_max_error,
            tile_size: self.tile_size,
            aabb: self
                .aabb
                .map(|aabb| rerecast::Aabb3d {
                    min: aabb.min.into(),
                    max: aabb.max.into(),
                })
                .unwrap_or_default(),
            contour_flags: self.contour_flags,
            tiling: self.tiling,
            area_volumes: self.area_volumes,
            cell_size_fraction: self.cell_size_fraction,
            cell_height_fraction: self.cell_height_fraction,
            edge_max_len_factor: self.edge_max_len_factor,
        }
    }
}
