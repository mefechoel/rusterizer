extern crate time;

pub fn timestamp() -> f64 {
  let timespec = time::get_time();
  let mills: f64 = timespec.sec as f64 * 1000.0 + (
    f64::from(timespec.nsec) / 1000.0 / 1000.0
  );
  mills
}
