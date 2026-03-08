mod math {
  fn abs(x: i64) -> i64 {
    match x {
      n if (n < 0) => (-n),
      n => n,
    }
  }
}

