use edgerun_machine_report::{
    gather_deployment_machine_inventory, gather_machine_capability_report_for_host,
    gather_machine_report, render_deployment_machine_inventory, render_machine_capability_report,
    render_machine_report,
};

fn usage() {
    eprintln!("usage: edgerun-machine-report [text|capability-report|deployment-inventory]");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("capability-report") {
        let report = gather_machine_capability_report_for_host(args.get(2).map(String::as_str));
        print!("{}", render_machine_capability_report(&report));
        return;
    }
    if args.get(1).map(String::as_str) == Some("deployment-inventory") {
        let inventory = gather_deployment_machine_inventory();
        print!("{}", render_deployment_machine_inventory(&inventory));
        return;
    }
    let text_report_requested = args.len() == 1 || args.get(1).map(String::as_str) == Some("text");
    if !text_report_requested {
        usage();
        std::process::exit(2);
    }

    let report = gather_machine_report();
    print!("{}", render_machine_report(&report));
}
