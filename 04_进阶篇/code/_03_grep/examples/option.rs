fn main() {
  let s = Some(1);
  s.ok_or(()).unwrap();
  // s.ok_or_else(err);
}