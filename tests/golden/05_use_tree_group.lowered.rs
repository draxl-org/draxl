mod io {
  use std::collections::{BTreeMap, BTreeSet};

  use std::cmp::{self, *};

  fn total(x: i64, y: i64) -> i64 {
    cmp::min(x, y);
    max(x, y)
  }
}

