#![allow(missing_docs)]

use cgmath;
use cgmath::{Point3, EuclideanSpace, InnerSpace, ElementWise};
use collision::{Aabb3};
use rand::Rng;
use time;

use common::color::Color4;
use common::protocol;

use audio_loader;
use audio_thread;
use client;
use terrain;
use vertex::ColoredVertex;
use view;

const TRIANGLES_PER_BOX: u32 = 12;
const VERTICES_PER_TRIANGLE: u32 = 3;
const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

/// dispatch a server message among the subsystems
pub fn apply_server_update<UpdateView, UpdateAudio, UpdateServer, EnqueueTerrainLoad>(
  client               : &client::T,
  update_view          : &mut UpdateView,
  update_audio         : &mut UpdateAudio,
  update_server        : &mut UpdateServer,
  enqueue_terrain_load : &mut EnqueueTerrainLoad,
  update               : protocol::ServerToClient,
) where
  UpdateView         : FnMut(view::update::T),
  UpdateAudio        : FnMut(audio_thread::Message),
  UpdateServer       : FnMut(protocol::ClientToServer),
  EnqueueTerrainLoad : FnMut(terrain::Load),
{
  match update {
    protocol::ServerToClient::LeaseId(_) => {
      warn!("Client ID has already been leased.");
    },
    protocol::ServerToClient::Ping => {
      update_server(protocol::ClientToServer::Ping(client.id));
    },
    protocol::ServerToClient::PlayerAdded(id, _) => {
      warn!("Unexpected PlayerAdded event: {:?}.", id);
    },
    protocol::ServerToClient::UpdatePlayer(player_id, bounds) => {
      let mesh = to_triangles(&bounds, &Color4::of_rgba(0.0, 0.0, 1.0, 1.0));
      update_view(view::update::UpdatePlayer(player_id, mesh));

      // We "lock" the client to client.player_id, so for updates to that player only,
      // there is more client-specific logic.
      if player_id != client.player_id {
        return
      }

      let position =
        (bounds.min.to_vec().mul_element_wise(cgmath::Vector3::new(0.5, 0.1, 0.5))) +
        (bounds.max.to_vec().mul_element_wise(cgmath::Vector3::new(0.5, 0.9, 0.5)));
      let position = Point3::from_vec(position);

      *client.player_position.lock().unwrap() = position;
      update_view(view::update::MoveCamera(position));
    },
    protocol::ServerToClient::UpdateMob(id, bounds) => {
      let mesh = to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0));
      update_view(view::update::UpdateMob(id, mesh));
    },
    protocol::ServerToClient::UpdateSun(fraction) => {
      update_view(view::update::SetSun(
          view::light::Sun {
            progression: fraction,
            rotation: 0.0,
          }
          ));
    },
    protocol::ServerToClient::Voxels { voxels, reason } => {
      let time_requested;
      match reason {
        protocol::VoxelReason::Updated => {
          time_requested = None;
        },
        protocol::VoxelReason::Requested { at } => {
          time_requested = Some(at);
          debug!("Receiving a voxel request after {}ns", time::precise_time_ns() - at);
        },
      }

      enqueue_terrain_load(
        terrain::Load::Voxels {
          voxels       : voxels,
          time_requested : time_requested,
        }
        );
    },
    protocol::ServerToClient::Collision(collision_type) => {
      if let protocol::Collision::PlayerTerrain(..) = collision_type {
        let player_position = *client.player_position.lock().unwrap();
        let mut last_footstep = client.last_footstep.lock().unwrap();
        if (player_position - *last_footstep).magnitude() >= 4.0 {
          *last_footstep = player_position;
          let idx = client.rng.lock().unwrap().gen_range(1, 17 + 1);
          update_audio(audio_thread::Message::PlayOneShot(audio_loader::SoundId::Footstep(idx)));
        }
      }
    }
  }
}

fn to_triangles(
  bounds: &Aabb3<f32>,
  c: &Color4<f32>,
) -> [ColoredVertex; TRIANGLE_VERTICES_PER_BOX as usize] {
  let (x1, y1, z1) = (bounds.min.x, bounds.min.y, bounds.min.z);
  let (x2, y2, z2) = (bounds.max.x, bounds.max.y, bounds.max.z);

  let vtx = |x, y, z| {
    ColoredVertex {
      position: Point3::new(x, y, z),
      color: *c,
    }
  };

  // Remember: x increases to the right, y increases up, and z becomes more
  // negative as depth from the viewer increases.
  [
    // front
    vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
    vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
    // left
    vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
    vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
    // top
    vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
    vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
    // back
    vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
    vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
    // right
    vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
    vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
    // bottom
    vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
    vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
  ]
}
