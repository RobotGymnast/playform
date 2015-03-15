//! Define the updates passed from the client to the view.

use cgmath::Point3;
use std::iter::repeat;

use common::block_position::BlockPosition;
use common::color::Color3;
use common::entity::EntityId;
use common::lod::LODIndex;
use common::terrain_block::TerrainBlock;

use light::{Light, set_point_light, set_ambient_light};
use mob_buffers::VERTICES_PER_MOB;
use player_buffers::VERTICES_PER_PLAYER;
use vertex::ColoredVertex;
use view::View;

/// Messages from the client to the view.
pub enum ClientToView {
  /// Set the camera location.
  MoveCamera(Point3<f32>),

  /// Update a player mesh.
  UpdatePlayer(EntityId, [ColoredVertex; VERTICES_PER_PLAYER]),
  /// Update a mob mesh.
  UpdateMob(EntityId, [ColoredVertex; VERTICES_PER_MOB]),

  /// Update the point light.
  SetPointLight(Light),
  /// Update the ambient light.
  SetAmbientLight(Color3<f32>),
  /// Update the GL clear color.
  SetClearColor(Color3<f32>),

  /// Add a terrain block to the view.
  AddBlock((BlockPosition, TerrainBlock, LODIndex)),
  /// Remove a terrain entity.
  RemoveTerrain(EntityId),
  /// Remove block-specific data.
  RemoveBlockData(BlockPosition, LODIndex),
}

unsafe impl Send for ClientToView {}

#[allow(missing_docs)]
pub fn apply_client_to_view(up: ClientToView, view: &mut View) {
  match up {
    ClientToView::MoveCamera(position) => {
      view.camera.translate_to(position);
    },
    ClientToView::UpdateMob(id, triangles) => {
      view.mob_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::UpdatePlayer(id, triangles) => {
      view.player_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::SetPointLight(light) => {
      set_point_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        &light
      );
    },
    ClientToView::SetAmbientLight(color) => {
      set_ambient_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        color,
      );
    },
    ClientToView::SetClearColor(color) => {
      view.gl.set_background_color(color.r, color.g, color.b, 1.0);
    },
    ClientToView::AddBlock((position, block, lod)) => {
      let block_index =
        view.terrain_buffers.push_block_data(
          &mut view.gl,
          position,
          block.pixels.as_slice(),
          lod,
        );

      let block_indices: Vec<_> =
        repeat(block_index).take(block.ids.len()).collect();

      view.terrain_buffers.push(
        &mut view.gl,

        block.vertex_coordinates.as_slice(),
        block.normals.as_slice(),
        block.coords.as_slice(),
        block_indices.as_slice(),
        block.ids.as_slice(),
      );
    },
    ClientToView::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    ClientToView::RemoveBlockData(block_position, lod) => {
      view.terrain_buffers.free_block_data(lod, &block_position);
    },
  };
}