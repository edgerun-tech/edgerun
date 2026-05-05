use edgerun_machine_report::{gather_machine_report, render_machine_report, OutputFormat};

fn usage() {
    eprintln!("usage: edgerun-machine-report [text|json|--format <text|json>]");
}

fn parse_format(args: &[String]) -> Result<OutputFormat, String> {
    match args {
        [_program] => Ok(OutputFormat::Text),
        [_program, format] if format == "text" => Ok(OutputFormat::Text),
        [_program, format] if format == "json" => Ok(OutputFormat::Json),
        [_program, flag, format] if flag == "--format" && format == "text" => {
            Ok(OutputFormat::Text)
        }
        [_program, flag, format] if flag == "--format" && format == "json" => {
            Ok(OutputFormat::Json)
        }
        _ => Err("invalid arguments".to_owned()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let format = match parse_format(&args) {
        Ok(format) => format,
        Err(_) => {
            usage();
            std::process::exit(2);
        }
    };

    let report = gather_machine_report();
    match render_machine_report(&report, format) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
