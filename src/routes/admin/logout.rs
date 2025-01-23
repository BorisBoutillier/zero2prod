use actix_web::HttpResponse;
use actix_web_flash_messages::{FlashMessage, Level};

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn admin_logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        session.purge();
        FlashMessage::new(
            "You have successfully logged out.".to_string(),
            Level::Error,
        )
        .send();
        Ok(see_other("/login"))
    }
}
