fn main() {
    println!("cargo:rerun-if-changed=resources/spectacle.rc");
    println!("cargo:rerun-if-changed=resources/spectacle.manifest");
    println!("cargo:rerun-if-changed=resources/spectacle.ico");
    // Only the executable carries the icon, manifest and version information;
    // the unit test binaries run without them.
    embed_resource::compile_for(
        "resources/spectacle.rc",
        ["Spectacle"],
        embed_resource::NONE,
    )
    .manifest_required()
    .expect("failed to compile the Windows resources");
}
