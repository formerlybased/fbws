use config::{Config, File};

use clap::{Parser, Subcommand};

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};

mod view;
use crate::view::View;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Static page generator & web server written in rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    New { name: Option<String> },
    Run,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run) => {
            run_server();
        }
        Some(Commands::New { name }) => {
            create_project(name.to_owned());
        }
        None => {
            println!("Usage: fbws [command]\nUse --help for help");
        }
    }
}

#[tokio::main]
async fn run_server() {
    let conf_result = Config::builder()
        .add_source(File::with_name("project.toml"))
        .build();

    let conf = match conf_result {
        Ok(config) => config,
        Err(config_error) => panic!("Not a valid FBWS project!\n Error: {}", config_error),
    };

    let views: Vec<View> = view::make_views(
        String::from("./pages"),
        conf.get::<String>("theme")
            .expect("Theme configuration unset"),
        conf.get::<String>("title")
            .expect("Project needs a valid title"),
        conf.get::<String>("header")
            .expect("Header path required (even if unused)"),
    )
    .await
    .unwrap();

    let address = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        conf.get::<u16>("port").expect("Port configuration unset"),
    );

    let service = make_service_fn(move |_| {
        let views = views.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |_req| {
                let views = views.clone();
                async move { respond(_req, views).await }
            }))
        }
    });

    let server = Server::bind(&address).serve(service);
    let server = server.with_graceful_shutdown(shutdown_signal_await());

    println!("Serving on http://{}", address);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    println!("\nServer shutdown...");
}

async fn respond(req: Request<Body>, views: Vec<View>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => send_view(&views[0]).await,
        (&Method::GET, path) => Ok(route(&views, String::from(path)).await),
        _ => Ok(handle_404(&views[1])),
    }
}

async fn route(views: &Vec<View>, path: String) -> Response<Body> {
    for v in views {
        if v.web_path == path {
            return send_view(&v).await.unwrap();
        };
    }

    return handle_404(&views[1]);
}

fn handle_404(view: &View) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(view.clone().source.into())
        .unwrap()
}

async fn send_view(v: &View) -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .body(v.clone().source.into())
        .unwrap())
}

fn create_project(dir: Option<String>) {
    if dir == None {
        println!("Usage: fbws new <project-name>");
        return;
    }
    let dir: String = dir.unwrap();
    std::fs::create_dir(dir.clone()).unwrap();
    std::fs::write(dir.clone() + "/home.html", "<h1>Home page!</h1>").unwrap();
    std::fs::write(dir.clone() + "/404.html", "<h1>404 Page!</h1>").unwrap();
    std::fs::write(dir.clone() + "/theme.css", "/* Add your style here */").unwrap();
    std::fs::write(
        dir.clone() + "/header.html",
        "<!--Use this file to add a header across all pages-->",
    )
    .unwrap();
    std::fs::write(
        dir.clone() + "/project.toml",
        format!(
            "title = \"{}\"\ntheme = \"theme.css\"\nport = 8080\nheader = \"header.html\"",
            dir.clone()
        ),
    )
    .unwrap();
    std::fs::create_dir(dir.clone() + "/pages/").unwrap();

    println!("Project created at {}/", dir);
}

async fn shutdown_signal_await() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to attach ctrl-c handler");
}
