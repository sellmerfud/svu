
use anyhow::Result;
use std::env;
use std::io::Write;
use crate::util::SvError::*;
use std::path::{Path, PathBuf};
use crate::svn;

#[derive(Debug, Clone)]
pub struct Credentials(pub String, pub String);   // username and password

//  In order to support subversions repositories that require authentication
//  we do the following in order:
//  First, if the SVU_USERNAME and SVU_PASSWORD environment variables are set
//  then we use those values for authentication.
//  
//  If that is not the case then we attempt to access the repository without
//  to see if authentication is necessary.  If this access if it succeeds
//  then authentication not is needed.  Either the repo does not require it or
//  the user has cached subversion credentials.
//
//  Finally, if authentication is needed, we prompt the user for their credentials.


pub fn get_credentials() -> Result<Option<Credentials>> 
{
    let wc_info = svn::workingcopy_info()?;  // Ensure we are in working copy directory
    let wc_root = PathBuf::from(wc_info.wc_path.unwrap());

    let username = env::var("SVU_USERNAME").ok();
    let password = env::var("SVU_PASSWORD").ok();

    match (username, password) {
        (Some(u), Some(p)) => {
            if access_repo(Some(Credentials(u.clone(), p.clone())), &wc_root)? {
                Ok(Some(Credentials(u, p)))
            }
            else {
                return Err(General("Not a valid SVU_USERNAME/SVU_PASSWORD.".to_string()).into())
            }
        }
        (None, Some(_)) => Err(General("SVU_USERNAME enviromnet variable must be set if using SVU_PASSWORD".to_string()).into()),
        _ => {

            //  First attempt to access the repo without credentials
            if access_repo(None,&wc_root)? {
                Ok(None)  // No credentials needed
            }
            else {
                //  Prompt for username and password.
                let mut username: Option<String> = None;
                let mut password: Option<String> = None;

                while username.is_none() && password.is_none() {
                    let u = prompt_for_username()?;
                    if u.is_empty() { continue }
                    let p = prompt_for_password()?;
                    if p.is_empty() { continue }

                    if access_repo(Some(Credentials(u.clone(), p.clone())), &wc_root)? {
                        username = Some(u);
                        password = Some(p);
                    }
                    else {
                        return Err(General("Not a valid username/password.".to_string()).into())
                    }
                }
                Ok(Some(Credentials(username.unwrap(), password.unwrap())))
            }
        }
    }
}

//  Check to see if we can access the repository by
//  running svn info ^/
fn access_repo(credentials: Option<Credentials>, wc_root: &Path) -> Result<bool> {
    let output = svn::SvnCmd::new("info")
        .with_creds(&credentials)
        .with_cwd(Some(wc_root))
        .arg("^/")
        .run()?;

    if output.status.success() {
        Ok(true)
    }
    else {
        let text = String::from_utf8_lossy(&output.stderr);
        if text.contains("Authentication failed") {
            Ok(false)
        } else {
            Err(SvnError(output).into())
        }
    }
}

fn prompt_for_username() -> Result<String> {
    let mut line = String::new();

    print!("Enter username for the subversion repo: ");
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_owned())
}

fn prompt_for_password() -> Result<String> {
    print!("Enter password for the subversion repo: ");
    std::io::stdout().flush()?;
    let line = rpassword::read_password()?;
    Ok(line.trim().to_owned())
}

