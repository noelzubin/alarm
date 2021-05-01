use chrono::prelude::*;
use notify_rust::Notification;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Alarm {
    pub id: usize,
    pub time: NaiveDateTime,
    pub label: String,
}

impl Alarm {
    pub fn new(id: usize, time: NaiveDateTime, label: String) -> Alarm {
        Alarm { id, time, label }
    }

    pub fn notify(&self) {
        println!("ALARM DONE: {:#?}", self);
        Notification::new()
            .summary(&format!("ALARM: {}", self.label))
            .body(&format!("{}", self.time.format("%H:%M on %b %-d")))
            .show()
            .unwrap();
    }
}

fn get_data_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap();
    path.push(".alarm.json");
    path
}

pub fn write_data(data: &Vec<Alarm>) -> std::io::Result<()> {
    let data = serde_json::to_string(&data).unwrap();
    std::fs::write(get_data_path(), data)?;
    Ok(())
}

pub fn read_data() -> Vec<Alarm> {
    let data = std::fs::read_to_string(get_data_path()).unwrap();
    let alarms: Vec<Alarm> = serde_json::from_str(&data).unwrap();
    alarms
}

pub fn create_config_file() {
    let path = get_data_path();
    if !path.exists() {
        std::fs::write(path, b"[]").unwrap();
    }
}
