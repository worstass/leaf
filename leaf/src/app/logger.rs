use crate::config;

use anyhow::{anyhow, Result};

pub fn setup_logger(config: &config::Log) -> Result<()> {
    let loglevel = match config.level.unwrap() {
        config::log::Level::TRACE => log::LevelFilter::Trace,
        config::log::Level::DEBUG => log::LevelFilter::Debug,
        config::log::Level::INFO => log::LevelFilter::Info,
        config::log::Level::WARN => log::LevelFilter::Warn,
        config::log::Level::ERROR => log::LevelFilter::Error,
    };
    let mut dispatch = fern::Dispatch::new()
        .format(move |out, message, record| {
            if *crate::option::LOG_NO_COLOR {
                #[cfg(not(target_os = "android"))]
                let d = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                #[cfg(target_os = "android")]
                let d = "";
                out.finish(format_args!(
                    "[{date}][{level}] {message}",
                    date = d,
                    level = record.level(),
                    message = message,
                ))
            } else {
                use fern::colors::{Color, ColoredLevelConfig};
                let colors_line = ColoredLevelConfig::new()
                    .error(Color::Red)
                    .warn(Color::Yellow)
                    .info(Color::White)
                    .debug(Color::White)
                    .trace(Color::BrightBlack);

                let colors_level = colors_line.info(Color::Green);
                #[cfg(not(target_os = "android"))]
                let d = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                #[cfg(target_os = "android")]
                let d = "";
                out.finish(format_args!(
                    // "{color_line}[{date}][{level}{color_line}][{target}] {message}\x1B[0m",
                    "{color_line}[{date}][{level}{color_line}] {message}\x1B[0m",
                    color_line = format_args!(
                        "\x1B[{}m",
                        colors_line.get_color(&record.level()).to_fg_str()
                    ),
                    date = d,
                    // target = record.target(),
                    level = colors_level.color(record.level()),
                    message = message,
                ))
            }
        })
        .level(log::LevelFilter::Warn)
        .level_for("rust_tun", loglevel)
        .level_for("netstack_lwip", loglevel)
        .level_for("leaf", loglevel);

    match config.output.unwrap() {
        config::log::Output::CONSOLE => {
            #[cfg(any(target_os = "ios", target_os = "android"))]
            {
                let console_output = fern::Output::writer(
                    Box::new(crate::mobile::logger::ConsoleWriter::default()),
                    "\n",
                );
                dispatch = dispatch.chain(console_output);
            }
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            {
                dispatch = dispatch.chain(fern::Output::stdout("\n"));
            }
            #[cfg(target_os = "macos")]
            if *crate::option::LOG_CONSOLE_OUT {
                let console_output = fern::Output::writer(
                    Box::new(crate::mobile::logger::ConsoleWriter::default()),
                    "\n",
                );
                dispatch = dispatch.chain(console_output);
            }
        }
        config::log::Output::FILE => {
            let f = fern::log_file(&config.output_file)?;
            let file_output = fern::Output::file(f, "\n");
            dispatch = dispatch.chain(file_output);
        }
    }

    if let Err(e) = dispatch.apply() {
        return Err(anyhow!("apply logger config failed: {}", e));
    }

    Ok(())
}
