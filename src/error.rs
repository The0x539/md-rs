use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("Failed to deserialize JSON as {}: {}", type_name, source))]
    JsonErr {
        type_name: String,
        source: serde_json::Error,
    },
    #[snafu(display("Failed to perform HTTP request: {}", source))]
    HttpErr { source: reqwest::Error },
    #[snafu(display("druid PlatformError during `{}`: {}", action, source))]
    DruidErr {
        action: &'static str,
        source: druid::PlatformError,
    },
    #[snafu(display("Failed to decode image: {}", source))]
    ImageErr { source: image::ImageError },
}

pub type Result<T> = std::result::Result<T, Error>;
