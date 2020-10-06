extern crate anyhow;
extern crate clap;
extern crate cpal;

use clap::{App, Arg, SubCommand};
use cpal::traits::{DeviceTrait, HostTrait};
use rodio::Sink;
use std::fs::File;
use std::io::BufReader;

fn main() {
  let matches = App::new("play")
    .version("0.1")
    .author("Duane R Bailey <duane@duane.cc>")
    .subcommand(SubCommand::with_name("list-devices"))
    .subcommand(SubCommand::with_name("play").arg(Arg::with_name("FILE").required(true)))
    .get_matches();

  if let Some(_) = matches.subcommand_matches("list-devices") {
    for host_id in cpal::available_hosts() {
      let host = match cpal::host_from_id(host_id) {
        Ok(h) => h,
        Err(_) => continue,
      };
      let devices = match host.devices() {
        Ok(d) => d,
        Err(_) => continue,
      };
      for device in devices.into_iter() {
        if let Ok(name) = device.name() {
          println!("DEVICE - {}", name);
        }
      }
      if let Some(default_device) = rodio::default_output_device() {
        if let Ok(name) = default_device.name() {
          println!("DEFAULT DEVICE - {}", name);
        }
      }
    }
  } else if let Some(matches) = matches.subcommand_matches("play") {
    let device = rodio::default_output_device().unwrap();

    let sink = Sink::new(&device);
    let file = File::open(&matches.value_of("FILE").unwrap()).unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    sink.append(source);

    sink.sleep_until_end();
  }
}
