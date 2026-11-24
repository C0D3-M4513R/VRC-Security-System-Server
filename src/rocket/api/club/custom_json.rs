use std::borrow::Cow;
use crate::rocket::{Response, State};
use crate::rocket::api::club::{Permissions, CLUB_OWNERS};
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::JWT;
use crate::modals::err::Err;

#[derive(serde_derive::Deserialize)]
pub struct CustomJsonForm {
    custom_json: String,
}

#[actix_web::post("/api/club/{club}/custom_json")]
pub async fn put_custom_json<'r>(
    auth: State<JWT>,
    path: actix_web::web::Path<String>,
    data: actix_web::web::Form<CustomJsonForm>,
) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let club = &*path;

    match Permissions::require_permission(&auth, CLUB_OWNERS, |v| v.manage_permissions.is_some()).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }

    let trimmed = data.custom_json.trim();
    let to_store: Option<serde_json::Value> = if trimmed.is_empty() {
        None
    } else {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) => {
                if !value.is_object() {
                    return Response::Error(
                        Some(actix_web::http::StatusCode::BAD_REQUEST),
                        AskamaWrapper(Err {
                            error: Cow::Borrowed("Custom JSON must be a JSON object"),
                            error_description: None,
                        }),
                    );
                }
                Some(value)
            }
            Err(err) => {
                return Response::Error(
                    Some(actix_web::http::StatusCode::BAD_REQUEST),
                    AskamaWrapper(Err {
                        error: Cow::Borrowed("Invalid JSON"),
                        error_description: Some(err.to_string().into()),
                    }),
                );
            }
        }
    };

    let db = crate::get_db().await;
    match sqlx::query!("UPDATE club SET custom_json = $1 WHERE \"path-name\" = $2", to_store, club)
        .execute(&db)
        .await
    {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("Failed to update custom_json: {err}");
            return Response::Error(
                Some(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
                AskamaWrapper(Err {
                    error: Cow::Borrowed("Failed to update custom JSON in DB"),
                    error_description: Some(err.to_string().into()),
                }),
            );
        }
    }

    Response::Redirect(None, format!("/auth/clubs/{club}/").into())
}
