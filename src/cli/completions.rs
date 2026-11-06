use crate::cli::args::Cli;
use crate::error::XFetchError;
use clap::CommandFactory;
use clap_complete::Shell;
use std::io::Write;

pub fn completions(shell: Shell) -> Result<(), XFetchError> {
    let mut command = Cli::command();
    let mut buffer = Vec::new();
    clap_complete::generate(shell, &mut command, "xfetch", &mut buffer);
    let mut stdout = std::io::stdout();
    match stdout.write_all(&buffer).and_then(|()| stdout.flush()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(XFetchError::Fatal(format!(
            "failed to write completions: {err}"
        ))),
    }
}

pub fn man(output: Option<String>) -> Result<(), XFetchError> {
    let mut buffer = Vec::new();
    clap_mangen::Man::new(Cli::command())
        .render(&mut buffer)
        .map_err(|err| XFetchError::Fatal(format!("failed to render man page: {err}")))?;

    match output {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    XFetchError::Fatal(format!("cannot create '{}': {}", parent.display(), err))
                })?;
            }
            crate::atomic_fs::write_atomic(&path, &buffer).map_err(|err| {
                XFetchError::Fatal(format!("cannot write '{}': {}", path.display(), err))
            })?;
            crate::runtime::info(format!("Wrote man page to {}", path.display()));
        }
        None => {
            let mut stdout = std::io::stdout();
            match stdout.write_all(&buffer).and_then(|()| stdout.flush()) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {}
                Err(err) => {
                    return Err(XFetchError::Fatal(format!(
                        "failed to write man page: {err}"
                    )));
                }
            }
        }
    }
    Ok(())
}
