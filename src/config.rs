use crate::handlers::AppConfig;

/// Load config from path
pub fn load_config(path: &str) -> anyhow::Result<AppConfig> {
    let raw = std::fs::read_to_string(path)?;
    let cfg: AppConfig = toml::from_str(&raw)?;

    // field names must be unique and non-empty
    let mut seen = std::collections::HashSet::new();
    for f in &cfg.fields {
        if f.name.trim().is_empty() {
            anyhow::bail!("field with empty name in config");
        }
        if !seen.insert(f.name.clone()) {
            anyhow::bail!("duplicate field name in config: {}", f.name);
        }
    }

    Ok(cfg)
}
