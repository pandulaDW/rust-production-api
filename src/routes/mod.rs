pub mod auth;
mod health_check;
mod home;
mod login;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

pub use health_check::health_check;
pub use home::home;
pub use login::{get as login_get, post as login_post};
pub use newsletters::publish_newsletter;
pub use subscriptions::{subscribe, FormData};
pub use subscriptions_confirm::confirm;
