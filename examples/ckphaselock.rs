use argh::FromArgs;
use std::{path::PathBuf, time::Duration};

#[derive(FromArgs)]
/// Run a clockkit client.
struct CmdlineConf {
    /// configuration file.
    #[argh(positional)]
    config_file: PathBuf,
}

fn main() {
    let args: CmdlineConf = argh::from_env();
    let config = clockkit::ConfigReader::from_config_file(args.config_file).unwrap();
    let plc = config.build_clock();

    plc.start();

    loop {
        if let Ok(ts) = plc.get_value() {
            println!("Value: {:?}", ts);
        } else {
            println!("Out of sync.");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
