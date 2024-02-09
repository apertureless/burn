use reqwest;
use std::error::Error;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

pub(crate) static CLIENT_ID: &'static str = "Iv1.84002254a02791f3";
static GITHUB_API_VERSION_HEADER: &'static str = "X-GitHub-Api-Version";
static GITHUB_API_VERSION: &'static str = "2022-11-28";

/// Return the file path for the auth cache on disk
pub fn get_auth_cache_file_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("an home directory should exist");
    let path_dir = home_dir.join(".cache").join("burn").join("burnbench");
    #[cfg(test)]
    let path_dir = path_dir.join("test");
    let path = Path::new(&path_dir);
    path.join("token.txt")
}

/// Return true if the token is still valid
pub(crate) fn is_token_valid(token: &str) -> bool {
    get_username_from_token(token).is_ok()
}

/// Retrieve the user name from the access token
fn get_username_from_token(token: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    // User-Agent is important otherwise GitHub rejects the request with a 403
    // See: https://github.com/seanmonstar/reqwest/issues/918#issuecomment-632906966
    let response = client
        .get("https://api.github.com/user")
        .header(reqwest::header::USER_AGENT, "burnbench")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .header(GITHUB_API_VERSION_HEADER, GITHUB_API_VERSION)
        .send()?;
    let response = response.json::<serde_json::Value>()?;
    let username = response["login"].as_str().map(|s| s.to_string());
    // Return an error if the login field is not found
    username.ok_or_else(|| {
        From::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Username not found in the response",
        ))
    })
}

/// Save token in Burn cache directory and adjust file permissions
pub(crate) fn save_token(token: &str) {
    let path = get_auth_cache_file_path();
    fs::create_dir_all(path.parent().expect("path should have a parent directory"))
        .expect("directory should be created");
    let mut file = File::create(&path).expect("file should be created");
    write!(file, "{}", token).expect("token should be written to file");
    // On unix systems we lower the permissions on the cache file to be readable
    // just by the current user
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .expect("permissions should be set to 600");
    println!("✅ Token saved at location: {}", path.to_str().unwrap());
}

/// Return the token saved in the cache file
pub(crate) fn get_token_from_cache() -> Option<String> {
    let path = get_auth_cache_file_path();
    match fs::read_to_string(&path) {
        Ok(contents) => contents.lines().next().map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
use serial_test::serial;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn cleanup_test_environment() {
        let path = get_auth_cache_file_path();
        if path.exists() {
            fs::remove_file(&path).expect("should be able to delete the token file");
        }
        let parent_dir = path
            .parent()
            .expect("token file should have a parent directory");
        if parent_dir.exists() {
            fs::remove_dir_all(parent_dir).expect("should be able to delete the cache directory");
        }
    }

    #[test]
    #[serial]
    fn test_save_token_when_file_does_not_exist() {
        cleanup_test_environment();
        let token = "unique_test_token";
        // Ensure the file does not exist
        let path = get_auth_cache_file_path();
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        save_token(token);
        assert_eq!(fs::read_to_string(path).unwrap(), token);
        cleanup_test_environment();
    }

    #[test]
    #[serial]
    fn test_overwrite_saved_token_when_file_already_exists() {
        cleanup_test_environment();
        let initial_token = "initial_test_token";
        let new_token = "new_test_token";
        // Save initial token
        save_token(initial_token);
        // Save new token that should overwrite the initial one
        save_token(new_token);
        let path = get_auth_cache_file_path();
        assert_eq!(fs::read_to_string(path).unwrap(), new_token);
        cleanup_test_environment();
    }

    #[test]
    #[serial]
    fn test_get_saved_token_from_cache_when_it_exists() {
        cleanup_test_environment();
        let token = "existing_test_token";
        // Save the token first
        save_token(token);
        // Now retrieve it
        let retrieved_token = get_token_from_cache().unwrap();
        assert_eq!(retrieved_token, token);
        cleanup_test_environment();
    }

    #[test]
    #[serial]
    fn test_return_only_first_line_of_cache_as_token() {
        cleanup_test_environment();
        let path = get_auth_cache_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("directory tree should be created");
        }
        // Create a file with multiple lines
        let mut file = File::create(&path).expect("test file should be created");
        write!(file, "first_line_token\nsecond_line\nthird_line")
            .expect("test file should contain several lines");
        // Test that only the first line is returned as the token
        let token = get_token_from_cache().expect("token should be present");
        assert_eq!(
            token, "first_line_token",
            "The token should match only the first line of the file"
        );
        cleanup_test_environment();
    }

    #[test]
    #[serial]
    fn test_return_none_when_cache_file_does_not_exist() {
        cleanup_test_environment();
        let path = get_auth_cache_file_path();
        // Ensure the file does not exist
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        assert!(get_token_from_cache().is_none());
        cleanup_test_environment();
    }

    #[test]
    #[serial]
    fn test_return_none_when_cache_file_exists_but_is_empty() {
        cleanup_test_environment();
        // Create an empty file
        let path = get_auth_cache_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("directory tree should be created");
        }
        File::create(&path).expect("empty file should be created");
        assert!(
            get_token_from_cache().is_none(),
            "Expected None for empty cache file, got Some"
        );
        cleanup_test_environment();
    }
}
