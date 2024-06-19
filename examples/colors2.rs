// #[cfg(feature = "colors")]
// use colored::{Color, ColoredString, Colorize};
// use flexi_logger::color_from_ansi_code;
//

fn main() {
    //     #[cfg(not(feature = "colors"))]
    //     println!("Feature color is switched off");

    //     #[cfg(feature = "colors")]
    //     {
    //         use std::io::IsTerminal;;

    //         colored::control::set_override(true);

    //         for r in [0, 95, 135, 175, 215, 255] {
    //             for g in [0, 95, 135, 175, 215, 255] {
    //                 for b in [0, 95, 135, 175, 215, 255] {
    //                     println!(
    //                         "   rgb = ({:3}, {:3}, {:3}), {}",
    //                         r,
    //                         g,
    //                         b,
    //                         "hello".truecolor(r, g, b)
    //                     );
    //                 }
    //             }
    //         }

    //         for i in 0..=255 {
    //             print!("{}: {}", i, nu_ansi_term::Color::Fixed(i).paint(i.to_string()));
    //             println!("{}: {}", i, i.to_string().color(color_from_ansi_code(i)));
    //         }

    //         println!();

    //         if std::io::stdout().is_terminal() {
    //             println!(
    //                 "Stdout is considered a tty - \
    //                  flexi_logger::AdaptiveFormat will use colors",
    //             );
    //         } else {
    //             println!(
    //                 "Stdout is not considered a tty - \
    //                  flexi_logger::AdaptiveFormat will NOT use colors"
    //             );
    //         }

    //         if std::io::stderr().is_terminal() {
    //             println!(
    //                 "Stderr is considered a tty - \
    //                  flexi_logger::AdaptiveFormat will use colors",
    //             );
    //         } else {
    //             println!(
    //                 "Stderr is not considered a tty - \
    //                  flexi_logger::AdaptiveFormat will NOT use colors!"
    //             );
    //         }

    //         #[cfg(target_os = "windows")]
    //         if nu_ansi_term::enable_ansi_support().is_err() {
    //             println!("Unsupported windows console detected, coloring will likely not work");
    //         }

    //         println!(
    //             "\n{}",
    //             "err! output (red) with default palette"
    //                 .color(color_from_ansi_code(196))
    //                 .bold()
    //         );
    //         println!(
    //             "{}",
    //             "warn! output (yellow) with default palette"
    //                 .color(color_from_ansi_code(208))
    //                 .bold()
    //         );
    //         println!("info! output (normal) with default palette");
    //         println!(
    //             "{}",
    //             "debug! output (normal) with default palette"
    //                 .color(color_from_ansi_code(7))
    //                 .bold()
    //         );
    //         println!(
    //             "{}",
    //             "trace! output (grey) with default palette"
    //                 .color(color_from_ansi_code(8))
    //                 .bold()
    //         );

    //         println!("\n{}", "err! output (red) with env_logger-palette".red());
    //         println!(
    //             "{}",
    //             "warn! output (yellow) with env_logger-palette".yellow()
    //         );
    //         println!("{}", "info! output (green) with env_logger-palette".green());
    //         println!("{}", "debug! output (blue) with env_logger-palette".blue());
    //         println!("{}", "trace! output (cyan) with env_logger-palette".cyan());
    //     }
}
