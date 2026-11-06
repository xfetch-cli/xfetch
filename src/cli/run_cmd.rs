use crate::cli::{Context, output};
use crate::error::XFetchError;

pub fn run(
    name: &str,
    args: &[String],
    render: bool,
    json: bool,
    ctx: &Context,
) -> Result<(), XFetchError> {
    let result = crate::commands::run_command(name, args).map_err(XFetchError::Addon)?;

    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "command": name,
            "args": args,
            "lines": result.lines,
        }));
    }

    if render {
        let config = ctx.load_config();
        return crate::ui::print_custom_content(&config, result.lines).map_err(XFetchError::Fatal);
    }

    print!("{}", result.raw);
    if !result.raw.ends_with('\n') {
        println!();
    }
    Ok(())
}
