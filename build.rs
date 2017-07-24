extern crate gcc;

fn main() {
    gcc::Config::new().file("src/bcndecode.c").compile("libbcndecode.a");
}