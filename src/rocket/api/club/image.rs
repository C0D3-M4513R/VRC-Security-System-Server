use std::borrow::Cow;
use std::cmp::min;
use std::sync::LazyLock;
use rocket::Request;
use crate::rocket::{AskamaWrapper, ETag, IfNoneMatch, Response};
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;
#[derive(rocket::FromForm)]
pub struct Upload<'r> {
    file: rocket::fs::TempFile<'r>,
}
pub struct PutImageResponse{
    location: String
}

#[rocket::async_trait]
impl<'r, 'o:'r> rocket::response::Responder<'r, 'o> for PutImageResponse {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        rocket::response::Redirect::to(self.location).respond_to(request)
            .map(|mut v|{
                v.adjoin_raw_header(rocket::http::hyper::header::CACHE_CONTROL.as_str(), "no-cache");
                v
            })
    }
}

#[rocket::post("/api/club/<club>/image/<name>", format="multipart/form-data", data = "<data>")]
pub async fn put_image(auth: Result<JWT, AuthErr>, club: &str, name: &str, data: rocket::form::Form<Upload<'_>>) -> Response<PutImageResponse> {
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
            let limits = ::image::Limits::default();
            #[cfg(feature = "restrict_image_upload_dimensions")]
            let limits = {
                let mut limits = limits;
                limits.max_image_width = Some(2048);
                limits.max_image_height = Some(2048);
                limits
            };
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

    let redir_url = format!("/clubs/{club}");
    let redir = Response::Redirect(rocket::response::Redirect::to(redir_url.clone()));
    let db = crate::get_db().await;
    let table = match match name {
        "Logo.png" => sqlx::query!(r#"SELECT change_logo($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster1.png" => sqlx::query!(r#"SELECT change_poster1($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster2.png" => sqlx::query!(r#"SELECT change_poster2($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster3.png" => sqlx::query!(r#"SELECT change_poster3($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err {
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        }))),
    }{
        Ok(Some(v)) => v,
        Ok(None) => return redir,
        Err(err) => {
            tracing::error!("Failed to update Image: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed up update Image in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    Response::Ok(PutImageResponse{
        location: redir_url,
    })
}

pub const PLACEHOLDER_PNG:&[u8] = include_bytes!("../../../../1x1-00000000.png");
pub const PLACEHOLDER_PNG_SHA3_512:&[u8] = include_bytes!("../../../../1x1-00000000.png.sha3-512");

pub static PLACEHOLDER_PNG_ETAG:LazyLock<ETag<'static>> = LazyLock::new(||ETag {
    status: rocket::http::Status::Ok,
    data: Some(Cow::Borrowed(PLACEHOLDER_PNG)),
    sha3_512: Cow::Borrowed(PLACEHOLDER_PNG_SHA3_512),
    header: rocket::http::HeaderMap::new()
});

#[rocket::get("/api/club/<club>/image/<name>")]
pub async fn get_image(auth: Result<JWT, AuthErr>, etag: IfNoneMatch, club: &str, name: &str) -> Response<(rocket::http::ContentType, ETag<'static>)> {
    let _ = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    let db = crate::get_db().await;
    macro_rules! make_etag {
        ($ident:ident) => {
            ETag{
                status: rocket::http::Status::Ok,
                data: Some(Cow::Owned($ident.image)),
                sha3_512: Cow::Owned($ident.digest),
                header: Default::default(),
            }
        };
        (etag, $ident:ident) => {
            ETag{
                status: rocket::http::Status::Ok,
                data: None,
                sha3_512: Cow::Owned($ident.digest),
                header: Default::default(),
            }
        };
    }
    for i in etag.0 {
        let resp = |i|{
            let mut header = rocket::http::HeaderMap::new();
            header.add_raw(rocket::http::hyper::header::CACHE_CONTROL.as_str(), "no-cache");
            Response::Ok((rocket::http::ContentType(rocket::http::MediaType::PNG), ETag{
                status: rocket::http::Status::Ok,
                data: None,
                sha3_512: Cow::Owned(i),
                header,
            }))
        };
        match match name {
            "Logo.png" => sqlx::query!(r#"SELECT true as dummy FROM club_logo INNER JOIN club ON club_logo.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster1.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster1 INNER JOIN club ON club_poster1.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster2.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster2 INNER JOIN club ON club_poster2.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster3.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster3 INNER JOIN club ON club_poster3.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Invalid image name"),
                error_description: None,
            }))),
        }{
            Ok(Some(())) => return resp(i),
            Ok(None) => {},
            Err(err) => {
                tracing::error!("Failed to get Image: {err}");
                return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                    error: Cow::Borrowed("Failed get Image from DB"),
                    error_description: Some(err.to_string().into()),
                })));
            }
        }
        if i == PLACEHOLDER_PNG_SHA3_512 {
            return resp(i);
        }
    }
    let mut etag:ETag = match match name {
        "Logo.png" => sqlx::query!(r#"SELECT image, digest FROM club_logo INNER JOIN club ON club_logo.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster1.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster1 INNER JOIN club ON club_poster1.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster2.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster2 INNER JOIN club ON club_poster2.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster3.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster3 INNER JOIN club ON club_poster3.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        _ => return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        }))),
    }{
        Ok(Some(v)) => v,
        Ok(None) => PLACEHOLDER_PNG_ETAG.clone(),
        Err(err) => {
            tracing::error!("Failed to get Image: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed get Image from DB"),
                error_description: Some(err.to_string().into()),
            })));
        }
    };

    etag.header.add_raw(rocket::http::hyper::header::CACHE_CONTROL.as_str(), "no-cache");
    Response::Ok((rocket::http::ContentType(rocket::http::MediaType::PNG), etag))
}