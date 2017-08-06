extern crate gcc;

fn main() {
    if cfg!(feature = "test") {
        gcc::Config::new()
            .file("src/bcndecode.c")
            .compile("libbcndecode.a");
    }
}
