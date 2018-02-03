use std::process::Command;
use std::path::Path;
use std::io;

pub struct Image {
    name: String,
}

impl Image {
    pub fn build(name: &str, docker_file: &Path) -> io::Result<Self> {
        debug!("docker file path: {}", docker_file.canonicalize()?.to_str().unwrap());
        
        let output = Command::new("docker")
            .current_dir(docker_file.parent().unwrap().canonicalize()?.to_str().unwrap())
            .arg("build")
            .arg("--network=host") // share the network with host
            // .arg(docker_file.canonicalize()?.to_str().unwrap())
            .arg(".")
            .arg("-t")
            .arg(name)
            .output()?;

        let success_message = format!("Successfully tagged {}:latest", name);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains(&success_message) {
            panic!("Failed to build docker image: {}",
                String::from_utf8_lossy(&output.stderr))
        }

        Ok(Image { 
            name: name.to_string(),
        })
    }
}

pub struct Container {
    id: String,
    image: Image,
}

impl Container {
    pub fn start(image: Image, home_path: &Path) -> io::Result<Self> {
        let output = Command::new("docker")
            .arg("run")
            .arg("-i") // keep container alive even though we are not attached
            .arg("-d") // run in the background
            .arg("-v") // link notebook folder
            .arg(format!("{}:/home", home_path.canonicalize()?.to_str().unwrap()))
            .arg("--net=host") // share the network with host
            .arg(&image.name)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr != "" {
            panic!("Failed to start container: {}", stderr);
        }

        Ok(Container {
            id: stdout.to_string(),
            image,
        })
    }

    fn kill(&self) -> io::Result<()> {
        let output = Command::new("docker")
            .arg("kill")
            .arg(&self.id)
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr != "" {
            warn!("failed to kill container: {}", stderr);
        }

        Ok(())
    }

    fn exec(&self, cmd: &str) -> io::Result<(String, String)> {
        let output = Command::new("docker")
            .arg("exec")
            .arg(&self.id)
            .arg(cmd)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok((stdout.to_string(), stderr.to_string()))
    }
}
