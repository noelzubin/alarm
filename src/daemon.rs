use chrono::prelude::*;
use std::io::prelude::*;
use std::io::BufRead;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub const SOCKET_PATH: &'static str = "/tmp/alarm";

type Sender = Arc<Mutex<std::sync::mpsc::Sender<()>>>;

#[derive(Deserialize, Serialize)]
pub enum Request {
    /// create new alarm with time string and label
    New(String, String),
    /// List all alarms
    List,
    /// delete an alarm by id
    Del(usize),
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    String(String),
    Alarms(Vec<crate::data::Alarm>),
}

fn start_stream(tx: std::sync::mpsc::Sender<()>) -> std::io::Result<()> {
    // remove previous socket file
    std::fs::remove_file(SOCKET_PATH).unwrap_or_else(|e| match e.kind() {
        std::io::ErrorKind::NotFound => (),
        _ => panic!("{}", e),
    });

    crate::data::create_config_file();

    let tx = Arc::new(Mutex::new(tx));
    let listener = UnixListener::bind(SOCKET_PATH)?;
    for stream in listener.incoming() {
        let tx = tx.clone();
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream, tx));
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

fn handle_client(stream: UnixStream, tx: Sender) -> std::io::Result<()> {
    let mut req = String::new();
    let mut reader = std::io::BufReader::new(&stream);
    dbg!("reading");
    reader.read_line(&mut req)?;
    let req: Request = serde_json::from_str(&req).unwrap();
    let _ = handle_request(req, tx, stream);
    Ok(())
}

fn find_next_id(ids: Vec<usize>) -> usize {
    (0..9999).find(|i| !ids.contains(i)).unwrap()
}

fn handle_request(req: Request, tx: Sender, mut stream: UnixStream) -> std::io::Result<()> {
    match req {
        Request::New(time, label) => {
            let mut alarms = crate::data::read_data();
            let next_id = find_next_id(alarms.iter().map(|a| a.id).collect());
            let time = crate::time::parse_time(time);
            alarms.push(crate::data::Alarm::new(next_id, time, label));
            crate::data::write_data(&alarms).unwrap();

            let msg = get_remaining_time_msg(&time);
            println!("{}", &msg);
            tx.lock().unwrap().send(()).unwrap();
            let response = Response::String(msg);
            writeln!(stream, "{}", serde_json::to_string(&response).unwrap()).unwrap();
            stream.flush().unwrap();
        }
        Request::List => {
            let alarms = crate::data::read_data();
            println!("listing alarms");
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&Response::Alarms(alarms)).unwrap()
            )
            .unwrap();
            stream.flush().unwrap();
        }
        Request::Del(id) => {
            let alarms = crate::data::read_data();
            let alarms: Vec<crate::data::Alarm> =
                alarms.into_iter().filter(|a| a.id != id).collect();
            crate::data::write_data(&alarms).unwrap();
            println!("deleted alarm");
            tx.lock().unwrap().send(()).unwrap();
            let response = Response::String("deleted alarm".into());
            writeln!(stream, "{}", serde_json::to_string(&response).unwrap()).unwrap();
            stream.flush().unwrap();
        }
    }
    Ok(())
}

pub fn start() {
    let (tx, rx) = std::sync::mpsc::channel();
    let server_handle = thread::spawn(move || {
        start_stream(tx).unwrap();
    });

    let scheduler = Scheduler::new();

    // handle from server
    while let Ok(()) = rx.recv() {
        scheduler.reschedule();
    }

    server_handle.join().unwrap();
}

#[derive(Clone)]
struct Scheduler;

impl Scheduler {
    fn new() -> Self {
        let scheduler = Scheduler;
        scheduler.notify();
        scheduler.reschedule();
        scheduler
    }

    // notify pending and deleted notified.
    fn notify(&self) {
        let now: DateTime<Local> = Local::now();
        let now = now.naive_local();
        let alarms = crate::data::read_data();
        let past: Vec<&crate::data::Alarm> = alarms.iter().filter(|a| a.time <= now).collect();

        past.iter().for_each(|a| {
            a.notify();
        });

        let remaining: Vec<crate::data::Alarm> =
            alarms.into_iter().filter(|a| a.time > now).collect();

        crate::data::write_data(&remaining).unwrap();
    }

    // set timer for next alarm
    fn reschedule(&self) {
        let next = Scheduler::find_next();

        if next.is_none() {
            return;
        };

        let next = next.unwrap();

        let now: DateTime<Local> = Local::now();
        let duration = next
            .signed_duration_since(now.naive_local())
            .to_std()
            .unwrap();

        let cloned = self.clone();

        thread::spawn(move || {
            thread::sleep(duration);
            cloned.notify();
            cloned.reschedule();
        });
    }

    fn find_next() -> Option<NaiveDateTime> {
        let now: DateTime<Local> = Local::now();
        let alarms = crate::data::read_data();
        alarms
            .iter()
            .map(|a| a.time)
            .filter(|t| t > &now.naive_local())
            .min()
    }
}

fn get_remaining_time_msg(next: &NaiveDateTime) -> String {
    let now: DateTime<Local> = Local::now();
    let duration = next.signed_duration_since(now.naive_local());

    let mut msg = String::from("Alarm set for");
    if duration.num_hours() > 0 {
        msg.push_str(&format!(" {} hours", duration.num_hours()));
    };

    if duration.num_minutes() % 60 > 0 {
        msg.push_str(&format!(" {} minutes", duration.num_minutes() % 60));
    };

    msg.push_str(" from now.");

    msg
}
