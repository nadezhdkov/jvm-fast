mod support;

use jvmfast::pom::{HttpPomProvider, PomProvider};
use support::start_mock_server;

const DEMO_POM: &str = r#"<project>
  <dependencies>
    <dependency>
      <groupId>com.example</groupId>
      <artifactId>inner</artifactId>
      <version>1.2.3</version>
    </dependency>
  </dependencies>
</project>"#;

#[test]
fn http_pom_provider_fetches_and_parses_using_maven_layout() {
    let server = start_mock_server(|path| {
        if path == "/com/example/demo/1.0.0/demo-1.0.0.pom" {
            (200, DEMO_POM.as_bytes().to_vec())
        } else {
            (404, Vec::new())
        }
    });

    let provider = HttpPomProvider::new(server.base_url);
    let parsed = provider
        .fetch("com.example:demo", "1.0.0")
        .expect("should fetch and parse");

    assert_eq!(parsed.dependencies.len(), 1);
    assert_eq!(parsed.dependencies[0].coordinate, "com.example:inner");
    assert_eq!(parsed.dependencies[0].version, "1.2.3");
}

#[test]
fn http_pom_provider_wraps_http_error_status_as_typed_error() {
    let server = start_mock_server(|_path| (404, Vec::new()));

    let provider = HttpPomProvider::new(server.base_url);
    let result = provider.fetch("com.example:missing", "1.0.0");

    assert!(result.is_err());
}

#[test]
fn http_pom_provider_wraps_malformed_xml_as_typed_parse_error() {
    let malformed =
        b"<project><dependencies><dependency></notdependency></dependencies></project>".to_vec();
    let server = start_mock_server(move |_path| (200, malformed.clone()));

    let provider = HttpPomProvider::new(server.base_url);
    let result = provider.fetch("com.example:broken", "1.0.0");

    assert!(result.is_err());
}

#[test]
fn http_pom_provider_rejects_coordinate_without_colon() {
    let provider = HttpPomProvider::new("http://127.0.0.1:1");

    let result = provider.fetch("invalid-coordinate", "1.0.0");

    assert!(result.is_err());
}
