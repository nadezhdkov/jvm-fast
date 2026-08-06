use thiserror::Error;

#[derive(Debug, Error)]
pub enum PomParseError {
    #[error("malformed POM XML: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("POM XML is not valid UTF-8: {0}")]
    Encoding(#[from] std::str::Utf8Error),
}
