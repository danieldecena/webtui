use crate::extract::{self, Document};
use anyhow::{anyhow, Result};
use chromiumoxide::error::CdpError;
use chromiumoxide::{Browser as Chromium, BrowserConfig, Page};
use futures::StreamExt;
use tokio::task::JoinHandle;

/// Owns the headless Chrome instance and the single page we drive.
pub struct Browser {
    chromium: Chromium,
    handler: Option<JoinHandle<()>>,
    page: Page,
}

impl Browser {
    pub async fn launch() -> Result<Self> {
        let profile = dirs_profile()?;
        let config = BrowserConfig::builder()
            .user_data_dir(profile)
            .build()
            .map_err(|e| anyhow!("browser config: {e}"))?;

        let (chromium, mut events) = Chromium::launch(config).await?;
        let handler = tokio::spawn(async move { while events.next().await.is_some() {} });
        let page = chromium.new_page("about:blank").await?;

        Ok(Self { chromium, handler: Some(handler), page })
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        self.page.goto(url).await?.wait_for_navigation().await?;
        Ok(())
    }

    pub async fn extract(&self) -> Result<Document> {
        let value = self.eval(extract::SNIPPET).await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn eval(&self, js: &str) -> Result<serde_json::Value> {
        match self.page.evaluate(js).await {
            Ok(result) => Ok(result.value().cloned().unwrap_or(serde_json::Value::Null)),
            // The raw CdpError prints the whole protocol struct; the console only
            // wants the line a browser devtools console would have shown.
            Err(CdpError::JavascriptException(details)) => Err(anyhow!(
                "{}",
                details
                    .exception
                    .as_ref()
                    .and_then(|e| e.description.clone())
                    .unwrap_or_else(|| details.text.clone())
            )),
            Err(e) => Err(e.into()),
        }
    }

    /// Elements are addressed by the `data-webtui-id` the snippet stamped on them,
    /// so a handle stays valid for as long as the node survives a re-render.
    pub async fn click(&self, id: u32) -> Result<()> {
        self.eval(&format!(
            "(() => {{ const el = document.querySelector('[data-webtui-id=\"{id}\"]'); \
             if (!el) return false; el.scrollIntoView({{block:'center'}}); el.click(); return true; }})()"
        ))
        .await?;
        self.settle().await;
        Ok(())
    }

    /// The native-setter dance is what makes React and other controlled inputs
    /// notice the change; assigning `.value` alone is silently ignored by them.
    pub async fn type_into(&self, id: u32, text: &str) -> Result<()> {
        let text = serde_json::to_string(text)?;
        self.eval(&format!(
            "(() => {{ const el = document.querySelector('[data-webtui-id=\"{id}\"]'); \
             if (!el) return false; \
             const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement : HTMLInputElement; \
             const setter = Object.getOwnPropertyDescriptor(proto.prototype, 'value').set; \
             el.focus(); setter.call(el, {text}); \
             el.dispatchEvent(new Event('input', {{bubbles:true}})); \
             el.dispatchEvent(new Event('change', {{bubbles:true}})); return true; }})()"
        ))
        .await?;
        Ok(())
    }

    pub async fn submit(&self, id: u32) -> Result<()> {
        self.eval(&format!(
            "(() => {{ const el = document.querySelector('[data-webtui-id=\"{id}\"]'); \
             if (!el) return false; \
             for (const type of ['keydown', 'keypress', 'keyup']) {{ \
               el.dispatchEvent(new KeyboardEvent(type, \
                 {{key:'Enter', code:'Enter', keyCode:13, which:13, bubbles:true}})); }} \
             if (el.form && el.form.requestSubmit) el.form.requestSubmit(); return true; }})()"
        ))
        .await?;
        self.settle().await;
        Ok(())
    }

    /// Clicks may navigate or may only mutate the DOM, and the two are
    /// indistinguishable from here — so give the page a moment either way.
    async fn settle(&self) {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }

    /// Chrome outlives the process unless it is closed explicitly.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.chromium.close().await?;
        self.chromium.wait().await?;
        if let Some(handler) = self.handler.take() {
            handler.abort();
        }
        Ok(())
    }
}

fn dirs_profile() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME is unset"))?;
    let dir = std::path::PathBuf::from(home).join(".local/share/webtui/profile");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
