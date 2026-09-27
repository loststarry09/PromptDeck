fn main() {
    slint_build::compile("ui/app.slint").expect("slint compile failed");

    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=ui/assets/app.ico");

    embed_resource::compile("app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("windows resource compile failed");
}
