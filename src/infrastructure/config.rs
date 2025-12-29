use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxConfig {
    pub influx: InfluxSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxSettings {
    pub host: String,
    pub token: String,
    pub database: String,
    pub retention_policy: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WidgetsConfig {
    #[serde(default)]
    pub tiles: Vec<TileConfig>,
    #[serde(default)]
    pub charts: Vec<ChartConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TileConfig {
    pub id: String,
    pub title: String,
    pub unit: String,
    pub precision: i32,
    pub query: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChartConfig {
    pub id: String,
    pub title: String,
    pub unit: Option<String>,
    pub kind: String,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub fraction_digits: Option<i32>,
    #[serde(default)]
    pub series: Vec<SeriesConfig>,
    #[serde(default)]
    pub overlays: Vec<OverlayConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SeriesConfig {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub query: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OverlayConfig {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub query: String,
}

pub fn load_influx_config() -> anyhow::Result<InfluxConfig> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config/influx"))
        .build()?;
    
    Ok(settings.try_deserialize()?)
}

pub fn load_widgets_config() -> anyhow::Result<WidgetsConfig> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config/widgets"))
        .build()?;
    
    Ok(settings.try_deserialize()?)
}

/// Replace template variables in a query string
pub fn prepare_query(query: &str, vars: &HashMap<String, String>) -> String {
    let mut result = query.to_string();
    for (key, value) in vars {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_query() {
        let mut vars = HashMap::new();
        vars.insert("source".to_string(), "reef".to_string());
        vars.insert("hours".to_string(), "12".to_string());
        
        let query = "SELECT * FROM apex_probe WHERE host='${source}' AND time >= now() - ${hours}h";
        let result = prepare_query(query, &vars);
        
        assert_eq!(result, "SELECT * FROM apex_probe WHERE host='reef' AND time >= now() - 12h");
    }
}

