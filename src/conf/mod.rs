use std::env;
use std::path::Path;

pub fn load_environment_variables() {
    let path = ".env";
    if Path::new(path).exists() {
        match dotenv::dotenv() {
            Ok(_) => {}
            Err(e) => println!("Failed to load {} file: {}", path, e),
        }
    }
}

pub fn get_env(key: &str, error_message: &str) -> String {
    match env::var(key) {
        Ok(val) => val,
        Err(_) => panic!("{}", error_message),
    }
}
