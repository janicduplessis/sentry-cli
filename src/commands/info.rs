//! Implements a command for showing infos from Sentry.
use std::io;
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use serde_json;

use prelude::*;
use api::Api;
use config::{Auth, Config};

#[derive(Serialize, Default)]
pub struct AuthStatus {
    #[serde(rename="type")]
    auth_type: Option<String>,
    successful: bool,
}

#[derive(Serialize, Default)]
pub struct ConfigStatus {
    config: HashMap<String, Option<String>>,
    auth: AuthStatus,
    have_dsn: bool,
}


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("print out information about the sentry server")
        .arg(Arg::with_name("quiet")
             .short("q")
             .long("quiet")
             .help("Do not output anything, just report a status \
                    code for correct config"))
        .arg(Arg::with_name("config_status_json")
             .long("config-status-json")
             .help("Returns the status of the config that sentry-cli loads \
                    as JSON dump. This can be used by external tools to aid \
                    the user towards configuration."))
}

fn describe_auth(auth: Option<&Auth>) -> &str {
    match auth {
        None => "Unauthorized",
        Some(&Auth::Token(_)) => "Auth Token",
        Some(&Auth::Key(_)) => "API Key",
    }
}

fn get_config_status_json(config: &Config) -> Result<()> {
    let mut rv = ConfigStatus::default();

    let (project, org) = config.get_org_and_project_defaults();
    rv.config.insert("org".into(), org);
    rv.config.insert("project".into(), project);
    rv.config.insert("url".into(), Some(config.url.clone()));

    rv.auth.auth_type = config.auth.as_ref().map(|val| match val {
        &Auth::Token(_) => "token".into(),
        &Auth::Key(_) => "api_key".into(),
    });
    rv.auth.successful = config.auth.is_some() && Api::new(config).get_auth_info().is_ok();
    rv.have_dsn = config.get_dsn().is_ok();

    serde_json::to_writer_pretty(&mut io::stdout(), &rv)?;
    println!("");
    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    if matches.is_present("config_status_json") {
        return get_config_status_json(config);
    }

    let (project, org) = config.get_org_and_project_defaults();
    let info_rv = Api::new(config).get_auth_info();
    let errors = project.is_none() || org.is_none() || config.auth.is_none() || info_rv.is_err();

    if !matches.is_present("quiet") {
        println!("Sentry Server: {}", config.url);
        println!("Default Organization: {}", project.unwrap_or("-".into()));
        println!("Default Project: {}", org.unwrap_or("-".into()));

        if config.auth.is_some() {
            println!("");
            println!("Authentication Info:");
            println!("  Method: {}", describe_auth(config.auth.as_ref()));
            match info_rv {
                Ok(info) => {
                    if let Some(ref user) = info.user {
                        println!("  User: {}", user.email);
                    }
                    if let Some(ref auth) = info.auth {
                        println!("  Scopes:");
                        for scope in &auth.scopes {
                            println!("    - {}", scope);
                        }
                    }
                }
                Err(err) => {
                    println!("  (failure on authentication: {})", err);
                }
            }
        }
    }

    if errors {
        Err(ErrorKind::QuietExit(1).into())
    } else {
        Ok(())
    }
}
