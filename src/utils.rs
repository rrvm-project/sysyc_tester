use std::io::{self, Write};

pub fn run_command(
    command: &str,
    args: &[&str],
    current_dir: &str,
) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new(command);
    cmd.args(args);
    cmd.current_dir(current_dir);
    let o = cmd.output()?;
    io::stdout().write_all(&o.stdout)?;
    io::stderr().write_all(&o.stderr)?;
    Ok(o)
}
