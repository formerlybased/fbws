use tokio::fs;

#[derive(Clone)]
pub struct View {
    pub web_path: String,
    pub source: String,
}

impl View {
    pub async fn build(
        path: String,
        theme: String,
        web_title: String,
        header_source: String,
    ) -> View {
        let web_path: String = if path.strip_prefix("./pages") == None {
            format!("/{}", path.strip_suffix(".html").unwrap().to_string())
        } else {
            format!(
                "{}",
                path.strip_prefix("./pages")
                    .unwrap()
                    .strip_suffix(".html")
                    .unwrap()
                    .to_string()
            )
        };

        let content = fs::read(path).await;
        let stylesheet = fs::read(theme).await;
        let header = fs::read(header_source).await;
        let source = generate_view(
            String::from_utf8(content.unwrap()).unwrap(),
            String::from_utf8(stylesheet.unwrap()).unwrap(),
            format!(
                "{} on {}",
                web_path.clone().strip_prefix("/").unwrap(),
                web_title
            ),
            String::from_utf8(header.unwrap()).unwrap(),
        ); // TODO:

        View { web_path, source }
    }
}

fn generate_view(src: String, theme: String, title: String, header: String) -> String {
    format!(
        "
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
    "
    )
}

pub async fn make_views(
    dir: String,
    theme: String,
    web_title: String,
    header: String,
) -> std::io::Result<Vec<View>> {
    let mut views: Vec<View> = Vec::new();
    let home_view = View::build(
        String::from("home.html"),
        theme.clone(),
        web_title.clone(),
        header.clone(),
    )
    .await;
    views.push(home_view);

    let error_view = View::build(
        String::from("404.html"),
        theme.clone(),
        web_title.clone(),
        header.clone(),
    )
    .await;
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
