fn main() {
    #[cfg(not(feature = "dedup"))]
    println!("Feature dedup is switched off");

    #[cfg(feature = "dedup")]
    {
        flexi_logger::Logger::with_str("info")
            .format(flexi_logger::colored_detailed_format)
            .log_to_stdout()
            .dedup(std::num::NonZeroUsize::new(2).unwrap())
            .start()
            .unwrap();

        for i in 0..10 {
            log::info!("{}", if i == 5 { "bar" } else { "foo" });
        }

        log::info!("the end");
    }
}
