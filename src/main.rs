#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate colored;
extern crate chrono;
extern crate handlebars;

use clap::{Arg, App, SubCommand, AppSettings};
use std::{path, io, fs, str, fmt, error::Error};
use chrono::prelude::*;
use colored::*;
use handlebars::Handlebars;
/*

    basic setup,
    1. cairo init
        a: make 2 folders, posts & html
    2. cairo make

*/

enum CairoError {
    IOError(io::Error),
    ParseBoolError(str::ParseBoolError),
    PostParseError(String),
    CantFindIndexTemplate(),
    CantFindPostTemplate()
}

type Result<T> = std::result::Result<T, CairoError>;

impl CairoError {

    fn post_parse_error<S>(msg: S) -> CairoError where S: Into<String> {
        CairoError::PostParseError(msg.into())
    }

    fn format(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CairoError::IOError(err) => write!(f, "io — {}", err.description())?,
            CairoError::ParseBoolError(err) => write!(f, "cli parse error — {:?}", err)?,
            CairoError::PostParseError(err) => write!(f, "post parse error — {}", err)?,
            CairoError::CantFindIndexTemplate() => write!(f, "could not find index template")?,
            CairoError::CantFindPostTemplate() => write!(f, "could not find post template")?,
        }
        Ok(())
    }
}

impl fmt::Debug for CairoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

impl From<str::ParseBoolError> for CairoError {
    fn from(e: str::ParseBoolError) -> CairoError {
        CairoError::ParseBoolError(e)
    }
}

impl From<io::Error> for CairoError {
    fn from(e: io::Error) -> CairoError {
        CairoError::IOError(e)
    }
}

impl From<chrono::ParseError> for CairoError {
    fn from(e: chrono::ParseError) -> CairoError {
        CairoError::post_parse_error(format!("time parse: {}", e.description()))
    }
}

#[derive(Debug, Serialize, Clone)]
struct Post {
    path: path::PathBuf,
    filename: String,
    source: String,
    title: String,
    date: NaiveDateTime,
    tags: Vec<String>,
}

impl Post {
    fn from(e: fs::DirEntry) -> Result<Post> {
        let contents = fs::read_to_string(e.path())?;
        let mut data = contents.split("\n---\n");
        let meta: Vec<String> = data.next().ok_or_else(|| CairoError::post_parse_error("Failed to parse post meta-info"))?
            .split('\n').map(String::from).collect();
        let body = data.next().ok_or_else(|| CairoError::post_parse_error("Failed to parse post body"))?;

        let title = meta.get(0).ok_or_else(|| CairoError::post_parse_error("Failed to parse title string"))?.clone();
        let date_str = meta.get(1).ok_or_else(|| CairoError::post_parse_error("Failed to parse date string"))?;
        let date = NaiveDateTime::parse_from_str(date_str, "%a %b %e %T %Y")?;
        
        Ok(Post{
            path: e.path(),
            filename: e.path().file_name().unwrap_or_else(|| OsStr::new("missing-filename")),
            source: body.to_string(),
            title,
            date,
            tags: meta.get(2).ok_or_else(|| CairoError::post_parse_error("Failed to parse tags"))?.split(' ').map(String::from).collect(),
        })
    }
}

fn make_project<S>(build: S, source: S) -> Result<()>
where S: Into<String> {
    let source_dir = source.into();
    let build_dir = build.into();
    if !path::Path::new(&source_dir).join("templates").join("index.hbs").exists() {
        return Err(CairoError::CantFindIndexTemplate())
    }
    if !path::Path::new(&source_dir).join("templates").join("post.hbs").exists() {
        return Err(CairoError::CantFindPostTemplate())
    }
    let posts_folder = path::Path::new(&source_dir).join("posts");
    let posts: Vec<Result<Post>> = fs::read_dir(posts_folder)?
        .filter_map(std::result::Result::ok)
        .map(Post::from)
        .collect();
    for p in posts {
        if p.is_err() {
            return Err(p.err().unwrap());
        }
    }

    let mut reg = Handlebars::new();
    let rendered_index = reg.render_template_source_to_write(template_source: &mut Read, data: &T, writer: W)

    Ok(())
}


fn create_project<S>(st: S, name: S, tagging: bool) -> std::result::Result<(), io::Error>
where S: Into<String> {
    let string = st.into();
    let path = path::Path::new(&string);
    let folder = path.join(name.into());
    fs::create_dir(&folder)?;
    if tagging {
        fs::File::create(folder.join("tags"))?;
    }
    fs::create_dir(folder.join("posts"))?;
    let templates = folder.join("templates");
    fs::create_dir(&templates)?;
    fs::File::create(templates.join("index.hbs"))?;
    fs::File::create(templates.join("post.hbs"))?;
    Ok(())
}

const AUTHOR: &str = "Haze <haze@tachyon.software>";

fn build_app<'a, 'b>() -> App<'a, 'b> { 
    App::new("cairo")
        .setting(AppSettings::ColorAuto)
        .version(crate_version!())
        .author(AUTHOR)
        .about("Minimalistic static blog site generator")
        .subcommand(SubCommand::with_name("init")
                    .about("Subcommand used to generate a basic cairo project")
                    .version("A")
                    .author(AUTHOR)
                    .arg(Arg::with_name("tags")
                        .short("t")
                        .help("Create the tag index")
                        .takes_value(false))
                    .arg(Arg::with_name("path")
                        .short("p")
                        .help("Specify a folder to create base structure in")
                        .hide_default_value(true)
                        .default_value("."))
                    .arg(Arg::with_name("name")
                        .help("The canonical name of the site")
                        .last(true)))
        .subcommand(SubCommand::with_name("make")
                    .about("Subcommand used to compile posts into a static website")
                    .version("A")
                    .author(AUTHOR)
                    .arg(Arg::with_name("build-dir")
                        .short("b")
                        .help("Specify build output folder")
                        .default_value("./build"))
                    .arg(Arg::with_name("source")
                        .help("Specify source input folder")
                        .default_value(".")
                        .required(true).last(true)))
}

fn main() -> Result<()> {
    let matches = build_app().get_matches();
    if let Some(init_matches) = matches.subcommand_matches("init") {
        let path = init_matches.value_of("path").unwrap();
        if let Some(name) = init_matches.value_of("name") {
            let tags = init_matches.value_of("tags").unwrap_or("false");
            create_project(path, name, tags.parse::<bool>()?)?;
            println!("{} created project {}", "SUCCESS!".green(), name);
        }
        return Ok(())
    } else if let Some(make_matches) = matches.subcommand_matches("make") {
        let build_path = make_matches.value_of("build-dir").unwrap();
        let source_path = make_matches.value_of("source").unwrap();
        make_project(build_path, source_path)?;
        return Ok(())
    }
    println!("subcommand not found");
    build_app().print_long_help().expect("Failed to print help");
    Ok(())
}
