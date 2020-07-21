fn main() {
    #[cfg(feature = "colors")]
    for i in 0..=255 {
        println!("{}: {}", i, yansi::Paint::fixed(i, i));
    }
}
