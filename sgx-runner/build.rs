extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/c/mapping.c")
        .compile("mapping");
}
