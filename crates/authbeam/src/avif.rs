use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header::CONTENT_TYPE, StatusCode},
};
use axum_extra::extract::Multipart;
use std::{fs::File, io::BufWriter};

/// An image extractor accepting:
/// * `multipart/form-data`
/// * `image/png`
/// * `image/jpeg`
/// * `image/avif`
/// * `image/webp`
pub struct Image(pub Bytes);

#[async_trait]
impl<S> FromRequest<S> for Image
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Some(content_type) = req.headers().get(CONTENT_TYPE) else {
            return Err(StatusCode::BAD_REQUEST);
        };

        let body = if content_type
            .to_str()
            .unwrap()
            .starts_with("multipart/form-data")
        {
            let mut multipart = Multipart::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            let Ok(Some(field)) = multipart.next_field().await else {
                return Err(StatusCode::BAD_REQUEST);
            };

            field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?
        } else if (content_type == "image/avif")
            | (content_type == "image/jpeg")
            | (content_type == "image/png")
            | (content_type == "image/webp")
        {
            Bytes::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?
        } else {
            return Err(StatusCode::BAD_REQUEST);
        };

        Ok(Self(body))
    }
}

/// Create an AVIF buffer given an input of `bytes`
pub fn save_avif_buffer(path: &str, bytes: Vec<u8>) -> std::io::Result<()> {
    let pre_img_buffer = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Image failed",
            ));
        }
    };

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    if let Err(_) = pre_img_buffer.write_to(&mut writer, image::ImageFormat::Avif) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Image conversion failed",
        ));
    };

    Ok(())
}
