fn main() {
    #[cfg(not(feature = "colors"))]
    println!("Feature color is switched off");

    #[cfg(feature = "colors")]
    {
        use atty::Stream::{Stderr, Stdout};
        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

        let mut stdout = StandardStream::stdout(ColorChoice::AlwaysAnsi);
        let mut color_spec = ColorSpec::new();

        for i in 0..=255 {
            print!("{}: ", i);
            color_spec.set_fg(Some(Color::Ansi256(i)));
            stdout.set_color(&color_spec).ok();
            println!("{}", i);
            stdout.reset().ok();
        }

        println!();

        if atty::is(Stdout) {
            println!(
                "Stdout is considered a tty - \
                 flexi_logger::AdaptiveFormat will use colors",
            );
        } else {
            println!(
                "Stdout is not considered a tty - \
                 flexi_logger::AdaptiveFormat will NOT use colors"
            );
        }

        if atty::is(Stderr) {
            println!(
                "Stderr is considered a tty - \
                 flexi_logger::AdaptiveFormat will use colors",
            );
        } else {
            println!(
                "Stderr is not considered a tty - \
                 flexi_logger::AdaptiveFormat will NOT use colors!"
            );
        }

        #[cfg(target_os = "windows")]
        if ansi_term::enable_ansi_support().is_err() {
            println!("Unsupported windows console detected, coloring will likely not work");
        }

        color_spec.set_fg(Some(Color::Ansi256(196))).set_bold(true);
        stdout.set_color(&color_spec).ok();
        println!("\n{}", "err! output (red) with default palette");
        color_spec.set_fg(Some(Color::Ansi256(208))).set_bold(true);
        stdout.set_color(&color_spec).ok();
        println!("{}", "warn! output (yellow) with default palette");
        stdout.reset().ok();
        println!("info! output (normal) with default palette");
        color_spec.set_fg(Some(Color::Ansi256(27))).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "debug! output (normal) with default palette");
        color_spec.set_fg(Some(Color::Ansi256(8))).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "trace! output (grey) with default palette");

        color_spec.set_fg(Some(Color::Red)).set_bold(true);
        stdout.set_color(&color_spec).ok();
        println!("\n{}", "err! output (red) with env_logger-palette");
        color_spec.set_fg(Some(Color::Yellow)).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "warn! output (yellow) with env_logger-palette");
        color_spec.set_fg(Some(Color::Green)).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "info! output (green) with env_logger-palette");
        color_spec.set_fg(Some(Color::Blue)).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "debug! output (blue) with env_logger-palette");
        color_spec.set_fg(Some(Color::Cyan)).set_bold(false);
        stdout.set_color(&color_spec).ok();
        println!("{}", "trace! output (cyan) with env_logger-palette");
    }
}
