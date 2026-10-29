//! What blackjack does once it is up. Written once by `angzarr scaffold
//! startup` and yours: send commands with `app.handle(book, &sampled)`, read
//! a stream with `app.events(&cover)`. The application is used, and dropped,
//! outside an async context.

use angzarr_embedded_host::App;

pub fn drive(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let _ = app;
    todo!("send the application's commands")
}
