use edgerun_virtual_disk::BlockError;
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

fn main() {
    if let Err(error) = run() {
        eprintln!("nbd-quick-attach error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), BlockError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 8 {
        print_usage(&args[0]);
        return Ok(());
    }

    let host = args[1].clone();
    let port = args[2].clone();
    let export_name = args[3].clone();
    let device = args[4].clone();
    let mode = args[5].clone();
    let mut server_args = vec![host_port(&host, &port), export_name.clone(), mode.clone()];
    server_args.extend_from_slice(&args[6..]);

    let server_bin = sibling_binary("nbd-server")?;
    let attach_bin = sibling_binary("nbd-attach")?;
    let mut child = Command::new(server_bin)
        .args(&server_args)
        .spawn()
        .map_err(|err| BlockError::BackendFailure(format!("failed to start nbd-server: {err}")))?;

    thread::sleep(Duration::from_millis(750));

    let mut attach_child = Command::new(attach_bin)
        .arg("attach")
        .arg(&host)
        .arg(&port)
        .arg(&export_name)
        .arg(&device)
        .spawn()
        .map_err(|err| BlockError::BackendFailure(format!("failed to run nbd-attach: {err}")))?;

    thread::sleep(Duration::from_millis(750));

    match attach_child.try_wait() {
        Ok(None) => {
            println!(
                "started nbd-server pid {} and nbd-attach pid {} for export `{}` on {}",
                child.id(),
                attach_child.id(),
                export_name,
                device
            );
            println!("stop server manually when done: kill {}", child.id());
            println!(
                "stop attach helper manually when done: kill {}",
                attach_child.id()
            );
            Ok(())
        }
        Ok(Some(status)) if status.success() => {
            println!(
                "started nbd-server pid {} and attached export `{}` to {}",
                child.id(),
                export_name,
                device
            );
            Ok(())
        }
        Ok(Some(_)) => {
            terminate_child(&mut child);
            Err(BlockError::BackendFailure(
                "attach helper exited early with failure; server was terminated".into(),
            ))
        }
        Err(err) => {
            terminate_child(&mut child);
            terminate_child(&mut attach_child);
            Err(BlockError::BackendFailure(format!(
                "failed to monitor attach helper: {err}"
            )))
        }
    }
}

fn host_port(host: &str, port: &str) -> String {
    format!("{host}:{port}")
}

fn sibling_binary(name: &str) -> Result<PathBuf, BlockError> {
    let exe = env::current_exe().map_err(|err| {
        BlockError::BackendFailure(format!("failed to locate current exe: {err}"))
    })?;
    let dir = exe.parent().ok_or_else(|| {
        BlockError::BackendFailure("failed to locate executable directory".into())
    })?;
    Ok(dir.join(name))
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} <host> <port> <export-name> <device> mem <block-size> <block-count>");
    eprintln!("  {program} <host> <port> <export-name> <device> file <disk-path> <block-size>");
}
