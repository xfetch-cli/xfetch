//! User-configurable value formatting: `"formats"` (per-module templates
//! with `{field}` placeholders) and `"labels"` (per-module row keys) in
//! config. Formatting happens once, when the render tree is built
//! (`ui::nodes::prepare_render_tree`), so every layout sees the same values.

use std::collections::HashMap;

/// Substitutes `{field}` placeholders in a template with the given fields.
/// Unknown fields render empty; `{{` and `}}` escape literal braces.
pub fn substitute(template: &str, fields: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => match chars.peek() {
                Some('{') => {
                    chars.next();
                    out.push('{');
                }
                Some(_) => {
                    let mut name = String::new();
                    for ch in chars.by_ref() {
                        if ch == '}' {
                            if let Some(v) = fields.get(name.trim()) {
                                out.push_str(v);
                            }
                            name.clear();
                            break;
                        }
                        if ch == '{' {
                            out.push('{');
                            out.push_str(&name);
                            name.clear();
                            break;
                        }
                        name.push(ch);
                    }
                    if !name.is_empty() {
                        out.push('{');
                        out.push_str(&name);
                    }
                }
                None => out.push('{'),
            },
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    out.push('}');
                } else {
                    out.push('}');
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Fields every module exposes: the module key and its current value.
fn base_fields(key: &str, value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    fields.insert("key".to_string(), key.to_string());
    fields.insert("value".to_string(), value.to_string());
    fields
}

/// Formats a module value with the configured template (`formats` in
/// config). Without a template the value passes through unchanged.
pub fn format_value(key: &str, value: &str, formats: &HashMap<String, String>) -> String {
    let template = formats.get(key).map(String::as_str).unwrap_or("{value}");
    let mut fields = base_fields(key, value);
    fields.extend(module_fields(key, value));
    substitute(template, &fields)
}

/// Formats every GPU line with the platform's own field extractor and joins
/// them exactly like the default GPU value (`" / "`). The template applies
/// per line, so multi-GPU setups format each device independently.
pub fn format_gpu_list(gpus: &[String], formats: &HashMap<String, String>) -> String {
    let template = formats.get("gpu").map(String::as_str).unwrap_or("{value}");
    gpus.iter()
        .map(|line| {
            let mut fields = base_fields("gpu", line);
            fields.extend(crate::info::platform::gpu_fields(line));
            substitute(template, &fields)
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

fn module_fields(key: &str, value: &str) -> HashMap<String, String> {
    match key {
        "cpu" => cpu_fields(value),
        "memory" | "swap" => mem_fields(value),
        "disk" => disk_fields(value),
        "os" => os_fields(value),
        "uptime" => uptime_fields(value),
        "battery" => battery_fields(value),
        "datetime" => datetime_fields(value),
        _ => HashMap::new(),
    }
}

/// Memory/swap fields derived from the default value
/// (`"2.27 GiB / 7.74 GiB (29%)"`): `{used}` and `{total}` (with unit) and
/// `{percent}` as a bare number.
fn mem_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    if let Some((used, total, percent)) = parse_mem_pair(value) {
        fields.insert("used".to_string(), used);
        fields.insert("total".to_string(), total);
        fields.insert("percent".to_string(), percent);
    }
    fields
}

/// Disk fields derived from the default value
/// (`"0.00 GiB / 3.87 GiB (0%) - overlay"`): the memory fields plus
/// `{fs}` (filesystem name, when shown).
fn disk_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    if let Some((mem, fs)) = value.rsplit_once(" - ") {
        if let Some((used, total, percent)) = parse_mem_pair(mem) {
            fields.insert("used".to_string(), used);
            fields.insert("total".to_string(), total);
            fields.insert("percent".to_string(), percent);
        }
        fields.insert("fs".to_string(), fs.trim().to_string());
    } else if let Some((used, total, percent)) = parse_mem_pair(value) {
        fields.insert("used".to_string(), used);
        fields.insert("total".to_string(), total);
        fields.insert("percent".to_string(), percent);
    }
    fields
}

/// Splits a memory-style value (`"2.27 GiB / 7.74 GiB (29%)"`) into
/// `(used, total, percent)`.
fn parse_mem_pair(value: &str) -> Option<(String, String, String)> {
    let (used, rest) = value.split_once(" / ")?;
    let mut tokens = rest.split_whitespace();
    let mut total_parts = Vec::new();
    let percent = loop {
        let tok = tokens.next()?;
        if tok.starts_with('(') {
            break tok
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim_end_matches('%')
                .to_string();
        }
        total_parts.push(tok);
    };
    Some((used.trim().to_string(), total_parts.join(" "), percent))
}

const KNOWN_ARCHS: &[&str] = &[
    "x86_64", "i686", "x86", "aarch64", "arm64", "armv7l", "armv6l", "riscv64", "ppc64le", "s390x",
];

/// OS fields derived from the decorated OS value
/// (`"Ubuntu 24.04 x86_64 (WSL)"`): `{distro}`, `{version}`, `{arch}` and
/// `{wsl}` (only when the WSL decoration is present).
fn os_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut base = value;
    if let Some(wsl_start) = value.find(" (WSL")
        && value[wsl_start..].ends_with(')')
    {
        let detail = value[wsl_start + 5..value.len() - 1].trim();
        let wsl = if detail.is_empty() {
            "WSL".to_string()
        } else {
            format!("WSL {}", detail)
        };
        fields.insert("wsl".to_string(), wsl);
        base = &value[..wsl_start];
    }
    let mut rest = base;
    if let Some((head, tail)) = base.rsplit_once(' ')
        && KNOWN_ARCHS.contains(&tail)
    {
        fields.insert("arch".to_string(), tail.to_string());
        rest = head;
    }
    if let Some((head, tail)) = rest.rsplit_once(' ')
        && tail.chars().any(|c| c.is_ascii_digit())
    {
        fields.insert("version".to_string(), tail.to_string());
        fields.insert("distro".to_string(), head.to_string());
        return fields;
    }
    fields.insert("distro".to_string(), rest.to_string());
    fields
}

/// Uptime fields derived from the default value (`"26 hours, 3 mins"`):
/// `{days}`, `{hours}` and `{mins}` (days are derived from the hour count).
fn uptime_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut hours = None;
    let mut mins = None;
    for part in value.split(", ") {
        let mut tokens = part.split_whitespace();
        let n = tokens.next().and_then(|t| t.parse::<u64>().ok());
        match tokens.next() {
            Some(unit) if unit.starts_with("hour") => hours = n,
            Some(unit) if unit.starts_with("min") => mins = n,
            _ => {}
        }
    }
    if let (Some(h), Some(m)) = (hours, mins) {
        fields.insert("days".to_string(), (h / 24).to_string());
        fields.insert("hours".to_string(), (h % 24).to_string());
        fields.insert("mins".to_string(), m.to_string());
    }
    fields
}

