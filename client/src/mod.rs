//! This crate contains client-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(env)]
#![feature(old_io)]
#![feature(old_path)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate cgmath;
extern crate common;
extern crate env_logger;
extern crate gl;
#[macro_use]
extern crate log;
extern crate libc;
extern crate nanomsg;
extern crate "rustc-serialize" as rustc_serialize;
extern crate sdl2;
extern crate "sdl2-sys" as sdl2_sys;
extern crate test;
extern crate time;
extern crate yaglw;

mod camera;
mod client;
mod client_update;
mod fontloader;
mod hud;
mod light;
mod mob_buffers;
mod process_event;
mod render;
mod server_thread;
mod shaders;
mod surroundings_thread;
mod terrain_buffers;
mod terrain_thread;
mod ttf;
mod view;
mod view_thread;
mod view_update;

use client::Client;
use common::communicate::ClientToServer;
use common::socket::{SendSocket, ReceiveSocket};
use std::ops::Deref;
use std::sync::mpsc::channel;
use std::sync::{Arc, Future, Mutex};
use view_thread::view_thread;

/// Entry point.
pub fn main() {
  env_logger::init().unwrap();

  debug!("starting");

  let mut args = std::env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/client.ipc"));
  let server_listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Sending to {}.", server_listen_url);
  info!("Listening on {}.", listen_url);

  let (client_to_view_send, client_to_view_recv) = channel();
  let (terrain_to_load_send, terrain_to_load_recv) = channel();

  let (ups_from_server_send, ups_from_server_recv) = channel();
  let listen =
    ReceiveSocket::spawn(
      listen_url.as_slice(),
      move |msg| ups_from_server_send.send(msg).unwrap(),
    );
  let ups_from_server = ups_from_server_recv;
  let ups_to_server = SendSocket::spawn(server_listen_url.as_slice());

  ups_to_server.send(ClientToServer::Init(listen_url));

  let client = Client::new();
  let client = Arc::new(client);

  let ups_to_server = Arc::new(Mutex::new(ups_to_server));
  let client_to_view_send = Arc::new(Mutex::new(client_to_view_send));

  let server_thread = {
    let client = client.clone();
    let client_to_view_send = client_to_view_send.clone();

    Future::spawn(move || {
      server_thread::server_thread(
        client.deref(),
        &ups_from_server,
        &mut |msg| { client_to_view_send.lock().unwrap().send(msg).unwrap() },
        &mut |msg| { terrain_to_load_send.send(msg).unwrap() },
      );

      (ups_from_server, terrain_to_load_send)
    })
  };

  let _surroundings_thread = {
    let client = client.clone();
    let ups_to_server = ups_to_server.clone();
    let client_to_view_send = client_to_view_send.clone();

    Future::spawn(move || {
      surroundings_thread::surroundings_thread(
        client.deref(),
        &mut |msg| { client_to_view_send.lock().unwrap().send(msg).unwrap() },
        &mut |msg| { ups_to_server.lock().unwrap().send(msg) },
      );
    })
  };

  let _terrain_thread = {
    let client = client.clone();
    let client_to_view_send = client_to_view_send.clone();

    Future::spawn(move || {
      terrain_thread::terrain_thread(
        client.deref(),
        &terrain_to_load_recv,
        &mut |msg| { client_to_view_send.lock().unwrap().send(msg).unwrap() },
      );

      terrain_to_load_recv
    })
  };

  view_thread(
    &client_to_view_recv,
    &mut |msg| { ups_to_server.lock().unwrap().send(msg) },
  );

  let _ups_from_server = server_thread.into_inner();

  listen.close();

  debug!("Done.");
}
