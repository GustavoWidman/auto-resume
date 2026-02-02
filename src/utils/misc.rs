pub fn strip_url(url: &str) -> &str {
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
}