/// Battery fields derived from the default value (`"85% [Charging]"`):
/// `{percent}` (bare number) and `{state}`.
fn battery_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    if let Some(open) = value.find('[')
        && let Some(close) = value[open..].find(']')
    {
        let state = value[open + 1..open + close].trim();
        let percent = value[..open].trim().trim_end_matches('%');
        if !state.is_empty() {
            fields.insert("state".to_string(), state.to_string());
        }
        if !percent.is_empty() && percent.chars().all(|c| c.is_ascii_digit()) {
            fields.insert("percent".to_string(), percent.to_string());
        }
    }
    fields
}

/// Datetime fields derived from the default value (`"2026-08-20 12:34:56"`):
/// `{date}` and `{time}`.
fn datetime_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    if let Some((date, time)) = value.split_once(' ')
        && date.chars().filter(|c| *c == '-').count() == 2
    {
        fields.insert("date".to_string(), date.to_string());
        fields.insert("time".to_string(), time.to_string());
    }
    fields
}

/// Formats the packages value. Beyond the universal fields, every manager in
/// the breakdown becomes a field named after it (`{pacman}`, `{aur}`, ...),
/// plus `{count}`/`{manager}` for the first entry and `{managers}` for the
/// joined manager names.
pub fn format_packages(
    value: &str,
    breakdown: &[(String, String)],
    formats: &HashMap<String, String>,
) -> String {
    let template = formats
        .get("packages")
        .map(String::as_str)
        .unwrap_or("{value}");
    let mut fields = base_fields("packages", value);
    for (cmd, count) in breakdown {
        fields.insert(cmd.clone(), count.clone());
    }
    if let Some((cmd, count)) = breakdown.first() {
        fields.insert("count".to_string(), count.clone());
        fields.insert("manager".to_string(), cmd.clone());
    }
    if !breakdown.is_empty() {
        fields.insert(
            "managers".to_string(),
            breakdown
                .iter()
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    substitute(template, &fields)
}

/// CPU fields derived from the default CPU value, e.g.
/// `"Intel(R) Core(TM) i5-7400 CPU @ 3.00GHz (4) @ 3.00 GHz"`:
/// `{brand}` raw, `{model}` cleaned (`"Intel Core i5-7400"`), `{cores}` and
/// `{freq}` (`"3.00 GHz"`).
fn cpu_fields(value: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    fields.insert("brand".to_string(), value.to_string());

    let mut brand = value;
    if let Some(start) = value.find(" (") {
        let rest = &value[start + 2..];
        if let Some(end) = rest.find(')')
            && let Some(cores) = rest[..end].split('@').next().map(str::trim)
            && !cores.is_empty()
            && cores.chars().all(|c| c.is_ascii_digit())
        {
            fields.insert("cores".to_string(), cores.to_string());
            if let Some(freq) = rest[end + 1..]
                .trim_start()
                .strip_prefix('@')
                .map(str::trim)
                .filter(|f| !f.is_empty())
            {
                fields.insert("freq".to_string(), freq.to_string());
            }
            brand = &value[..start];
        }
    }

    let mut model = brand
        .replace("(R)", "")
        .replace("(r)", "")
        .replace("(TM)", "")
        .replace("(tm)", "")
        .replace("(C)", "")
        .replace("(c)", "");
    if let Some(cpu_at) = model.find(" CPU @") {
        model.truncate(cpu_at);
    }
    for suffix in [
        " 64-Core Processor",
        " 32-Core Processor",
        " 24-Core Processor",
        " 16-Core Processor",
        " 12-Core Processor",
        " 8-Core Processor",
        " 6-Core Processor",
        " 4-Core Processor",
        " 2-Core Processor",
        " Processor",
    ] {
        if let Some(stripped) = model.strip_suffix(suffix) {
            model = stripped.to_string();
            break;
        }
    }
    let model = model.replace("  ", " ").trim().to_string();
    fields.insert("model".to_string(), model);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_substitute_known_and_unknown_fields() {
        let f = fields(&[("value", "v1"), ("cores", "4")]);
        assert_eq!(substitute("{value}", &f), "v1");
        assert_eq!(substitute("{cores} núcleos", &f), "4 núcleos");
        assert_eq!(substitute("x {nope} y", &f), "x  y");
        assert_eq!(substitute("{cores}{cores}", &f), "44");
    }

    #[test]
    fn test_substitute_literal_braces() {
        let f = fields(&[]);
        assert_eq!(substitute("{{value}}", &f), "{value}");
        assert_eq!(substitute("a }} b", &f), "a } b");
        assert_eq!(substitute("{", &f), "{");
        assert_eq!(substitute("}", &f), "}");
        assert_eq!(substitute("unclosed {field", &f), "unclosed {field");
    }

    #[test]
    fn test_substitute_trims_field_names() {
        let f = fields(&[("value", "v")]);
        assert_eq!(substitute("{ value }", &f), "v");
    }

    #[test]
    fn test_format_value_default_is_passthrough() {
        let formats = HashMap::new();
        assert_eq!(format_value("os", "Arch Linux", &formats), "Arch Linux");
    }

    #[test]
    fn test_format_value_custom_template() {
        let mut formats = HashMap::new();
        formats.insert("os".to_string(), "OS: {value}".to_string());
        assert_eq!(format_value("os", "Arch Linux", &formats), "OS: Arch Linux");
    }

    #[test]
    fn test_format_value_unknown_fields_render_empty() {
        let mut formats = HashMap::new();
        formats.insert("os".to_string(), "[{unknown}] {value}".to_string());
        assert_eq!(format_value("os", "Arch", &formats), "[] Arch");
    }

    #[test]
    fn test_cpu_fields_intel() {
        let v = "Intel(R) Core(TM) i5-7400 CPU @ 3.00GHz (4) @ 3.00 GHz";
        let f = cpu_fields(v);
        assert_eq!(f.get("brand").unwrap(), v);
        assert_eq!(f.get("model").unwrap(), "Intel Core i5-7400");
        assert_eq!(f.get("cores").unwrap(), "4");
        assert_eq!(f.get("freq").unwrap(), "3.00 GHz");
    }

    #[test]
    fn test_cpu_fields_amd() {
        let v = "AMD Ryzen 7 5800X 8-Core Processor (8) @ 4.70 GHz";
        let f = cpu_fields(v);
        assert_eq!(f.get("model").unwrap(), "AMD Ryzen 7 5800X");
        assert_eq!(f.get("cores").unwrap(), "8");
        assert_eq!(f.get("freq").unwrap(), "4.70 GHz");
    }

    #[test]
    fn test_cpu_fields_apple() {
        let v = "Apple M2 Pro (12) @ 2.40 GHz";
        let f = cpu_fields(v);
        assert_eq!(f.get("model").unwrap(), "Apple M2 Pro");
        assert_eq!(f.get("cores").unwrap(), "12");
    }

    #[test]
    fn test_cpu_fields_no_suffix() {
        let v = "Some Vendor (3) @ 1.00 GHz";
        let f = cpu_fields(v);
        assert_eq!(f.get("model").unwrap(), "Some Vendor");
        assert_eq!(f.get("cores").unwrap(), "3");
    }

    #[test]
    fn test_cpu_fields_unparseable_passthrough() {
        let v = "Unknown";
        let f = cpu_fields(v);
        assert_eq!(f.get("brand").unwrap(), "Unknown");
        assert_eq!(f.get("model").unwrap(), "Unknown");
        assert!(!f.contains_key("cores"));
    }

    #[test]
    fn test_format_value_cpu_template() {
        let mut formats = HashMap::new();
        formats.insert("cpu".to_string(), "{model} · {cores} núcleos".to_string());
        let v = "Intel(R) Core(TM) i5-7400 CPU @ 3.00GHz (4) @ 3.00 GHz";
        assert_eq!(
            format_value("cpu", v, &formats),
            "Intel Core i5-7400 · 4 núcleos"
        );
    }

    #[test]
    fn test_format_gpu_list_default_joins_with_slash() {
        let formats = HashMap::new();
        let gpus = vec!["A".to_string(), "B".to_string()];
        assert_eq!(format_gpu_list(&gpus, &formats), "A / B");
    }

    #[test]
    fn test_mem_fields() {
        let f = mem_fields("2.27 GiB / 7.74 GiB (29%)");
        assert_eq!(f.get("used").unwrap(), "2.27 GiB");
        assert_eq!(f.get("total").unwrap(), "7.74 GiB");
        assert_eq!(f.get("percent").unwrap(), "29");
        assert_eq!(mem_fields("0 B / 0 B (0%)").get("percent").unwrap(), "0");
        assert!(mem_fields("Unknown").is_empty());
    }

    #[test]
    fn test_disk_fields() {
        let f = disk_fields("0.00 GiB / 3.87 GiB (0%) - overlay");
        assert_eq!(f.get("used").unwrap(), "0.00 GiB");
        assert_eq!(f.get("total").unwrap(), "3.87 GiB");
        assert_eq!(f.get("percent").unwrap(), "0");
        assert_eq!(f.get("fs").unwrap(), "overlay");

        let f = disk_fields("1.00 GiB / 2.00 GiB (50%)");
        assert_eq!(f.get("percent").unwrap(), "50");
        assert!(!f.contains_key("fs"));
    }

    #[test]
    fn test_os_fields_plain() {
        let f = os_fields("Ubuntu 24.04 x86_64");
        assert_eq!(f.get("distro").unwrap(), "Ubuntu");
        assert_eq!(f.get("version").unwrap(), "24.04");
        assert_eq!(f.get("arch").unwrap(), "x86_64");
        assert!(!f.contains_key("wsl"));
    }

    #[test]
    fn test_os_fields_wsl() {
        let f = os_fields("Ubuntu 24.04 x86_64 (WSL)");
        assert_eq!(f.get("wsl").unwrap(), "WSL");
        let f = os_fields("Ubuntu 24.04 x86_64 (WSL 2, WSLg)");
        assert_eq!(f.get("wsl").unwrap(), "WSL 2, WSLg");
    }

    #[test]
    fn test_os_fields_no_version() {
        let f = os_fields("Arch Linux x86_64");
        assert_eq!(f.get("distro").unwrap(), "Arch Linux");
        assert_eq!(f.get("arch").unwrap(), "x86_64");
        assert!(!f.contains_key("version"));

        let f = os_fields("macOS 15.1 arm64");
        assert_eq!(f.get("distro").unwrap(), "macOS");
        assert_eq!(f.get("version").unwrap(), "15.1");
        assert_eq!(f.get("arch").unwrap(), "arm64");
    }

    #[test]
    fn test_uptime_fields() {
        let f = uptime_fields("3 hours, 12 mins");
        assert_eq!(f.get("days").unwrap(), "0");
        assert_eq!(f.get("hours").unwrap(), "3");
        assert_eq!(f.get("mins").unwrap(), "12");
        let f = uptime_fields("26 hours, 3 mins");
        assert_eq!(f.get("days").unwrap(), "1");
        assert_eq!(f.get("hours").unwrap(), "2");
        let f = uptime_fields("1 hour, 1 min");
        assert_eq!(f.get("hours").unwrap(), "1");
        assert_eq!(f.get("mins").unwrap(), "1");
        assert!(uptime_fields("Unknown").is_empty());
    }

    #[test]
    fn test_battery_fields() {
        let f = battery_fields("85% [Charging]");
        assert_eq!(f.get("percent").unwrap(), "85");
        assert_eq!(f.get("state").unwrap(), "Charging");
        let f = battery_fields("40% [Discharging]");
        assert_eq!(f.get("state").unwrap(), "Discharging");
        assert!(battery_fields("N/A").is_empty());
    }

    #[test]
    fn test_datetime_fields() {
        let f = datetime_fields("2026-08-20 12:34:56");
        assert_eq!(f.get("date").unwrap(), "2026-08-20");
        assert_eq!(f.get("time").unwrap(), "12:34:56");
        assert!(datetime_fields("Unknown").is_empty());
    }

    #[test]
    fn test_format_packages_fields() {
        let breakdown = vec![
            ("pacman".to_string(), "1234".to_string()),
            ("aur".to_string(), "25".to_string()),
        ];
        let mut formats = HashMap::new();
        formats.insert(
            "packages".to_string(),
            "pacman: {pacman} · AUR: {aur}".to_string(),
        );
        assert_eq!(
            format_packages("1234 + 25", &breakdown, &formats),
            "pacman: 1234 · AUR: 25"
        );
        let formats = HashMap::new();
        assert_eq!(
            format_packages("1234 + 25", &breakdown, &formats),
            "1234 + 25"
        );
        assert_eq!(
            format_packages("1234 + 25", &breakdown, &formats),
            format_packages("1234 + 25", &breakdown, &formats)
        );
    }

    #[test]
    fn test_format_value_mem_template() {
        let mut formats = HashMap::new();
        formats.insert(
            "memory".to_string(),
            "{used} / {total} ({percent}%)".to_string(),
        );
        assert_eq!(
            format_value("memory", "2.27 GiB / 7.74 GiB (29%)", &formats),
            "2.27 GiB / 7.74 GiB (29%)"
        );
    }
}
