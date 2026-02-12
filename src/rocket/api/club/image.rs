use std::borrow::Cow;
use std::cmp::min;
use crate::rocket::{AskamaWrapper, Response};
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;
#[derive(rocket::FromForm)]
pub struct Upload<'r> {
    file: rocket::fs::TempFile<'r>,
}

#[rocket::post("/api/club/<club>/image/<name>", format="multipart/form-data", data = "<data>")]
pub async fn put_image(auth: Result<JWT, AuthErr>, club: &str, name: &str, data: rocket::form::Form<Upload<'_>>) -> Response<rocket::response::Redirect> {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    let permission:fn(&Permissions)->bool = match name {
        "Logo.png" => |v|v.update_logo,
        "Poster1.png" => |v|v.update_poster1,
        "Poster2.png" => |v|v.update_poster2,
        "Poster3.png" => |v|v.update_poster3,
        _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
            error: Cow::Borrowed("Invalid image name"),
            error_description: None
        }))),
    };
    match Permissions::require_permission(&auth, club, permission).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }

    //The VRChat Limit is 32 MB (do they mean MB or MiB?).
    //We choose a limit less than that, just to be safe.

    if data.file.len() as u128 > rocket::data::ByteUnit::Megabyte(30).as_u128() {
        return Response::Error((rocket::http::Status::PayloadTooLarge, AskamaWrapper(Err{
            error: Cow::Borrowed("Request data was too large? Please upload smaller files."),
            error_description: None,
        })));
    }
    let bytes = {
        use image::GenericImageView;

        fn helper<T: ::std::io::BufRead + ::std::io::Seek>(mut image: image::ImageReader<T>) -> Result<Vec<u8>, (rocket::http::Status, AskamaWrapper<Err<'static>>)> {
            let mut limits = ::image::Limits::default();
            // limits.max_image_width = Some(2048);
            // limits.max_image_height = Some(2048);
            image.limits(limits);
            let image = match image.with_guessed_format() {
                Ok(v) => v,
                Err(err) => return Err((rocket::http::Status::PayloadTooLarge, AskamaWrapper(Err{
                    error: Cow::Borrowed("Io error, whilst guessing image format:"),
                    error_description: Some(Cow::Owned(err.to_string()))
                }))),
            };
            let image = match image.decode() {
                Ok(image) => image,
                Err(image::ImageError::Limits(limit)) => return Err((rocket::http::Status::PayloadTooLarge, AskamaWrapper(Err{
                    error: Cow::Borrowed("Hit limit whilst decoding an image:"),
                    error_description: Some(Cow::Owned(limit.to_string()))
                }))),
                Err(err) => return Err((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                    error: Cow::Borrowed("Image decode error"),
                    error_description: Some(Cow::Owned(err.to_string()))
                }))),
            };

            let image =
                if image.dimensions().0 > 2048 || image.dimensions().1 > 2048 {
                    image.resize(min(2048, image.dimensions().0), min(2048, image.dimensions().1), image::imageops::FilterType::CatmullRom)
                } else {
                    image
                };

            if image.dimensions().0 > 2048 {
                return Err((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                    error: Cow::Borrowed("Image was too wide even after resize (max dimensions 2048x2048)"),
                    error_description: Some(Cow::Owned(format!("Actual: {}", image.dimensions().0))),
                })));
            }
            if image.dimensions().1 > 2048 {
                return Err((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                    error: Cow::Borrowed("Image was too high even after resize (max dimensions 2048x2048)"),
                    error_description: Some(Cow::Owned(format!("Actual: {}", image.dimensions().1))),
                })));
            }

            let mut buf = Vec::new();
            match image.write_with_encoder(image::codecs::png::PngEncoder::new(&mut buf)) {
                Ok(()) => {}
                Err(err) => {
                    return Err((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                        error: Cow::Borrowed("Failed to encode image as png"),
                        error_description: Some(Cow::Owned(err.to_string())),
                    })))
                }
            }

            Ok(buf)
        }
        let data = data.into_inner();
        let image = match match data.file {
            rocket::fs::TempFile::File { path, .. } => {
                let path = path.as_ref().either(|p|p.as_ref(), |p|p.as_path());
                let reader = match tokio::fs::File::open(path).await {
                    Ok(v) => v,
                    Err(err) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                        error: Cow::Borrowed("Io error, whilst opening Temporary-File"),
                        error_description: Some(Cow::Owned(err.to_string()))
                    }))),
                };
                let reader = reader.into_std().await;
                match tokio::task::spawn_blocking(move ||{
                    let reader = reader;
                    let buf_reader = std::io::BufReader::new(&reader);
                    helper(image::ImageReader::new(buf_reader))
                }).await {
                    Ok(v) => v,
                    Err(err) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                        error: Cow::Borrowed("Whilst resizing the Image, the server encountered an internal Exception"),
                        error_description: Some(Cow::Owned(err.to_string())),
                    })))
                }
            },
            rocket::fs::TempFile::Buffered { content } => {
                tokio::task::block_in_place(||helper(image::ImageReader::new(std::io::Cursor::new(content))))
            },
        } {
            Ok(v) => v,
            Err(err) => return Response::Error(err),
        };

        image
    };
    let db = crate::get_db().await;
    let table = match match name {
        "Logo.png" => sqlx::query!("SELECT change_logo($1, $2, $3)", auth.get_user_id().cast_signed(), club, bytes),
        "Poster1.png" => sqlx::query!("SELECT change_poster1($1, $2, $3)", auth.get_user_id().cast_signed(), club, bytes),
        "Poster2.png" => sqlx::query!("SELECT change_poster2($1, $2, $3)", auth.get_user_id().cast_signed(), club, bytes),
        "Poster3.png" => sqlx::query!("SELECT change_poster3($1, $2, $3)", auth.get_user_id().cast_signed(), club, bytes),
        _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err {
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        }))),
    }.execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to update Image: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed up update Image in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("Image update query affected more than 1 row? {affected}");
            return redir
        }
    }
