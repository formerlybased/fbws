use config::{Config, File };

use clap::{Parser, Subcommand};

use tokio::fs;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    New { name: Option<String> },
    Run,
}

#[derive(Clone)]
struct View {
    web_path: String,
    source: String,
}

impl View {
    async fn build(path: String, theme: String, web_title: String, header_source: String) -> View {
        let web_path: String;

        if path.strip_prefix("./pages") == None {
            web_path = format!("/{}", path.strip_suffix(".html").unwrap().to_string());
        } else {
            web_path = format!("{}", path.strip_prefix("./pages").unwrap().strip_suffix(".html").unwrap().to_string());
        }

        let content = fs::read(path).await;
        let stylesheet = fs::read(theme).await;
        let header = fs::read(header_source).await;
        let source = generate_view(String::from_utf8(content.unwrap()).unwrap(), String::from_utf8(stylesheet.unwrap()).unwrap(), format!("{} on {}", web_path.clone().strip_prefix("/").unwrap(), web_title), String::from_utf8(header.unwrap()).unwrap()); // TODO:

        View {
            web_path,
            source,
        }
    }

}

fn generate_view(src: String, theme: String, title: String, header: String) -> String {
    return format!("
    <style>\n
    {theme}\n
    </style>\n
    <html>\n
    <head>\n
    <title>{title}</title>\n
    </head>\n
    <body>\n
    <header>\n
    {header}\n
    </header>\n
    {src}\n
    </body>\n
    </html>
    ");
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
        Err(config_error) => panic!("Not a valid FBWS project!\n Error: {}", config_error )
    };

    let views: Vec<View> = make_views(String::from("./pages"), conf.get::<String>("theme").expect("Theme configuration unset"), conf.get::<String>("title").expect("Project needs a valid title"), conf.get::<String>("header").expect("Header path required (even if unused)")).await.unwrap();

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), conf.get::<u16>("port").expect("Port configuration unset"));

    let service = make_service_fn(move |_| { 
        let views = views.clone();
        async move {Ok::<_, hyper::Error>(service_fn(move |_req| {
            let views = views.clone();
            async move { respond(_req, views).await }
        }))}
    });

    let server = Server::bind(&address).serve(service);
    let server = server.with_graceful_shutdown(shutdown_signal_await());

    println!("Serving on http://{}", address);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    println!("\nServer shutdown...");
}

async fn make_views(dir: String, theme: String, web_title: String, header: String) -> std::io::Result<Vec<View>> {
    let mut views: Vec<View> = Vec::new();
    let home_view = View::build(String::from("home.html"), theme.clone(), web_title.clone(), header.clone()).await;
    views.push(home_view);
    let error_view = View::build(String::from("404.html"), theme.clone(), web_title.clone(), header.clone()).await;
    views.push(error_view);

    for f in std::fs::read_dir(dir.clone())? {
        let file_path = f?.path();
        if file_path.is_dir() {
            break;
        }
        let file_path = file_path.into_os_string().into_string().unwrap();
        views.push(View::build(file_path, theme.clone(), web_title.clone(), header.clone()).await);
    }

    Ok(views)
}

async fn respond(req: Request<Body>, views: Vec<View> ) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => send_view(&views[0]).await,
        (&Method::GET, path) => Ok(route(&views, String::from(path)).await),
        _ => Ok(handle_404(&views[1])),
    }
}

async fn route(views: &Vec<View>, path: String) -> Response<Body> {
    for v in views {
        if v.web_path == path { return send_view(&v).await.unwrap() };
    }

    return handle_404(&views[1])
}

fn handle_404(view: &View) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(view.clone().source.into())
        .unwrap()
}

async fn send_view(v: &View) -> Result<Response<Body>> {
    return Ok(Response::builder().status(StatusCode::FOUND).body(v.clone().source.into()).unwrap());
}

fn create_project(dir: Option<String>) {
    if dir == None {
        println!("Usage: fbws new <project-name>");
        return
    }
    let dir: String = dir.unwrap();
    std::fs::create_dir(dir.clone()).unwrap();
    std::fs::write(dir.clone() + "/home.html", "<h1>Home page!</h1>").unwrap();
    std::fs::write(dir.clone() + "/404.html", "<h1>404 Page!</h1>").unwrap();
    std::fs::write(dir.clone() + "/theme.css", "/* Add your style here */").unwrap();
    std::fs::write(dir.clone() + "/header.html", "<!--Use this file to add a header across all pages-->").unwrap();
    std::fs::write(dir.clone() + "/project.toml", format!("title = \"{}\"\ntheme = \"theme.css\"\nport = 8080\nheader = \"header.html\"", dir.clone())).unwrap();
    std::fs::create_dir(dir.clone() + "/pages/").unwrap();

    println!("Project created at {}/", dir);
}

async fn shutdown_signal_await() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to attach ctrl-c handler");
}
