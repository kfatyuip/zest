pub mod config;
pub mod init;
pub mod route;
pub mod zest;

#[cfg(test)]
mod tests {
    use super::route::mime_match;

    #[test]
    fn mime_test() {
        assert_eq!(mime_match("test.txt"), mime::TEXT_PLAIN);
        assert_eq!(mime_match("image.jpg"), mime::IMAGE_JPEG);
        assert_eq!(mime_match("data.bin"), mime::APPLICATION_OCTET_STREAM);
        assert_eq!(mime_match("index.html"), mime::TEXT_HTML);
    }
}