/*
if false {
    let target_branch_name = format!("autopr/club/{club}");
    let commit_message = format!("Club: Update {name} for {club}");
    let repo = repo.inner().clone().lock_owned().await;
    match tokio::task::spawn_blocking(move || {
        crate::git::push::push_file(
            &*repo,
            bytes.as_slice(),
            &target_branch_name,
            &club,
            &name,
            &commit_message,
        )
    }).await {
        Ok(Ok(())) => {},
        Ok(Err(err)) => return Response::Err((rocket::http::Status::InternalServerError, err)),
        Err(err) => return Response::Err((rocket::http::Status::InternalServerError, Cow::Owned(format!("Error Updating Repo: {err}")))),
    }
}
*/
    redir
}

pub const PLACEHOLDER_PNG:&[u8] = include_bytes!("../../../../1x1-00000000.png");
#[rocket::get("/api/club/<club>/image/<name>")]
pub async fn get_image(auth: Result<JWT, AuthErr>, club: &str, name: &str) -> Response<(rocket::http::ContentType, Cow<'static, [u8]>)> {
    let _ = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    let db = crate::get_db().await;
    match match name {
        "Logo.png" => sqlx::query!(r#"SELECT image FROM club_logo INNER JOIN club ON club_logo.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|v.image)),
        "Poster1.png" => sqlx::query!(r#"SELECT image FROM club_poster1 INNER JOIN club ON club_poster1.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|v.image)),
        "Poster2.png" => sqlx::query!(r#"SELECT image FROM club_poster2 INNER JOIN club ON club_poster2.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|v.image)),
        "Poster3.png" => sqlx::query!(r#"SELECT image FROM club_poster3 INNER JOIN club ON club_poster3.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|v.image)),
        _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        }))),
    }{
        Ok(Some(v)) => Response::Ok((rocket::http::ContentType(rocket::http::MediaType::PNG), Cow::Owned(v))),
        Ok(None) => Response::Ok((rocket::http::ContentType(rocket::http::MediaType::PNG), Cow::Borrowed(PLACEHOLDER_PNG))),
        Err(err) => {
            tracing::error!("Failed to get Image: {err}");
            Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed get Image from DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    }
}