extern crate time;

pub type Timestamp = f64;

pub fn timestamp() -> Timestamp {
  let timespec = time::get_time();
  let mills: Timestamp = timespec.sec as Timestamp + (
    timespec.nsec as Timestamp / 1000.0 / 1000.0 / 1000.0
  );
  mills
}
