fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/blobstore.c")
        .compile("cxx_poc");
}
