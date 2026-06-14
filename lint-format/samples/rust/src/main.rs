//! lint-format サンプル: rustfmt の整形と clippy の検査を通すための最小バイナリ。
//! 外部 crate を足さず std だけで `imports_granularity = "Crate"`（マージ）を見せる。

use std::collections::{HashMap, HashSet};

fn main() {
    let words = ["lint", "format", "lint", "showcase"];

    let mut counts: HashMap<&str, u32> = HashMap::new();
    for word in words {
        *counts.entry(word).or_insert(0) += 1;
    }

    let unique: HashSet<&str> = words.into_iter().collect();
    println!("{} unique of {} words", unique.len(), words.len());

    for (word, count) in &counts {
        println!("{word}: {count}");
    }
}
