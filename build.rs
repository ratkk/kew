use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("compiling shaders");

    let shader_dir_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("shader");
    println!("shader source directory: {}", shader_dir_path.to_str().unwrap());

    fs::read_dir(shader_dir_path.clone())
        .unwrap()
        .map(Result::unwrap)
        .filter(|file| file.file_type().unwrap().is_file())
        .for_each(|file| {
            let output = Command::new("glslc")
                .current_dir(&shader_dir_path)
                .arg(file.path())
                .arg("-o")
                .arg(format!(
                    "{}\\compiled\\{}.spv",
                    file.path().parent().unwrap().to_str().unwrap(),
                    file.file_name().to_str().unwrap()
                ))
                .output()
                .expect("failed to compile shader");

            if !output.stdout.is_empty() {
                println!("glslc stdout: {}", String::from_utf8_lossy(&output.stdout));
            }

            if !output.stderr.is_empty() {
                eprintln!("glslc stderr: {}", String::from_utf8_lossy(&output.stderr));
            }

            if !output.status.success() {
                panic!("glslc failed with: {}", output.status);
            }
        })
}
