pub mod request;
pub mod model;

#[macro_export]
macro_rules! parse_field {
    ($map:expr, $field:literal) => {
        $map.get($field)
            .ok_or_else(|| anyhow!("{} not found", $field))?
            .parse()
            .map_err(|_| anyhow!("{} parse error", $field))?
    };
    ($map:expr, $field:literal, $type:ty) => {
        $map.get($field)
            .ok_or_else(|| anyhow!("{} not found", $field))?
            .parse::<$type>()
            .map_err(|_| anyhow!("{} parse error", $field))?
    };
    ($map:expr, $field:literal, $type:ty, $default:expr) => {
        $map.get($field)
            .map_or_else(|| $default, |v| v.parse::<$type>().unwrap_or($default))
    };
}
