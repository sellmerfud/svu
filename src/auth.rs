
use anyhow::Result;
use std::env;
use std::io::Write;
use crate::util::SvError::*;
use std::path::Path;
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
    let _       = svn::workingcopy_info()?;  // Ensure we are in working copy directory
    let cwd     = env::current_dir()?;
    let wc_root = svn::workingcopy_root(&cwd).unwrap();

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

fn access_repo(credentials: Option<Credentials>, wc_root: &Path) -> Result<bool> {
    let mut args = Vec::new();
    args.push("info".to_string());
    push_creds(&mut args, &credentials);
    args.push("^/".to_string());

    let output = svn::run_svn(&args, Some(wc_root))?;
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

pub fn push_creds(args: &mut Vec<String>, creds: &Option<Credentials>) -> () {
    if let Some(Credentials(username, password)) = creds {
        args.push(format!("--username={}", username));
        args.push(format!("--password={}", password));
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

