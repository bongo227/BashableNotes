use std::process::Command;
use std::path::Path;
use std::io;

#[derive(Clone)]
pub struct Image {
    name: String,
}

impl Image {
    pub fn build(name: &str, docker_file: &Path) -> io::Result<Self> {
        info!(
            "building docker file: {}",
            docker_file.canonicalize()?.to_str().unwrap()
        );

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
            panic!(
                "Failed to build docker image: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }

        Ok(Image {
            name: name.to_string(),
        })
    }
}

#[derive(Clone)]
pub struct Container {
    id: String,
    image: Image,
}

impl Container {
    pub fn start(image: Image, home_path: &Path) -> io::Result<Self> {
        let mut command = Command::new("docker");
        let command = command
            .arg("run")
            .arg("-i") // keep container alive even though we are not attached
            .arg("-d") // run in the background
            .arg("-v") // link notebook folder
            .arg(format!("{}:/home", home_path.canonicalize()?.to_str().unwrap()))
            .arg("--net=host") // share the network with host
            .arg(&image.name);

        debug!("docker run command: {:?}", command);
        let output = command.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stderr != "" {
            panic!("Failed to start container: {}", stderr);
        }

        Ok(Container {
            id: stdout.trim().to_string(),
            image,
        })
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn kill(self) -> io::Result<()> {
        info!("killing container: {}", self.id);

        let output = Command::new("docker").arg("kill").arg(self.id).output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr != "" {
            warn!("failed to kill container: {}", stderr);
        } else {
            info!("container killed");
        }

        Ok(())
    }

    pub fn exec(&self, cmd: &str, code: &str) -> io::Result<(String, String)> {
        let mut command = Command::new("docker");
        let command = command
            .arg("exec")
            .arg("--env")
            .arg(&format!("CODE={}", code))
            .arg(&self.id)
            .arg("bash")
            .arg("-c")
            .arg(&format!("cd home && {}", cmd));
        // .output()?;

        debug!("docker exec command: {:?}", command);
        let output = command.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("docker exec output: {} {}", stdout, stderr);

        Ok((stdout.to_string(), stderr.to_string()))
    }
}
