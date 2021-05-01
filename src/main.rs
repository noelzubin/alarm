#[macro_use]
extern crate nom;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate prettytable;

use daemon::{Request, Response, SOCKET_PATH};
use prettytable::Table;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use structopt::StructOpt;

mod daemon;
mod data;
mod time;

#[derive(StructOpt)]
enum Opt {
    New { time: String, label: String },
    List,
    Del { id: usize },
    Daemon,
}

fn main() {
    let opts = Opt::from_args();
    match opts {
        Opt::Daemon => {
            daemon::start();
        }
        Opt::New { time, label } => {
            let mut stream = connect_socket();
            let request = Request::New(time, label);
            write_request(&mut stream, &request);
            let resp = get_response(&mut stream);
            if let Response::String(resp) = serde_json::from_str(&resp).unwrap() {
                println!("{}", resp);
            }
        }
        Opt::Del { id } => {
            let mut stream = connect_socket();
            let request = Request::Del(id);
            write_request(&mut stream, &request);
            serde_json::to_string(&request).unwrap();
            let resp = get_response(&mut stream);
            dbg!(resp);
        }
        Opt::List => {
            let mut stream = connect_socket();
            let request = Request::List;
            write_request(&mut stream, &request);
            let resp = get_response(&mut stream);
            let resp: Response = serde_json::from_str(&resp).unwrap();
            if let Response::Alarms(alarms) = resp {
                display_table(alarms);
            }
        }
    }
}

fn connect_socket() -> UnixStream {
    UnixStream::connect(SOCKET_PATH).unwrap()
}

fn write_request(stream: &mut UnixStream, request: &Request) {
    let request = serde_json::to_string(request).unwrap();
    writeln!(stream, "{}", request).unwrap();
    stream.flush().unwrap();
}

fn get_response(stream: &mut UnixStream) -> String {
    let mut resp = String::new();
    let mut stream = std::io::BufReader::new(stream);
    stream.read_line(&mut resp).unwrap();
    resp
}

fn display_table(alarms: Vec<data::Alarm>) {
    let mut table = Table::new();

    alarms.iter().for_each(|a| {
        let time_str = a.time.format("%H:%M on %b %-d");
        table.add_row(row![a.id, time_str, a.label]);
    });

    table.printstd();
}
