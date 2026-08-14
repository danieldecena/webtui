mod app;
mod browser;
mod extract;
mod render;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::args().nth(1).unwrap_or_else(|| "https://example.com".into());

    let browser = browser::Browser::launch().await?;
    let app = app::App::new(browser, &url).await?;

    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();

    // Chrome is a child process, not a resource the terminal cleans up for us.
    let mut browser = result?;
    browser.shutdown().await?;
    Ok(())
}
